using System.Threading.Channels;
using Spectre.Console;
using Sync.Core;
using static Sync.Core.Prelude;

namespace Sync.Client;

internal sealed class WorkerPool<TIO>(
    string dir,
    string host,
    int port,
    TimeProvider clock,
    TIO io,
    CancellationToken cancellationToken
)
    where TIO : class, IIO, new()
{
    public readonly string Dir = dir;
    private readonly string _host = host;
    private readonly int _port = port;
    private readonly TimeProvider _clock = clock;
    private readonly TIO _io = io;
    private readonly CancellationToken _cancellationToken = cancellationToken;
    internal readonly int WorkerCount = Math.Max(Environment.ProcessorCount / 2, 1);
    private Worker[]? _workers;
    private Session? _session;
    private Task? _task;
    private Channel<SyncEvent>? _queue;
    private IIoSocket? _serverConnection;
    private ulong _msgId;

    internal ref Worker GetWorker(int id)
    {
        Assert(_workers is not null, "WorkerPool should be initialized before fetching a worker");
        return ref _workers[id];
    }

    private int GetWorkerId(SyncFilePath path)
    {
        var id = path.GetUnsignedHashCode() % (uint)WorkerCount;
        return (int)id;
    }

    internal void Enqueue(SyncEvent @event)
    {
        Assert(_queue is not null, "WorkerPool must be initialized");
        var enqueued = _queue.Writer.TryWrite(@event);
        Assert(enqueued, "WorkerPool queue should be unbounded");
    }

    internal void Start()
    {
        Assert(_session is null, "Session should not be initialized when starting worker pool");
        Assert(_task is null, "WorkerPool task should not be started");
        Assert(WorkerCount is > 0 and <= byte.MaxValue, "Worker count should fit within a byte");
        var partitionCount = (byte)WorkerCount;
        Assert(partitionCount > 0, "Partition count should be a non-zero byte");
        _session = Session.Create(partitionCount, _clock);

        _workers = GC.AllocateUninitializedArray<Worker>(WorkerCount, pinned: true);
        for (int id = 0; id < WorkerCount; id++)
        {
            _workers[id] = new Worker(id, _cancellationToken);
            _workers[id].Start();
        }

        _queue = Channel.CreateUnbounded<SyncEvent>(
            new UnboundedChannelOptions
            {
                AllowSynchronousContinuations = true,
                SingleWriter = false,
                SingleReader = true,
            }
        );
        _task = Task
            .Factory.StartNew(
                static s =>
                {
                    var self = s as WorkerPool<TIO>;
                    Assert(self is not null, "Unexpected state parameter");
                    return self.RunThread();
                },
                this,
                _cancellationToken,
                TaskCreationOptions.DenyChildAttach | TaskCreationOptions.RunContinuationsAsynchronously,
                TaskScheduler.Default
            )
            .Unwrap();
    }

    private async Task RunThread()
    {
        // Runs the WorkerPool thread, which coordinates messages and handles session-level messages.
        // CPU/thread topology could be improved by pulling session-level messages into a separate thread.
        try
        {
            Assert(_session is not null && _queue is not null, "WorkerPool must be initialized");
            var resequenceQueue = new Queue<SyncEvent>(8);
            await foreach (var @event in _queue.Reader.ReadAllAsync(_cancellationToken))
            {
                HandleEvent(@event, resequenceQueue);

                // If any action event has lead to the session now being active,
                // we can immediately process all the queued events that arrived
                // before session init completed.
                if (_session.State >= SessionState.Running && resequenceQueue.Count > 0)
                {
                    while (resequenceQueue.Count > 0)
                    {
                        var queuedEvent = resequenceQueue.Dequeue();
                        HandleEvent(@queuedEvent, resequenceQueue);
                    }
                }
            }
        }
        catch (Exception ex) when (ex is not OperationCanceledException)
        {
            AnsiConsole.WriteException(ex);
            Environment.Exit(1);
        }
    }

    private void HandleEvent(SyncEvent @event, Queue<SyncEvent> resequenceQueue)
    {
        Assert(_session is not null, "WorkerPool must be initialized");

        switch (@event)
        {
            case SyncEvent.Fs { Event: var fsEvent }:
                {
                    Assert(_workers is not null, "WorkerPool must be initialized");
                    if (_session.State < SessionState.Running)
                    {
                        // Session hasn't been started yet, so just queue it up and hopefully we will get there soon
                        resequenceQueue.Enqueue(@event);
                    }
                    else
                    {
                        var id = GetWorkerId(fsEvent.Path);
                        var worker = GetWorker(id);
                        worker.Enqueue(fsEvent);
                    }
                }
                break;
            case SyncEvent.Tick:
                {
                    var action = _session.Tick();
                    if (action != SessionAction.None)
                        Enqueue(new SyncEvent.SessionActionCommand(action));
                }
                break;
            case SyncEvent.SessionActionCommand { Action: var action }:
                {
                    switch (action)
                    {
                        case SessionAction.Start:
                            SendSessionStart();
                            break;
                        default:
                            Assert(false, "Unknown session action");
                            break;
                    }
                }
                break;
            case SyncEvent.SessionActionCompleted { Action: var action }:
                {
                    switch (action)
                    {
                        case SessionAction.Start:
                            _session.StartAcked();
                            break;
                        default:
                            Assert(false, "Unknown session action");
                            break;
                    }
                }
                break;
        }
    }

    private void SendSessionStart()
    {
        var task = Task
            .Factory.StartNew(
                static async s =>
                {
                    var self = s as WorkerPool<TIO>;
                    Assert(self is not null, "Unexpected state parameter");
                    Assert(self._session is not null, "WorkerPool must be initialized");
                    Assert(self._serverConnection is null, "Server connection is initialized on session start");
                    var serverConnection = self._serverConnection = await self._io.Connect(
                        self._host,
                        self._port,
                        self._cancellationToken
                    );

                    // TODO: need a better (off-heap) allocator
                    byte[] buffer = new byte[Protocol.SizeFor<SessionPackage>()];
                    var header = new PackageHeader
                    {
                        PartitionId = ulong.MaxValue,
                        MessageId = self._msgId++,
                        PackageType = (byte)PackageType.Session,
                    };

                    var package = new SessionPackage { Id = self._session.Id, Partitions = (byte)self.WorkerCount, };

                    Protocol.Serialize(buffer, ref header, ref package);

                    await serverConnection.Send(buffer, self._cancellationToken);
                },
                this,
                _cancellationToken,
                TaskCreationOptions.DenyChildAttach | TaskCreationOptions.RunContinuationsAsynchronously,
                TaskScheduler.Default
            )
            .Unwrap();

        task.ContinueWith(
            static (_, s) =>
            {
                var self = s as WorkerPool<TIO>;
                Assert(self is not null, "Unexpected state parameter");
                self.Enqueue(new SyncEvent.SessionActionCompleted(SessionAction.Start));
            },
            this
        );
    }

    internal Task Wait()
    {
        Assert(_workers is not null, "WorkerPool should be initialized before awaiting workers");
        return Task.WhenAll(_workers.Select(w => w.Wait()));
    }
}
