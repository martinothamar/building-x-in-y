using System.Numerics;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;

namespace RedisClone;

internal enum StorageMutationResult : byte
{
    Add,
    Update,
}

internal sealed unsafe class Storage : IDisposable
{
    private readonly CancellationToken _stopping;
    private readonly int _shift;
    private readonly int _shards;
    private readonly Table* _tables;

    internal Storage(CancellationToken stopping)
    {
        _stopping = stopping;

        _shards = (int)BitOperations.RoundUpToPowerOf2((uint)(Environment.ProcessorCount * 4));
        Assert(_shards > 1 && BitOperations.IsPow2(_shards), "Shards should be pow2");
        var shift = 32 - BitOperations.TrailingZeroCount(_shards);
        _shift = shift;
        const int initialCapacity = 64;

        const int alignTo = 256;
        Assert(sizeof(Table) is alignTo, "Table struct should be cache line aligned/occupy a whole line");
        _tables = (Table*)NativeMemory.AlignedAlloc((nuint)(sizeof(Table) * _shards), alignTo);
        for (int i = 0; i < _shards; i++)
        {
            _tables[i] = default;
            _tables[i].EnsureCapacity(initialCapacity);
        }
    }

    public StorageMutationResult Set(ref ByteString key, ref ByteString value)
    {
        uint hashCode = (uint)key.GetHashCode();
        var shard = hashCode >> _shift;
        Assert(shard < _shards, "Should resolve to valid shart");
        ref var table = ref Unsafe.AsRef<Table>(_tables + shard);
        bool lockTaken = false;
        table.Lock.Enter(ref lockTaken);
        Assert(lockTaken, "Expect lock to succeed");
        var result = table.Set(ref key, ref value, hashCode);
        table.Lock.Exit(useMemoryBarrier: true);
        return result;
    }

    public bool TryGetValue(ref ByteString key, out ByteString value)
    {
        uint hashCode = (uint)key.GetHashCode();
        var shard = hashCode >> _shift;
        Assert(shard < _shards, "Should resolve to valid shart");
        ref var table = ref Unsafe.AsRef<Table>(_tables + shard);
        bool lockTaken = false;
        table.Lock.Enter(ref lockTaken);
        Assert(lockTaken, "Expect lock to succeed");
        var result = table.TryGetValue(ref key, out value, hashCode);
        table.Lock.Exit(useMemoryBarrier: true);
        return result;
    }

