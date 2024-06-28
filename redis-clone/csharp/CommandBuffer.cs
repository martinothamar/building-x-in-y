using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;

namespace RedisClone;

internal unsafe struct CommandBuffer : IDisposable
{
    public Command* Ptr;
    public int Capacity;
    public int Length;

    public readonly Span<Command> Span => new Span<Command>(Ptr, Length);

    public ref Command Add()
    {
        Assert(Ptr is not null, "CommandBuffer must be allocated");
        Assert(Capacity > 0, "CommandBuffer must have capacity");

        var newLength = Length + 1;
        if (newLength > Capacity)
            Grow();

        ref var cmd = ref Unsafe.AsRef<Command>(Ptr + Length);
        Length = newLength;
        return ref cmd;
    }

    public static CommandBuffer Allocate(int capacity = 4)
    {
        CommandBuffer r = default;
        r.Capacity = capacity;
        r.Length = 0;
        r.Ptr = (Command*)NativeMemory.AlignedAlloc((nuint)(sizeof(Command) * capacity), 64);
        return r;
    }

    private void Grow()
    {
        var newLength = Capacity * 2;
        Ptr = (Command*)NativeMemory.AlignedRealloc(Ptr, (nuint)(sizeof(Command) * newLength), 64);
        Capacity = newLength;
    }

    public void Dispose()
    {
        if (Ptr is not null)
        {
            foreach (ref var cmd in Span)
            {
                cmd.Dispose();
            }
            NativeMemory.AlignedFree(Ptr);
        }
    }
}
