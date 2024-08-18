using System.IO.Hashing;
using System.Runtime.CompilerServices;

namespace Sync.Core;

public static class Checksummer
{
    // Should always be 16: https://github.com/dotnet/runtime/blob/b764692d18e66561c7cff2101d642b0f789dae0c/src/libraries/System.Private.CoreLib/src/System/UInt128.cs#L25
    public static readonly int ChecksumSize = 16;

    public static UInt128 Compute(ReadOnlySpan<byte> memory) => XxHash128.HashToUInt128(memory, 1337);

    public static UInt128 Compute(ref byte memory, int length)
    {
        unsafe
        {
            var ptr = Unsafe.AsPointer(ref memory);
            return Compute(new ReadOnlySpan<byte>(ptr, length));
        }
    }
}
