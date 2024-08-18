using System.Threading.Channels;
using Spectre.Console;
using static Sync.Core.Prelude;

namespace Sync.Client;

internal sealed class Worker
{
    private readonly int _id;
    private readonly Channel<FilesystemEvent> _queue;
    private CancellationToken _cancellationToken;
    private Task? _task;

    // private ulong _msgId;

    internal Task Wait()
    {
        Assert(_task is not null, "");
        return _task;
    }

    internal Worker(int id, CancellationToken cancellationToken)
    {
        _id = id;
        _cancellationToken = cancellationToken;
        _queue = Channel.CreateUnbounded<FilesystemEvent>(
            new UnboundedChannelOptions
            {
                AllowSynchronousContinuations = true,
                SingleWriter = true,
                SingleReader = true,
            }
        );
        _task = null;
        // _msgId = 0;
    }

    internal void Enqueue(FilesystemEvent @event)
    {
        var enqueued = _queue.Writer.TryWrite(@event);
        Assert(enqueued, "Unbuffered channel write");
    }

    internal void Start()
    {
        _task = Task.Factory.StartNew(
            static async (s) =>
            {
                var self = s as Worker;
                Assert(self is not null, "Unexpected state parameter");

                var reader = self._queue.Reader;
                try
                {
                    await foreach (var @event in reader.ReadAllAsync(self._cancellationToken))
                    {
                        var path = @event.Path;
                        var eventName = @event.GetType().Name;
                        AnsiConsole.MarkupLine($"(w={self._id, 2} e=[green]{eventName, 8}[/]) event - [blue]{path}[/]");
                    }
                }
                catch (OperationCanceledException) { }
                catch (Exception ex)
                {
                    AnsiConsole.WriteException(ex);
                    Environment.Exit(1);
                }
            },
            this,
            _cancellationToken,
            TaskCreationOptions.DenyChildAttach | TaskCreationOptions.RunContinuationsAsynchronously,
            TaskScheduler.Default
        );
    }
}
