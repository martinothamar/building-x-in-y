using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using System.Text;

namespace RedisClone;

internal unsafe struct Command : IDisposable
{
    private CommandArg* _ptr;
    public int Capacity { get; private set; }
    public int Length { get; private set; }

    public readonly ref CommandArg this[int index] => ref Unsafe.AsRef<CommandArg>(_ptr + index);

    private Command(CommandArg* ptr, int capacity)
    {
        _ptr = ptr;
        Capacity = capacity;
        Length = 0;
    }

    // public readonly Span<CommandArg> Span => new Span<CommandArg>(_ptr, Length);

    public void Add(ref CommandArg arg)
    {
        Assert(_ptr is not null, "Command must be allocated");
        Assert(Length + 1 <= Capacity, "Should keep within capacity");
        *(_ptr + Length++) = arg;
    }

    public static Command Allocate(int capacity)
    {
        var ptr = (CommandArg*)NativeMemory.AlignedAlloc((nuint)(sizeof(CommandArg) * capacity), 64);
        return new Command(ptr, capacity);
    }

    public readonly void Dispose()
    {
        if (_ptr is not null)
        {
            NativeMemory.AlignedFree(_ptr);
        }
    }
}

internal unsafe struct CommandArg
{
    private byte* _ptr;
    public readonly int Length;
    public readonly ValueKind Kind;

    public readonly ReadOnlySpan<byte> Span => new ReadOnlySpan<byte>(_ptr, Length);

    public CommandArg(ReadOnlySpan<byte> span, ValueKind kind)
    {
        _ptr = (byte*)Unsafe.AsPointer(ref MemoryMarshal.GetReference(span));
        Length = span.Length;
        Kind = kind;
    }

    public override readonly string ToString() => Encoding.ASCII.GetString(Span);
}

internal enum ValueKind : byte
{
    BulkString,
}
