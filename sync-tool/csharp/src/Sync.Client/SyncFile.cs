using Sync.Core;

namespace Sync.Client;

public sealed class SyncFile(SyncFilePath path) : IEquatable<SyncFile>
{
    public readonly SyncFilePath Path = path;

    public override bool Equals(object? obj) => obj is SyncFile other && Equals(other);

    public bool Equals(SyncFile? other) => other is not null && other.Equals(Path);

    public override int GetHashCode() => Path.GetHashCode();

    public override string ToString() => Path.ToString();
}
