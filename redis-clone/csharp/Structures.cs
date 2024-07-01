using System.Numerics;

namespace RedisClone;

internal static class Structures
{
    private struct AlignmentStruct<T>
        where T : unmanaged
    {
#pragma warning disable CS0169 // Field is never used
        T value;
        byte b;
#pragma warning restore CS0169
    }

    internal static unsafe int Alignment<T>()
        where T : unmanaged
    {
        var alignment = sizeof(AlignmentStruct<T>) - sizeof(T);
        Assert(
            alignment is > 0 and <= 256 && BitOperations.IsPow2(alignment),
            "Alignment must be between 1 and 256 and be a power of 2"
        );
        return alignment;
    }
}
