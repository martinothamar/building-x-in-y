using System.Diagnostics.CodeAnalysis;
using System.Runtime.CompilerServices;

namespace RedisClone;

internal static class Assertion
{
    internal static void Assert([DoesNotReturnIf(false)] bool success, string message)
    {
        if (!success)
            AssertionException.Throw($"Assertion failed: {message}");
    }
}

internal sealed class AssertionException : Exception
{
    public AssertionException(string message)
        : base(message) { }

    [DoesNotReturn]
    [MethodImpl(MethodImplOptions.NoInlining)]
    internal static void Throw(string message) => throw new AssertionException(message);
}
