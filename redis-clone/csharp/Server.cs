using System.Buffers;
using System.Globalization;
using System.Net;
using System.Net.Sockets;
using System.Runtime.InteropServices;
using System.Text;

namespace RedisClone;

/// <summary>
/// Redis server class
/// </summary>
public sealed class Server : IDisposable
{
    private const int BufferSize = 1024 * 1024;

    private readonly string _host;
    private readonly int _port;

    private CancellationTokenSource? _cts;
    private Storage? _storage;

    private Server(string host, int port)
    {
        _host = host;
        _port = port;
    }

    /// <summary>
    /// Run the Redis server, blocking the main thread
    /// </summary>
    /// <param name="cancellationToken">Cancellation token</param>
    /// <returns>Task</returns>
    public async Task Run(CancellationToken cancellationToken = default)
    {
        Assert(RuntimeInformation.IsOSPlatform(OSPlatform.Linux), "Only Linux-support");

        var tcs = new TaskCompletionSource();
        _cts =
            cancellationToken != default
                ? CancellationTokenSource.CreateLinkedTokenSource(cancellationToken)
                : new CancellationTokenSource();
        var signals = PosixSignalRegistration.Create(PosixSignal.SIGINT, c => tcs.TrySetResult());

        var cores = Environment.ProcessorCount;
        var threads = new Task[cores / 2 / 2];

        _storage = new Storage();

        for (int i = 0; i < threads.Length; i++)
        {
            threads[i] = Task
                .Factory.StartNew(
                    static s =>
                    {
                        var state = s as State;
                        Assert(state is not null, "listener thread received invalid state");
                        return state.Server.ServerThread(state);
                    },
                    new State(i, this, _cts.Token),
                    _cts.Token,
                    TaskCreationOptions.DenyChildAttach,
                    TaskScheduler.Default
                )
                .Unwrap();
        }

        await tcs.Task;
        await _cts.CancelAsync();
    }

    /// <summary>
    /// Create an instance of a Redis server
    /// </summary>
    /// <param name="host">host, i.e. 127.0.0.1</param>
    /// <param name="port">port, default is 6379</param>
    /// <returns></returns>
    public static Server Create(string host, int port = 6379)
    {
        return new Server(host, port);
    }

    async Task ServerThread(State state)
    {
        await Console.Out.WriteLineAsync($"[{state.Id}] Server thread starting");
        try
        {
            var cancellationToken = state.Stopping;

            using var socket = new Socket(AddressFamily.InterNetwork, SocketType.Stream, ProtocolType.Tcp);
            socket.SetSocketOption(SocketOptionLevel.Socket, SocketOptionName.ReuseAddress, true);
            socket.NoDelay = true;
            socket.Bind(IPEndPoint.Parse($"{_host}:{_port}"));
            socket.Listen();

            while (true)
            {
                var clientSocket = await socket.AcceptAsync(cancellationToken);
                Assert(clientSocket.RemoteEndPoint is not null, "remote endpoint");
                var clientState = new ClientState(state.Id, state.Server, cancellationToken, clientSocket);
                await Task.Factory.StartNew(
                    static s =>
                    {
                        var state = s as ClientState;
                        Assert(state is not null, "listener thread received invalid state");
                        return state.Server.ClientThread(state);
                    },
                    clientState,
                    cancellationToken,
                    TaskCreationOptions.DenyChildAttach,
                    TaskScheduler.Default
                );
            }
        }
        catch (Exception ex) when (ex is not OperationCanceledException)
        {
            await Console.Error.WriteLineAsync($"[{state.Id}] Server thread crash: {ex}");
        }
    }

    async Task ClientThread(ClientState state)
    {
        Assert(state.Socket.RemoteEndPoint is not null, "remote endpoint");
        var clientId = state.Socket.RemoteEndPoint.ToString();
        await Console.Out.WriteLineAsync($"[{state.ServerThreadId}] Client connected: '{clientId}'");
        byte[]? recvBuffer = null;
        byte[]? sendBuffer = null;
        int read = 0;
        try
        {
            var cancellationToken = state.Stopping;
            using var socket = state.Socket;

            recvBuffer = ArrayPool<byte>.Shared.Rent(BufferSize);
            sendBuffer = ArrayPool<byte>.Shared.Rent(BufferSize);
            Assert(recvBuffer.Length >= BufferSize, "buffer size");
            Assert(sendBuffer.Length >= BufferSize, "buffer size");
            int receiveFrom = 0;
            Memory<byte> buffer = recvBuffer.AsMemory(0, BufferSize);
            while (true)
            {
                read = await socket.ReceiveAsync(
                    receiveFrom == 0 ? buffer : buffer.Slice(receiveFrom),
                    cancellationToken
                );

                if (read == 0)
                    break; // EOF
                if (receiveFrom + read > BufferSize)
                    break; // Too large

                var response = Handle(recvBuffer, receiveFrom + read, sendBuffer);
                if (response.IsEmpty)
                {
                    receiveFrom += read;
                }
                else
                {
                    await socket.SendAsync(response, cancellationToken);
                    receiveFrom = 0;
                }
            }

            socket.Close();
        }
        catch (Exception ex) when (ex is not OperationCanceledException)
        {
            var str = recvBuffer is not null && read > 0 ? Encoding.ASCII.GetString(recvBuffer, 0, read) : "";
            str = str.Replace("\r\n", "\\r\\n");
            await Console.Error.WriteLineAsync(
                $"[{state.ServerThreadId}] Client thread '{clientId}' crash for msg '{str}': {ex}"
            );
        }
        finally
        {
            if (recvBuffer is not null)
                ArrayPool<byte>.Shared.Return(recvBuffer);
            if (sendBuffer is not null)
                ArrayPool<byte>.Shared.Return(sendBuffer);
        }
    }

