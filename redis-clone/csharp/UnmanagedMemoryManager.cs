using System.Buffers;

namespace RedisClone;

internal sealed unsafe class UnmanagedMemoryManager<T> : MemoryManager<T>
    where T : unmanaged
{
    private readonly T* _pointer;
    private readonly int _length;

    public UnmanagedMemoryManager(T* pointer, int length)
    {
        if (length < 0)
            throw new ArgumentOutOfRangeException(nameof(length));
        _pointer = pointer;
        _length = length;
    }

    public override Span<T> GetSpan() => new Span<T>(_pointer, _length);

    public override MemoryHandle Pin(int elementIndex = 0)
    {
        Assert(!(elementIndex < 0 || elementIndex >= _length), "Bounds check");
        return new MemoryHandle(_pointer + elementIndex);
    }

    public override void Unpin() { }

    protected override void Dispose(bool disposing) { }
}
