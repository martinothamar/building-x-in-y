using System.Collections.Concurrent;

namespace RedisClone;

internal sealed class Storage
{
    private readonly ConcurrentDictionary<ByteString, ByteString> _data;

    internal Storage()
    {
        _data = new();
    }

    public bool TryGetValue(in ByteString key, out ByteString value) => _data.TryGetValue(key, out value);

    public void Set(in ByteString key, in ByteString value) => _data[key] = value;
}
