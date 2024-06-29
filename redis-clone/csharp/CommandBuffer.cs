using System.Runtime.CompilerServices;

namespace RedisClone;

internal unsafe struct CommandBuffer
{
    private const int Capacity = 1024 * 4;
    private Command* _ptr;
    public int Length { get; private set; }

    private CommandBuffer(Command* ptr)
    {
        _ptr = ptr;
        Length = 0;
    }

    public ref Command Add()
    {
        Assert(Length < Capacity, "fixed CommandBuffer capacity exceeded");
        ref var cmd = ref Unsafe.AsRef<Command>(_ptr + Length);
        Length++;
        return ref cmd;
    }

    public Span<Command> Span => new Span<Command>(_ptr, Length);

    public static CommandBuffer Allocate(ArenaAllocator allocator)
    {
        var ptr = allocator.AllocatePtr<Command>(Capacity);
        return new CommandBuffer(ptr);
    }
}
