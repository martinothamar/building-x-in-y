namespace RedisClone;

internal sealed record State(int Id, Server Server, CancellationToken Stopping);