    public void Dispose()
    {
        Assert(_stopping.IsCancellationRequested, "Should only dispose storage when server is shutting down");
        for (int i = 0; i < _shards; i++)
        {
            _tables[i].Dispose();
        }
        NativeMemory.AlignedFree(_tables);
        // foreach (var kvp in _data)
        // {
        //     kvp.Key.Dispose();
        //     kvp.Value.Value.Dispose();
        // }
        // _data.Clear();
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct Table : IDisposable
    {
        private const int StartOfFreeList = -3;

        public int* Buckets; // 8 bytes
        public Entry* Entries; // 8 bytes
        public SpinLock Lock; // 4 bytes
        public int Capacity; // 4 bytes
        private int _count; // 4 bytes
        private int _freeList; // 4 bytes
        private int _freeCount; // 4 bytes
        private fixed byte _padding[220]; // Pad the rest, ending up on 256 bytes

        public readonly int Count => _count - _freeCount;

        public void EnsureCapacity(int capacity)
        {
            Assert(capacity > 1 && BitOperations.IsPow2(capacity), "Capacity should be pow2");

            if (Buckets is not null)
            {
                Assert(capacity > Capacity, "Capacity should only increase");
                Buckets = (int*)
                    NativeMemory.AlignedRealloc(
                        Buckets,
                        (nuint)(sizeof(int) * capacity),
                        (nuint)Structures.Alignment<int>()
                    );
                Entries = (Entry*)
                    NativeMemory.AlignedRealloc(
                        Entries,
                        (nuint)(sizeof(Entry) * capacity),
                        (nuint)Structures.Alignment<Entry>()
                    );
            }
            else
            {
                _count = 0;
                _freeList = -1;
                _freeCount = 0;
                Buckets = (int*)
                    NativeMemory.AlignedAlloc((nuint)(sizeof(int) * capacity), (nuint)Structures.Alignment<int>());
                Entries = (Entry*)
                    NativeMemory.AlignedAlloc((nuint)(sizeof(Entry) * capacity), (nuint)Structures.Alignment<Entry>());
                Lock = new SpinLock(enableThreadOwnerTracking: false);
            }
            Assert(Buckets is not null && Entries is not null, "Table should be initialized");
            Capacity = capacity;
        }

        public bool TryGetValue(ref ByteString key, out ByteString value, uint hashCode)
        {
            Assert(Buckets is not null && Entries is not null, "Table should be initialized");
            ref var entry = ref FindEntry(ref key, hashCode);
            if (!Unsafe.IsNullRef(ref entry))
            {
                value = entry.Value;
                return true;
            }

            value = default;
            return false;
        }

        public StorageMutationResult Set(ref ByteString key, ref ByteString value, uint hashCode)
        {
            Assert(Buckets is not null && Entries is not null, "Table should be initialized");
            Assert(key.IsSet, "Should be a valid key");

            ref int bucket = ref GetBucket(hashCode);
            int i = bucket - 1;
            while ((uint)i < (uint)Capacity)
            {
                ref var entry = ref Unsafe.AsRef<Entry>(Entries + i);

                if (entry.HashCode == hashCode && key.RefEquals(ref entry.Key))
                {
                    if (value.Length <= entry.Value.Length)
                    {
                        entry.Value.CopyFrom(ref value);
                    }
                    else
                    {
                        entry.Value.Dispose();
                        entry.Value = value;
                        entry.Value.Copy();
                    }
                    return StorageMutationResult.Update;
                }

                i = entry.Next;
            }

            int index;
            if (_freeCount > 0)
            {
                index = _freeList;
                var next = Entries[_freeList].Next;
                Assert((StartOfFreeList - next) >= -1, "shouldn't overflow because `next` cannot underflow");
                _freeList = StartOfFreeList - next;
                _freeCount--;
            }
            else
            {
                int count = _count;
                if (count == Capacity)
                {
                    EnsureCapacity(Capacity * 2);
                    bucket = ref GetBucket(hashCode);
                }

                index = count;
                _count = count + 1;
            }

            {
                ref var entry = ref Unsafe.AsRef<Entry>(Entries + index);
                entry.HashCode = hashCode;
                entry.Next = bucket - 1; // Value in _buckets is 1-based
                entry.Key = key;
                entry.Value = value;
                entry.Key.Copy();
                entry.Value.Copy();
                bucket = index + 1;
                return StorageMutationResult.Add;
            }
        }

        private ref Entry FindEntry(ref ByteString key, uint hashCode)
        {
            Assert(key.IsSet, "Should be a valid key");

            int i = GetBucket(hashCode);

            i--;
            for (int j = 0; j < Capacity; j++)
            {
                if ((uint)i >= (uint)Capacity)
                {
                    return ref Unsafe.NullRef<Entry>();
                }

                ref var entry = ref Unsafe.AsRef<Entry>(Entries + i);
                if (entry.HashCode == hashCode && key.RefEquals(ref entry.Key))
                {
                    return ref entry;
                }

                i = entry.Next;
            }

            Assert(false, "Couldn't find entry");
            return ref Unsafe.NullRef<Entry>();
        }

        [MethodImpl(MethodImplOptions.AggressiveInlining)]
        private ref int GetBucket(uint hashCode)
        {
            var index = hashCode & (Capacity - 1);
            return ref Unsafe.AsRef<int>(Buckets + index);
        }

        public void Dispose()
        {
            Assert(Buckets is not null && Entries is not null, "Table should be initialized");
            NativeMemory.AlignedFree(Buckets);
            NativeMemory.AlignedFree(Entries);
            Buckets = null;
            Entries = null;
            Capacity = 0;
            Lock = default;
            _count = 0;
            _freeList = -1;
            _freeCount = 0;
        }
    }

    private struct Entry
    {
        public uint HashCode;
        public int Next;
        public ByteString Key;
        public ByteString Value;
    }
}
