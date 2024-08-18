using System.Diagnostics.CodeAnalysis;

namespace Sync.Core;

public sealed class AssertionException : Exception
{
    public AssertionException(string message)
        : base(message) { }

    [DoesNotReturn]
    internal static void Throw(string message) => throw new AssertionException(message);
}

partial class Prelude
{
    public static void Assert([DoesNotReturnIf(false)] bool condition, string assertion)
    {
        if (!condition)
            AssertionException.Throw(assertion);
    }
}
