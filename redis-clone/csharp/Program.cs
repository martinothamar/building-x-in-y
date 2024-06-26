global using static Assertion;
using System.Buffers;
using System.Collections.Concurrent;
using System.Diagnostics.CodeAnalysis;
using System.Globalization;
using System.Net;
using System.Net.Sockets;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using System.Text;

[module: SkipLocalsInit]

Assert(RuntimeInformation.IsOSPlatform(OSPlatform.Linux), "Only Linux-support");

var tcs = new TaskCompletionSource();
using var cts = new CancellationTokenSource();
var signals = PosixSignalRegistration.Create(PosixSignal.SIGINT, c => tcs.TrySetResult());

var cores = Environment.ProcessorCount;
var threads = new Task[cores / 2 / 2];

var storage = new Storage();

for (int i = 0; i < threads.Length; i++)
{
    threads[i] = Task.Factory
        .StartNew(
            static s =>
            {
                var state = s as State;
                Assert(state is not null, "listener thread received invalid state");
                return ServerThread(state);
            },
            new State(cts.Token, storage),
            cts.Token,
            TaskCreationOptions.DenyChildAttach,
            TaskScheduler.Default
        )
        .Unwrap();
}

await tcs.Task;
await cts.CancelAsync();

static async Task ServerThread(State state)
{
    await Console.Out.WriteLineAsync("Server thread starting");
    try
    {
        var cancellationToken = state.Stopping;

        using var socket = new Socket(AddressFamily.InterNetwork, SocketType.Stream, ProtocolType.Tcp);
        socket.SetSocketOption(SocketOptionLevel.Socket, SocketOptionName.ReuseAddress, true);
        socket.NoDelay = true;
        socket.Bind(IPEndPoint.Parse("127.0.0.1:6379"));
        socket.Listen();

        while (true)
        {
            var clientSocket = await socket.AcceptAsync(cancellationToken);
            var clientState = new ClientState(cancellationToken, clientSocket, state.Storage);
            await Task.Factory.StartNew(
                static s =>
                {
                    var state = s as ClientState;
                    Assert(state is not null, "listener thread received invalid state");
                    return ClientThread(state);
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
        await Console.Error.WriteLineAsync($"Server thread crash: {ex}");
    }
}

static async Task ClientThread(ClientState state)
{
    await Console.Out.WriteLineAsync("Client connected");
    byte[]? recvBuffer = null;
    byte[]? sendBuffer = null;
    int read = 0;
    try
    {
        var cancellationToken = state.Stopping;
        using var socket = state.Socket;

        const int BufferSize = 1024 * 1024;
        recvBuffer = ArrayPool<byte>.Shared.Rent(BufferSize);
        sendBuffer = ArrayPool<byte>.Shared.Rent(BufferSize);
        Assert(recvBuffer.Length >= BufferSize, "buffer size");

        var storage = state.Storage;

        Command cmd;
        while (true)
        {
            read = await socket.ReceiveAsync(recvBuffer, cancellationToken);

            if (read == 0)
            {
                break; // EOF
            }

            cmd = Parse(recvBuffer, read);

            switch (cmd.Type)
            {
                case CommandType.Get:
                    Assert(cmd.Key.IsSet, "Need key to process GET");
                    if (storage.TryGetValue(in cmd.Key, out var value))
                    {
                        var response = FormatResponse(sendBuffer, in value);
                        await socket.SendAsync(response, cancellationToken);
                    }
                    else
                    {
                        await socket.SendAsync(Responses.NotFound, cancellationToken);
                    }
                    break;
                case CommandType.Set:
                    Assert(cmd.Key.IsSet, "Need key to process SET");
                    Assert(cmd.Value.IsSet, "Need value to process SET");
                    var val = cmd.Value.Copy();
                    storage.Set(in cmd.Key, in val);

                    await socket.SendAsync(Responses.OK, cancellationToken);
                    break;
                case CommandType.Ping:
                    await socket.SendAsync(Responses.Pong, cancellationToken);
                    break;
                case CommandType.Config:
                    await socket.SendAsync(Responses.ConfigSave, cancellationToken);
                    await socket.SendAsync(Responses.ConfigAppendOnly, cancellationToken);
                    break;
            }
        }

        socket.Close();
    }
    catch (Exception ex) when (ex is not OperationCanceledException)
    {
        var str = recvBuffer is not null && read > 0 ? Encoding.ASCII.GetString(recvBuffer, 0, read) : "";
        str = str.Replace("\r\n", "\\r\\n");
        await Console.Error.WriteLineAsync($"Client thread crash for msg '{str}': {ex}");
    }
    finally
    {
        if (recvBuffer is not null)
            ArrayPool<byte>.Shared.Return(recvBuffer);
        if (sendBuffer is not null)
            ArrayPool<byte>.Shared.Return(sendBuffer);
    }

    static unsafe Command Parse(byte[] buffer, int read)
    {
        Command cmd = default;

        var ptr = (byte*)Unsafe.AsPointer(ref MemoryMarshal.GetReference(buffer.AsSpan(0, read)));

        // 'GET key' - *2\r\n$3\r\nGET\r\n$3\r\nkey\r\n
        // 'CONFIG GET save CONFIG get appendonly' - *3\r\n$6\r\nCONFIG\r\n$3\r\nGET\r\n$4\r\nsave\r\n*3\r\n$6\r\nCONFIG\r\n$3\r\nGET\r\n$10\r\nappendonly\r\n

        if (IsPing(ptr))
        {
            cmd.Type = CommandType.Ping;
            return cmd;
        }

        Assert(*(ptr + 0) == (byte)'*', "Should start with string command");
        Assert(*(ptr + 1) is (byte)'2' or (byte)'3', "Command arrays with 2|3 length");
        ptr += 2;
        AssertCRLF(&ptr);

        cmd.Type = ParseType(&ptr);
        Assert(cmd.Type != CommandType.None, "Should always have valid resp command");

        var key = ParseString(&ptr);
        cmd.Key = ByteString.BorrowFrom(key);

        if (cmd.Type == CommandType.Set)
        {
            var value = ParseString(&ptr);
            cmd.Value = ByteString.BorrowFrom(value);
        }

        return cmd;
    }

    static unsafe bool IsPing(byte* ptr)
    {
        if (*(ptr + 0) == (byte)'P' && *(ptr + 1) == (byte)'I' && *(ptr + 2) == (byte)'N' && *(ptr + 3) == (byte)'G')
            return true;

        var ping = "*1\r\n$4\r\nPING\r\n"u8;
        if (ping.SequenceEqual(new ReadOnlySpan<byte>(ptr, ping.Length)))
            return true;

        return false;
    }

    static unsafe ReadOnlyMemory<byte> FormatResponse(byte[] sendBuffer, in ByteString value)
    {
        var val = value.Span;
        sendBuffer[0] = (byte)'$';
        Assert(
            val.Length.TryFormat(sendBuffer.AsSpan(1), out var lenBytes, provider: CultureInfo.InvariantCulture),
            "Must write len prefix"
        );
        sendBuffer[1 + lenBytes + 0] = (byte)'\r';
        sendBuffer[1 + lenBytes + 1] = (byte)'\n';
        val.CopyTo(sendBuffer.AsSpan(3 + lenBytes));
        sendBuffer[3 + lenBytes + val.Length + 0] = (byte)'\r';
        sendBuffer[3 + lenBytes + val.Length + 1] = (byte)'\n';
        var result = sendBuffer.AsMemory(0, 3 + lenBytes + val.Length + 2);

        var s = Encoding.ASCII.GetString(result.Span);

        return result;
    }

    static unsafe CommandType ParseType(byte** ptrRef)
    {
        var src = ParseString(ptrRef);
        CommandType result = default;
        if (src.SequenceEqual("GET"u8))
            result = CommandType.Get;
        else if (src.SequenceEqual("SET"u8))
            result = CommandType.Set;
        else if (src.SequenceEqual("CONFIG"u8))
            result = CommandType.Config;

        return result;
    }

    static unsafe ReadOnlySpan<byte> ParseString(byte** ptrRef)
    {
        Assert(*(*ptrRef + 0) == (byte)'$', "Should start with string command");
        *ptrRef += 1;

        var i = ReadUntilCRLF(ptrRef);
        Assert(
            int.TryParse(new ReadOnlySpan<byte>(*ptrRef, i), CultureInfo.InvariantCulture, out var len),
            "size param"
        );
        *ptrRef += i;
        AssertCRLF(ptrRef);

        var result = new ReadOnlySpan<byte>(*ptrRef, len);
        *ptrRef += len;
        AssertCRLF(ptrRef);
        return result;
    }

    static unsafe int ReadUntilCRLF(byte** ptrRef)
    {
        var ptr = *ptrRef;
        int i = 0;
        for (; ; i++)
        {
            if (*(ptr + i) == (byte)'\r')
                break;
        }

        return i;
    }

    static unsafe void AssertCRLF(byte** ptrRef)
    {
        var ptr = *ptrRef;

        Assert(*(ptr + 0) == (byte)'\r', "CR");
        Assert(*(ptr + 1) == (byte)'\n', "LF");

        *ptrRef += 2;
    }
}

struct Command
{
    public CommandType Type;

    public ByteString Key;
    public ByteString Value;
}

enum CommandType : byte
{
    None,
    Get,
    Set,
    Ping,
    Config,
}

sealed record State(CancellationToken Stopping, Storage Storage);

sealed record ClientState(CancellationToken Stopping, Socket Socket, Storage Storage);

sealed class Storage
{
    private readonly ConcurrentDictionary<ByteString, ByteString> _data;

    internal Storage()
    {
        _data = new();
    }

    public bool TryGetValue(in ByteString key, out ByteString value) => _data.TryGetValue(key, out value);

    public void Set(in ByteString key, in ByteString value) => _data[key] = value;
}

readonly unsafe struct ByteString : IEquatable<ByteString>, IDisposable
{
    private readonly byte* _buf;
    private readonly int _len;
    private readonly bool _owned;

    public readonly int Length => _len;

    public readonly ReadOnlySpan<byte> Span => new(_buf, _len);

    public readonly bool IsSet => _buf is not null;

    private ByteString(byte* buf, int len, bool owned)
    {
        _buf = buf;
        _len = len;
        _owned = owned;
    }

    public override string ToString() => Encoding.ASCII.GetString(_buf, _len);

    public override bool Equals([NotNullWhen(true)] object? obj) => obj is ByteString other && Equals(other);

    public override int GetHashCode()
    {
        HashCode hash = default;
        hash.AddBytes(new ReadOnlySpan<byte>(_buf, _len));
        return hash.ToHashCode();
    }

    public bool Equals(ByteString other) =>
        new ReadOnlySpan<byte>(_buf, _len).SequenceEqual(new ReadOnlySpan<byte>(other._buf, other._len));

    public static ByteString CopyFrom(byte* origBuf, int len)
    {
        var buf = (byte*)NativeMemory.Alloc((nuint)len);
        new ReadOnlySpan<byte>(origBuf, len).CopyTo(new Span<byte>(buf, len));

        return new ByteString(buf, len, owned: true);
    }

    public static ByteString BorrowFrom(byte* buf, int len) => new ByteString(buf, len, owned: false);

    public static ByteString BorrowFrom(ReadOnlySpan<byte> buf) =>
        new ByteString((byte*)Unsafe.AsPointer(ref MemoryMarshal.GetReference(buf)), buf.Length, owned: false);

    public ByteString Copy() => CopyFrom(_buf, _len);

    public void Dispose()
    {
        if (_owned)
            NativeMemory.Free(_buf);
    }
}

static class Responses
{
    public static readonly byte[] NotFound = "$-1\r\n"u8.ToArray();

    public static readonly byte[] OK = "+OK\r\n"u8.ToArray();

    public static readonly byte[] Pong = "+PONG\r\n"u8.ToArray();

    public static readonly byte[] ConfigSave = "*2\r\n$4\r\nsave\r\n$0\r\n\r\n"u8.ToArray();

    public static readonly byte[] ConfigAppendOnly = "*2\r\n$10\r\nappendonly\r\n$2\r\nno\r\n"u8.ToArray();
}
