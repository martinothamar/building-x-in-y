using System.Net.Sockets;
using Spectre.Console;
using Sync.Core;

namespace Sync.Client;

public interface IIO
{
    IFsWatcher Watch<TState>(string dir, TState state, Action<TState, FilesystemEvent> callback);

    bool DirectoryExists(string dir);

    string GetAbsolutePath(string path);

    IReadOnlyList<SyncFilePath> GetFiles(string dir);

    ValueTask<IIoSocket> Connect(string host, int port, CancellationToken cancellationToken);
}

public interface IIoSocket
{
    ValueTask Send(ReadOnlyMemory<byte> buffer, CancellationToken cancellationToken);
}

public interface IFsWatcher : IDisposable { }

internal sealed class DefaultIO : IIO
{
    public IReadOnlyList<SyncFilePath> GetFiles(string dir)
    {
        // This uses sync IO APIs, but this is just for initialization so it won't affect throughput
        return Directory
            .GetFiles(dir, "*", SearchOption.AllDirectories)
            .Select(f => new SyncFilePath(Path.GetFullPath(f)))
            .ToArray();
    }

    public IFsWatcher Watch<TState>(string dir, TState state, Action<TState, FilesystemEvent> callback) =>
        new FsWatcher<TState>(this, dir, state, callback);

    public bool DirectoryExists(string dir) => Directory.Exists(dir);

    public string GetAbsolutePath(string path) => Path.GetFullPath(path);

    public async ValueTask<IIoSocket> Connect(string host, int port, CancellationToken cancellationToken)
    {
        var socket = new Socket(AddressFamily.InterNetwork, SocketType.Stream, ProtocolType.Tcp);
        socket.SetSocketOption(SocketOptionLevel.Socket, SocketOptionName.KeepAlive, false);
        socket.NoDelay = true;

        await socket.ConnectAsync(host, port);
        return new IoSocket(socket);
    }

    private sealed class IoSocket(Socket socket) : IIoSocket
    {
        private readonly Socket _socket = socket;

        public async ValueTask Send(ReadOnlyMemory<byte> buffer, CancellationToken cancellationToken)
        {
            var written = 0;
            while (written < buffer.Length)
            {
                written += await _socket.SendAsync(buffer.Slice(written), cancellationToken);
            }
        }
    }

    private sealed class FsWatcher<TState> : IFsWatcher
    {
        private readonly FileSystemWatcher _watcher;

        public FsWatcher(IIO io, string dir, TState state, Action<TState, FilesystemEvent> callback)
        {
            // Watcher runs on separate background thread
            _watcher = new FileSystemWatcher(dir);

            // Register callbacks
            _watcher.Created += (_, e) =>
                callback(state, new FilesystemEvent.Creation(new SyncFilePath(io.GetAbsolutePath(e.FullPath))));
            _watcher.Changed += (_, e) =>
                callback(state, new FilesystemEvent.Change(new SyncFilePath(io.GetAbsolutePath(e.FullPath))));
            _watcher.Deleted += (_, e) =>
                callback(state, new FilesystemEvent.Deletion(new SyncFilePath(io.GetAbsolutePath(e.FullPath))));
            _watcher.Renamed += (_, e) =>
                callback(state, new FilesystemEvent.Renaming(new SyncFilePath(io.GetAbsolutePath(e.FullPath))));
            _watcher.Error += (_, e) =>
            {
                AnsiConsole.WriteException(e.GetException());
                Environment.Exit(1);
            };

            _watcher.IncludeSubdirectories = true;
            // This starts the watcher (I think)
            _watcher.EnableRaisingEvents = true;
        }

        public void Dispose()
        {
            _watcher.Dispose();
        }
    }
}