    ReadOnlyMemory<byte> Handle(byte[] buffer, int read, byte[] sendBuffer)
    {
        ReadOnlySpan<byte> data = buffer.AsSpan(0, read);
        var outbox = sendBuffer.AsSpan(0, BufferSize);

        var commandBuffer = CommandBuffer.Allocate(4);
        try
        {
            if (!RespParser.TryParse(data, ref commandBuffer))
                return ReadOnlyMemory<byte>.Empty;

            var commands = commandBuffer.Span;
            foreach (ref var cmd in commands)
                HandleCommand(ref outbox, ref cmd);

            return sendBuffer.AsMemory(0, sendBuffer.Length - outbox.Length);
        }
        finally
        {
            commandBuffer.Dispose();
        }

        // 'GET key' - *2\r\n$3\r\nGET\r\n$3\r\nkey\r\n
        // 'CONFIG GET save CONFIG get appendonly' - *3\r\n$6\r\nCONFIG\r\n$3\r\nGET\r\n$4\r\nsave\r\n*3\r\n$6\r\nCONFIG\r\n$3\r\nGET\r\n$10\r\nappendonly\r\n
        // 'COMMAND' - *1\r\n$7\r\nCOMMAND\r\n
    }

    void HandleCommand(ref Span<byte> outbox, ref Command cmd)
    {
        ref readonly var cmdName = ref cmd[0];
        var cmdNameValue = cmdName.Span;
        Assert(cmdName.Kind is ValueKind.BulkString, "First arg should always be string");

        switch (cmdNameValue.Length)
        {
            case 3:
            {
                if (cmdNameValue.SequenceEqual("GET"u8))
                {
                    HandleGetCommand(ref outbox, ref cmd);
                }
                else if (cmdNameValue.SequenceEqual("SET"u8))
                {
                    HandleSetCommand(ref outbox, ref cmd);
                }
                else
                    Assert(false, "Invalid command");
                break;
            }
            case 4:
            {
                if (cmdNameValue.SequenceEqual("PING"u8))
                {
                    HandlePingCommand(ref outbox, ref cmd);
                }
                else
                    Assert(false, "Invalid command");
                break;
            }
            case 6:
            {
                if (cmdNameValue.SequenceEqual("CONFIG"u8))
                {
                    HandleConfigCommand(ref outbox, ref cmd);
                }
                else
                    Assert(false, "Invalid command");
                break;
            }
            case 7:
            {
                if (cmdNameValue.SequenceEqual("COMMAND"u8))
                {
                    HandleCommandCommand(ref outbox, ref cmd);
                }
                else
                    Assert(false, "Invalid command");
                break;
            }
        }
    }

    void HandleGetCommand(ref Span<byte> outbox, ref Command cmd)
    {
        Assert(cmd.Length is 2, "GET has two args");
        Assert(_storage is not null, "storage was null");
        var keyValue = ByteString.BorrowFrom(cmd[1].Span);
        if (_storage.TryGetValue(in keyValue, out var value))
        {
            RespWriter.WriteBulkString(ref outbox, in value);
        }
        else
        {
            Responses.NotFound.CopyTo(outbox);
            outbox = outbox.Slice(Responses.NotFound.Length);
        }
    }

    void HandleSetCommand(ref Span<byte> outbox, ref Command cmd)
    {
        Assert(cmd.Length is 3, "SET has three args");
        Assert(_storage is not null, "storage was null");
        var keyValue = ByteString.BorrowFrom(cmd[1].Span);
        var valueValue = ByteString.CopyFrom(cmd[2].Span);
        _storage.Set(in keyValue, in valueValue);

        Responses.OK.CopyTo(outbox);
        outbox = outbox.Slice(Responses.OK.Length);
    }

    void HandlePingCommand(ref Span<byte> outbox, ref Command cmd)
    {
        Assert(cmd.Length is 1, "PING should have no arguments");
        Responses.Pong.CopyTo(outbox);
        outbox = outbox.Slice(Responses.Pong.Length);
    }

    void HandleConfigCommand(ref Span<byte> outbox, ref Command cmd)
    {
        Assert(cmd.Length is 3, "should have 3 arguments for CONFIG");
        ref var subCmd = ref cmd[1];
        var subCmdValue = subCmd.Span;
        Assert(subCmdValue.SequenceEqual("GET"u8), "only support GET for CONFIG");

        ref var itemCmd = ref cmd[2];
        var itemCmdValue = itemCmd.Span;
        if (itemCmdValue.SequenceEqual("save"u8))
        {
            Responses.ConfigSave.CopyTo(outbox);
            outbox = outbox.Slice(Responses.ConfigSave.Length);
        }
        else if (itemCmdValue.SequenceEqual("appendonly"u8))
        {
            Responses.ConfigAppendOnly.CopyTo(outbox);
            outbox = outbox.Slice(Responses.ConfigAppendOnly.Length);
        }
        else
            Assert(false, "Invalid args for CONFIG");
    }

    void HandleCommandCommand(ref Span<byte> outbox, ref Command cmd)
    {
        Responses.Command.CopyTo(outbox);
        outbox = outbox.Slice(Responses.Command.Length);
    }

    /// <summary>
    /// Disposes the resources for the server
    /// </summary>
    public void Dispose()
    {
        _cts?.Dispose();
    }
}
