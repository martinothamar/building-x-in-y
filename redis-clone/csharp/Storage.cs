using System.Collections.Concurrent;

namespace RedisClone;

internal sealed class Storage : IDisposable
{
    private readonly CancellationToken _stopping;
    private readonly ConcurrentDictionary<ByteString, Record> _data;

    internal Storage(CancellationToken stopping)
    {
        _stopping = stopping;
        _data = new();
    }

    private record struct Record(ByteString Value, ulong Generation);

    public bool TryGetValue(ref ByteString key, out ByteString value)
    {
        if (!_data.TryGetValue(key, out var record))
        {
            value = default;
            return false;
        }

        value = record.Value;
        return true;
    }

    public void Set(ref ByteString key, ref ByteString value)
    {
        Assert(!_stopping.IsCancellationRequested, "Should not receive SET's when shutting down");

        _data.AddOrUpdate(
            key,
            static (key, value) =>
            {
                key.Copy();
                value.Copy();
                return new Record(value, 0);
            },
            static (_, record, value) =>
            {
                if (record.Value.Equals(value))
                    return record;

                record.Generation++;
                if (value.Length <= record.Value.Length)
                {
                    record.Value.CopyFrom(value);
                }
                else
                {
                    record.Value.Dispose();
                    value.Copy();
                    record.Value = value;
                }
                return record;
            },
            value
        );
    }

    public void Dispose()
    {
        Assert(_stopping.IsCancellationRequested, "Should only dispose storage when server is shutting down");
        foreach (var kvp in _data)
        {
            kvp.Key.Dispose();
            kvp.Value.Value.Dispose();
        }
        _data.Clear();
    }
}
