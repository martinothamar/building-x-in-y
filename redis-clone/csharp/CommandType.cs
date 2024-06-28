namespace RedisClone;

[Flags]
internal enum CommandType : byte
{
    None = 0,
    Get = 1 << 0,
    Set = 1 << 1,
    Ping = 1 << 2,
    Config = 1 << 3,
    Command = 1 << 4,
}
