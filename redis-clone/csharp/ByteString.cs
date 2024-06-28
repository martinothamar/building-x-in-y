using System.Diagnostics.CodeAnalysis;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using System.Text;

namespace RedisClone;

internal readonly unsafe struct ByteString : IEquatable<ByteString>, IDisposable
{
    private readonly byte* _buf;
    private readonly int _len;
    private readonly bool _owned;

    public readonly int Length => _len;

    public readonly ReadOnlySpan<byte> Span => new(_buf, _len);

    public readonly bool IsSet => _buf is not null;

    private ByteString(byte* buf, int len, bool owned)
    {
        _buf = buf;
        _len = len;
        _owned = owned;
    }

    public override string ToString() => Encoding.ASCII.GetString(_buf, _len);

    public override bool Equals([NotNullWhen(true)] object? obj) => obj is ByteString other && Equals(other);

    public override int GetHashCode()
    {
        HashCode hash = default;
        hash.AddBytes(new ReadOnlySpan<byte>(_buf, _len));
        return hash.ToHashCode();
    }

    public bool Equals(ByteString other) =>
        new ReadOnlySpan<byte>(_buf, _len).SequenceEqual(new ReadOnlySpan<byte>(other._buf, other._len));

    public static ByteString CopyFrom(byte* origBuf, int len)
    {
        var buf = (byte*)NativeMemory.Alloc((nuint)len);
        new ReadOnlySpan<byte>(origBuf, len).CopyTo(new Span<byte>(buf, len));

        return new ByteString(buf, len, owned: true);
    }

    public static ByteString CopyFrom(ReadOnlySpan<byte> buf) =>
        CopyFrom((byte*)Unsafe.AsPointer(ref MemoryMarshal.GetReference(buf)), buf.Length);

    public static ByteString BorrowFrom(byte* buf, int len) => new ByteString(buf, len, owned: false);

    public static ByteString BorrowFrom(ReadOnlySpan<byte> buf) =>
        new ByteString((byte*)Unsafe.AsPointer(ref MemoryMarshal.GetReference(buf)), buf.Length, owned: false);

    public ByteString Copy() => CopyFrom(_buf, _len);

    public void Dispose()
    {
        if (_owned)
            NativeMemory.Free(_buf);
    }
}
