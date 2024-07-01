using System.Numerics;
using System.Runtime.InteropServices;

namespace RedisClone;

internal sealed unsafe class ArenaAllocator : IDisposable
{
    // Adapted from https://www.gingerbill.org/article/2019/02/08/memory-allocation-strategies-002/

    private const int MinSize = 1024;
    private const int MaxSize = 1024 * 1024 * 128;

    private byte* _buffer;
    private nuint _capacity;
    private nuint _currentOffset;
    private readonly bool _isGrowable;

    public int Capacity => (int)_capacity;

    private ArenaAllocator(byte* buffer, nuint capacity, bool isGrowable)
    {
        _buffer = buffer;
        _capacity = capacity;
        _currentOffset = 0;
        _isGrowable = isGrowable;
    }

    public UnmanagedMemoryManager<T> Allocate<T>(int length)
        where T : unmanaged
    {
        var ptr = AllocatePtr<T>(length);
        return new UnmanagedMemoryManager<T>(ptr, length);
    }

    public unsafe T* AllocatePtr<T>(int length)
        where T : unmanaged
    {
        Assert(_buffer is not null, "buffer isn't allocated");
        Assert(length is > 0, "Number of elements allocated must be greater than 0");

        var currPtr = (nuint)_buffer + _currentOffset;
        var alignment = Structures.Alignment<T>();
        var offset = AlignForward(currPtr, alignment);
        offset -= (nuint)_buffer;

        var size = (nuint)sizeof(T) * (nuint)length;
        var allocationFits = offset + size <= _capacity;
        Assert(_isGrowable || allocationFits, "Allocation fits arena, or arena is growable");
        if (!allocationFits)
            Grow();

        var ptr = _buffer + offset;
        _currentOffset = offset + size;
        return (T*)ptr;
    }

    private void Grow()
    {
        var newCapacity = _capacity * 2;
        Assert(newCapacity <= MaxSize, "Must not exceed max capacity");
        Assert(_buffer is not null, "Buffer must be allocated");
        Assert(_isGrowable, "Arena must be growable");

        var buffer = (byte*)NativeMemory.AlignedRealloc(_buffer, newCapacity, 4096);
        _buffer = buffer;
        _capacity = newCapacity;
        Assert(buffer is not null, "Allocation should always succeed");
    }

    public void Reset()
    {
        Assert(_buffer is not null, "buffer isn't allocated");

        _currentOffset = 0;
    }

    public static ArenaAllocator Allocate(int capacity = 1024 * 1024, bool isGrowable = false)
    {
        Assert(capacity is >= MinSize and <= MaxSize, "Capacity must be between 1KB and 128MB");
        Assert(BitOperations.IsPow2(capacity), "Capacity must be a power of 2");
        var buffer = NativeMemory.AlignedAlloc((nuint)capacity, 4096);
        Assert(buffer is not null, "Allocation should always succeed");
        return new ArenaAllocator((byte*)buffer, (nuint)capacity, isGrowable);
    }

    private static nuint AlignForward(nuint ptr, int align)
    {
        nuint p,
            a,
            modulo;

        Assert(BitOperations.IsPow2(align), "Can only align power of 2 numbers");
        Assert(
            align is > 0 and <= 256 && BitOperations.IsPow2(align),
            "Alignment must be between 1 and 256 and be a power of 2"
        );

        p = ptr;
        a = (nuint)align;
        // Same as (p % a) but faster as 'a' is a power of two
        modulo = p & (a - 1);

        if (modulo != 0)
        {
            // If 'p' address is not aligned, push the address to the
            // next value which is aligned
            p += a - modulo;
        }
        return p;
    }

    public void Dispose()
    {
        Assert(_buffer is not null, "buffer isn't allocated");

        NativeMemory.AlignedFree(_buffer);
        _buffer = null;
    }
}
