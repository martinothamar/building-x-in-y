using System.Net.Sockets;

namespace RedisClone;

internal sealed record ClientState(int ServerThreadId, Server Server, CancellationToken Stopping, Socket Socket);
