using System.Diagnostics;
using System.Runtime.CompilerServices;

namespace CalculationEngine;

internal ref struct StackStack<T>
    where T : unmanaged
{
    private readonly Span<T> _data;
    private int _size;

    public readonly int Count => _size;

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    internal StackStack(Span<T> data)
    {
        _data = data;
        _size = 0;
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    internal ref T Push()
    {
        Debug.Assert(_size < _data.Length);
        var size = _size;
        ref var ptr = ref _data[size];
        _size = size + 1;
        return ref ptr;
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    internal ref T Pop()
    {
        Debug.Assert(_size > 0);
        var size = _size;
        ref var ptr = ref _data[size - 1];
        _size = size - 1;
        return ref ptr;
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    internal void Clear()
    {
        _size = 0;
    }
}
