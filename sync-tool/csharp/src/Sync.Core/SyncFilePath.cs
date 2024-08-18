using System.Diagnostics.CodeAnalysis;
using System.Numerics;
using System.Runtime.InteropServices;
using static Sync.Core.Prelude;

namespace Sync.Core;

public readonly struct SyncFilePath : IEquatable<SyncFilePath>
{
    private readonly string _value;

    public SyncFilePath(string path)
    {
        Assert(Path.IsPathRooted(path), "This is an absolute path");
        _value = path;
    }

    public bool Equals(SyncFilePath other) => _value.SequenceEqual(other._value);

    public override bool Equals([NotNullWhen(true)] object? obj) => obj is SyncFilePath other && Equals(other);

    public override string ToString() => _value;

    public override int GetHashCode() => _value.GetHashCode();

    private const uint Hash1Start = (5381 << 16) + 5381;
    private const uint Factor = 1_566_083_941;

    public uint GetUnsignedHashCode()
    {
        int length = _value.Length;
        unchecked
        {
            unsafe
            {
                // Taken from BCL somewhere
                fixed (char* src = &MemoryMarshal.GetReference(_value.AsSpan()))
                {
                    uint hash1,
                        hash2;
                    hash1 = Hash1Start;
                    hash2 = hash1;

                    uint* ptrUInt32 = (uint*)src;
                    while (length >= 4)
                    {
                        hash1 = (BitOperations.RotateLeft(hash1, 5) + hash1) ^ ptrUInt32[0];
                        hash2 = (BitOperations.RotateLeft(hash2, 5) + hash2) ^ ptrUInt32[1];
                        ptrUInt32 += 2;
                        length -= 4;
                    }

                    char* ptrChar = (char*)ptrUInt32;
                    while (length-- > 0)
                    {
                        hash2 = (BitOperations.RotateLeft(hash2, 5) + hash2) ^ *ptrChar++;
                    }

                    return hash1 + (hash2 * Factor);
                }
            }
        }
    }
}
