using System.Globalization;

namespace RedisClone;

internal static class RespWriter
{
    private static ReadOnlySpan<byte> CRLF => "\r\n"u8;

    public static void WriteBulkString(ref Span<byte> outbox, ref ByteString value)
    {
        var outboxIndex = 0;
        outbox[outboxIndex++] = (byte)'$';
        Assert(
            value.Length.TryFormat(outbox.Slice(1), out var lenBytes, provider: CultureInfo.InvariantCulture),
            "write len bytes for string"
        );
        outboxIndex += lenBytes;
        CRLF.CopyTo(outbox.Slice(outboxIndex));
        outboxIndex += CRLF.Length;
        var valueSpan = value.Span;
        valueSpan.CopyTo(outbox.Slice(outboxIndex));
        outboxIndex += valueSpan.Length;
        CRLF.CopyTo(outbox.Slice(outboxIndex));
        outboxIndex += CRLF.Length;
        outbox = outbox.Slice(outboxIndex);
    }
}
