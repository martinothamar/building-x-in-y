using System.Diagnostics.CodeAnalysis;

internal static class Assertion
{
    internal static void Assert([DoesNotReturnIf(false)] bool success, string message)
    {
        if (!success)
            throw new Exception($"Assertion failed: {message}");
    }
}
