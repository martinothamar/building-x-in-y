using System.Globalization;

namespace RedisClone;

internal static class RespParser
{
    private static ReadOnlySpan<byte> CRLF => "\r\n"u8;

    public static bool TryParse(ArenaAllocator allocator, ReadOnlySpan<byte> data, ref CommandBuffer commandBuffer)
    {
        while (data.Length > 0)
        {
            switch (data[0])
            {
                case (byte)'$':
                {
                    if (!TryParseBulkString(ref data, out var strArg))
                        return false;
                    ref var cmd = ref commandBuffer.Add();
                    cmd = Command.Allocate(allocator, 1);
                    cmd.Add(ref strArg);
                    break;
                }
                case (byte)'*':
                {
                    if (!TryParseArray(allocator, ref data, ref commandBuffer))
                        return false;
                    break;
                }
                case (byte)'P':
                {
                    Assert(data.SequenceEqual("PING\r\n"u8), "Invalid command");
                    ref var cmd = ref commandBuffer.Add();
                    cmd = Command.Allocate(allocator, 1);
                    var arg = new CommandArg(data.Slice(0, "PING"u8.Length), ValueKind.BulkString);
                    cmd.Add(ref arg);
                    data = data.Slice("PING\r\n"u8.Length);
                    break;
                }
                default:
                    Assert(false, "fallthrough case, invalid root type");
                    return false;
            }
        }

        return true;
    }

    static bool TryParseArray(ArenaAllocator allocator, ref ReadOnlySpan<byte> data, ref CommandBuffer commandBuffer)
    {
        if (!TryParseLength(ref data, out var arrayLength))
            return false;

        ref var cmd = ref commandBuffer.Add();
        cmd = Command.Allocate(allocator, arrayLength);

        for (int i = 0; i < arrayLength; i++)
        {
            Assert(data[0] is (byte)'$', "Array elements must be bulk string");
            if (!TryParseBulkString(ref data, out var strArg))
                return false;
            cmd.Add(ref strArg);
        }

        return true;
    }

    static unsafe bool TryParseBulkString(ref ReadOnlySpan<byte> data, out CommandArg arg)
    {
        arg = default;
        Assert(data[0] is (byte)'$', "bulk strings are '$' prefixed");
        if (!TryParseLength(ref data, out var valLength))
            return false;
        if (valLength >= data.Length)
            return false;

        if (!data.Slice(valLength).StartsWith(CRLF))
            return false;

        arg = new CommandArg(data.Slice(0, valLength), ValueKind.BulkString);
        data = data.Slice(valLength + CRLF.Length);

        return true;
    }

    static bool TryParseLength(ref ReadOnlySpan<byte> data, out int length)
    {
        length = -1;
        Assert(data[0] is (byte)'$' or (byte)'*', "current element must be array or bulk string");
        data = data.Slice(1);
        var integerStrLength = data.IndexOf(CRLF);
        if (integerStrLength == -1)
            return false;
        if (!data.Slice(integerStrLength).StartsWith(CRLF))
            return false;

        length = int.Parse(data.Slice(0, integerStrLength), CultureInfo.InvariantCulture);
        data = data.Slice(integerStrLength + CRLF.Length);

        return true;
    }
}
