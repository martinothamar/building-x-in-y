global using static RedisClone.Assertion;
using System.Runtime.CompilerServices;
using RedisClone;

[module: SkipLocalsInit]

var server = Server.Create("127.0.0.1");
await server.Run();
