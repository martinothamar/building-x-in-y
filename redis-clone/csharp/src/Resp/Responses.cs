namespace RedisClone;

internal static class Responses
{
    public static readonly byte[] NotFound = "$-1\r\n"u8.ToArray();

    public static readonly byte[] OK = "+OK\r\n"u8.ToArray();

    public static readonly byte[] Pong = "+PONG\r\n"u8.ToArray();

    public static readonly byte[] ConfigSave = "*2\r\n$4\r\nsave\r\n$0\r\n\r\n"u8.ToArray();

    public static readonly byte[] ConfigAppendOnly = "*2\r\n$10\r\nappendonly\r\n$2\r\nno\r\n"u8.ToArray();

    public static readonly byte[] Command = "*2\r\n*2\r\n$3\r\nGET\r\n:2\r\n*2\r\n$3\r\nSET\r\n:2\r\n"u8.ToArray();
}
