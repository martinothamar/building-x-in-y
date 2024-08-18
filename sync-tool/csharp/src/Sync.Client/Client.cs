using Spectre.Console;
using static Sync.Core.Prelude;

namespace Sync.Client;

public static class Client<TIO>
    where TIO : class, IIO, new()
{
    public static async Task<int> Run(
        string dir,
        string host = "localhost",
        int port = 8050,
        CancellationToken cancellationToken = default
    )
    {
        var io = new TIO();
        var clock = TimeProvider.System;

        if (!io.DirectoryExists(dir))
            return 1;

        Assert(BitConverter.IsLittleEndian, "Protocol only supports little-endian");

        AnsiConsole.MarkupLine($"Will sync from: [blue]{dir}[/]");

        var workers = new WorkerPool<TIO>(dir, host, port, clock, io, cancellationToken);

        // Start the workers so that we can
        // 1. Start getting watcher events
        // 2. Initialize from current state of `dir`
        // Workers will have to deal with the possibility of receiving
        // change events before having received initialization events.
        // If the order was the other way around, we might miss changes instead.
        workers.Start();

        var timerTask = Task
            .Factory.StartNew(
                async _ =>
                {
                    using var timer = new PeriodicTimer(TimeSpan.FromSeconds(1), clock);

                    try
                    {
                        workers.Enqueue(new SyncEvent.Tick());
                        while (await timer.WaitForNextTickAsync(cancellationToken))
                        {
                            workers.Enqueue(new SyncEvent.Tick());
                        }
                    }
                    catch (Exception ex) when (ex is not OperationCanceledException)
                    {
                        AnsiConsole.MarkupLine("Timer thread crashed:");
                        AnsiConsole.WriteException(ex);
                    }
                },
                null,
                cancellationToken,
                TaskCreationOptions.DenyChildAttach | TaskCreationOptions.RunContinuationsAsynchronously,
                TaskScheduler.Default
            )
            .Unwrap();

        using var watcher = io.Watch(
            dir,
            workers,
            static (workers, @event) => workers.Enqueue(new SyncEvent.Fs(@event))
        );

        // Make sure we stop watching for events when user does Ctrl-C
        cancellationToken.Register(
            static state =>
            {
                AnsiConsole.MarkupLine("[yellow]Cancelled[/], exiting...");
                var watcher = state as IFsWatcher;
                Assert(watcher is not null, "We pass watcher as state");
                watcher.Dispose(); // FileSystemWatcher tolerates being disposed twice
            },
            watcher
        );

        // Now we can send initialization events for all files in the `dir` tree
        foreach (var path in io.GetFiles(dir))
        {
            var @event = new FilesystemEvent.Init(path);
            workers.Enqueue(new SyncEvent.Fs(@event));
        }

        await timerTask;
        await workers.Wait();

        return 0;
    }
}
