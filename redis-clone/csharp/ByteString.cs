using System.Diagnostics.CodeAnalysis;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using System.Text;

namespace RedisClone;

internal unsafe struct ByteString : IEquatable<ByteString>, IDisposable
{
    private byte* _buf;
    private int _len;
    private bool _owned;

    public readonly int Length => _len;

    public readonly ReadOnlySpan<byte> Span => new(_buf, _len);

    public readonly bool IsSet => _buf is not null;

    private ByteString(byte* buf, int len, bool owned)
    {
        _buf = buf;
        _len = len;
        _owned = owned;
    }

    public override readonly string ToString() => Encoding.ASCII.GetString(_buf, _len);

    public override readonly bool Equals([NotNullWhen(true)] object? obj) => obj is ByteString other && Equals(other);

    public override readonly int GetHashCode()
    {
        HashCode hash = default;
        hash.AddBytes(Span);
        return hash.ToHashCode();
    }

    public readonly bool Equals(ByteString other) => Span.SequenceEqual(other.Span);

    public static ByteString BorrowFrom(ReadOnlySpan<byte> buf) =>
        new ByteString((byte*)Unsafe.AsPointer(ref MemoryMarshal.GetReference(buf)), buf.Length, owned: false);

    public void Copy()
    {
        Assert(!_owned, "Copying only makes sense for borrowed strings");
        var ptr = (byte*)NativeMemory.Alloc((nuint)_len);
        Assert(ptr is not null, "Allocation should succeed");
        Span.CopyTo(new Span<byte>(ptr, _len));
        _buf = ptr;
        _owned = true;
    }

    public void CopyFrom(in ByteString source)
    {
        Assert(source._len <= _len, "Must fit within buffer");
        Assert(_owned && !source._owned, "Buffer must be owned, while soure should be borrowed");
        source.Span.CopyTo(new Span<byte>(_buf, source._len));
        _len = source._len;
    }

    public void Dispose()
    {
        Assert(_buf is not null, "buffer should be allocated");
        if (_owned)
            NativeMemory.Free(_buf);
        _buf = null;
        _len = default;
        _owned = default;
    }
}
