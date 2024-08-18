using System.Runtime.CompilerServices;
using static Sync.Core.Prelude;

namespace Sync.Core;

public enum SessionState : byte
{
    None,
    Starting,
    Running,
}

public enum SessionAction : byte
{
    None,
    Start,
}

public sealed class Session
{
    public readonly UInt128 Id;
    public readonly byte PartitionCount;
    private readonly TimeProvider _clock;
    private SessionState _state;

    public SessionState State => _state;
    private DateTimeOffset? _startSent;

    private Session(UInt128 id, byte partitionCount, TimeProvider clock)
    {
        Id = id;
        PartitionCount = partitionCount;
        _state = SessionState.None;
        _clock = clock;
    }

    public SessionAction Tick()
    {
        switch (_state)
        {
            case SessionState.None:
                Assert(_startSent is null, "Should never be set in this state");
                _startSent = _clock.GetUtcNow();
                _state = SessionState.Starting;
                return SessionAction.Start;
            case SessionState.Starting:
                Assert(_startSent is not null, "When we're starting, the start timestamp should have been recorded");
                var now = _clock.GetUtcNow();
                Assert(now - _startSent.Value < TimeSpan.FromSeconds(5), "For now, just assert that we never time out");
                return SessionAction.None;
            case SessionState.Running:
                return SessionAction.None;
            default:
                Assert(false, "SHould not reach this");
                return default;
        }
    }

    public void StartAcked()
    {
        Assert(_state == SessionState.Starting, "Must be in the correct state");
        _state = SessionState.Running;
    }

    public static Session Create(byte partitionCount, TimeProvider clock)
    {
        Assert(partitionCount > 0, "Should be atleast 1 partition");
        Unsafe.SkipInit<UInt128>(out var id);
        unsafe
        {
            var mem = new Span<byte>(Unsafe.AsPointer(ref id), sizeof(UInt128));
            Random.Shared.NextBytes(mem);
        }

        return new Session(id, partitionCount, clock);
    }
}
