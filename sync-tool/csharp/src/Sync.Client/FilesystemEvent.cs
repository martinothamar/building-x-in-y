using System.Diagnostics;
using Sync.Core;

namespace Sync.Client;

internal abstract record SyncEvent
{
    private SyncEvent() { }

    internal sealed record Fs(FilesystemEvent Event) : SyncEvent();

    internal sealed record Tick() : SyncEvent();

    internal sealed record SessionActionCommand(SessionAction Action) : SyncEvent();

    internal sealed record SessionActionCompleted(SessionAction Action) : SyncEvent();
}

public abstract record FilesystemEvent
{
    private FilesystemEvent(SyncFilePath path)
    {
        // High resolution timestamp (CLOCK_MONOTONIC on Linux)
        Timestamp = Stopwatch.GetTimestamp();
        Path = path;
    }

    public long Timestamp { get; }

    public SyncFilePath Path { get; }

    public sealed record Init(SyncFilePath _path) : FilesystemEvent(_path);

    public sealed record Creation(SyncFilePath _path) : FilesystemEvent(_path);

    public sealed record Change(SyncFilePath _path) : FilesystemEvent(_path);

    public sealed record Deletion(SyncFilePath _path) : FilesystemEvent(_path);

    public sealed record Renaming(SyncFilePath _path) : FilesystemEvent(_path);
}
