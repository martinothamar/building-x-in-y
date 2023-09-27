using System.Diagnostics.CodeAnalysis;
using System.Runtime.CompilerServices;

namespace CalculationEngine;

internal static class ThrowHelper
{
    [MethodImpl(MethodImplOptions.NoInlining), DoesNotReturn]
    internal static void ThrowArgumentException(string message) => throw new ArgumentException(message);
}
