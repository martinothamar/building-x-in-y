using System.Net;
using System.Net.Sockets;
using System.Runtime.InteropServices;
using System.Text;

if (!RuntimeInformation.IsOSPlatform(OSPlatform.Linux))
{
    Console.Error.WriteLine("This program only runs on Linux.");
    return 1;
}

using var cts = new CancellationTokenSource();
var cancellationToken = cts.Token;

AppDomain.CurrentDomain.ProcessExit += (sender, eventArgs) =>
{
    cts.Cancel();
};

Console.CancelKeyPress += (sender, eventArgs) =>
{
    cts.Cancel();
};

var port = 8080;
var localAddr = IPAddress.Parse("127.0.0.1");

var server = new Socket(localAddr.AddressFamily, SocketType.Stream, ProtocolType.Tcp);
server.Bind(new IPEndPoint(localAddr, port));
server.Listen(128);

var queue = new Spmc<Socket>();

var cpus = Environment.ProcessorCount;
var threads = new Thread[cpus];
for (int i = 0; i < cpus; i++)
{
    threads[i] = new Thread(() => RunThread(queue, cancellationToken));
    threads[i].Start();
}

Console.Out.WriteLine($"INFO: listening on {localAddr}:{port}");

while (!cancellationToken.IsCancellationRequested)
{
    var socket = server.Accept();
    queue.Enqueue(socket);
}

foreach (var thread in threads)
    thread.Join();

return 0;

static void RunThread(Spmc<Socket> queue, CancellationToken cancellationToken)
{
    try
    {
        var buffer = new byte[1024 * 64];

        var response =
            "HTTP/1.1 200 OK\r\n"u8
            + "Content-Length: 13\r\n"u8
            + "Content-Type: text/plain\r\n"u8
            + "\r\n"u8
            + "Hello, World!"u8;

        while (!cancellationToken.IsCancellationRequested)
        {
            using var socket = queue.Dequeue();
            if (socket == null)
                continue; // Lost race

            // Console.Out.WriteLine($"DEBUG({socket.RemoteEndPoint}): opened connection");

            int bytesReceived = 0;
            while (true)
            {
                var read = socket.Receive(buffer, bytesReceived, buffer.Length - bytesReceived, SocketFlags.None);
                if (read == 0)
                    break;

                bytesReceived += read;

                if (read >= 4 && buffer.AsSpan(bytesReceived - 4, 4).SequenceEqual("\r\n\r\n"u8))
                {
                    // var req = Encoding.UTF8.GetString(buffer, 0, bytesReceived).Replace("\r\n", "\\r\\n");
                    // Console.Out.WriteLine($"DEBUG({socket.RemoteEndPoint}): received request: {req}");
                    socket.Send(response, SocketFlags.None);
                    bytesReceived = 0;
                }
                else
                {
                    // Console.Out.WriteLine(
                    //     $"DEBUG({socket.RemoteEndPoint}): received partial request: {Encoding.UTF8.GetString(buffer, bytesReceived, read)}"
                    // );
                }
            }

            // Console.Out.WriteLine($"DEBUG({socket.RemoteEndPoint}): closed connection");
            socket.Shutdown(SocketShutdown.Both);
            socket.Close();
        }
    }
    catch (Exception ex)
    {
        Console.Error.WriteLine($"ERROR: thread crashed: {ex}");
    }
}

class Spmc<T>
{
    const int _capacity = 1024;
    private readonly T[] _buffer;
    private readonly AutoResetEvent _sync;
    private int _head;
    private int _tail;

    public Spmc()
    {
        _buffer = new T[_capacity];
        _head = 0;
        _tail = 0;
        _sync = new AutoResetEvent(false);
    }

    public void Enqueue(T item)
    {
        var head = Thread.VolatileRead(ref _head);
        var wasEmpty = _tail == head;
        if ((_tail + 1) % _capacity == head)
            throw new Exception("queue is full");

        _buffer[_tail] = item;
        _tail = (_tail + 1) % _capacity;
        _sync.Set();
        Console.Out.WriteLine("DEBUG: enqueued");
    }

    public T? Dequeue()
    {
        var head = Thread.VolatileRead(ref _head);
        var isEmpty = head == _tail;
        if (isEmpty)
        {
            Console.Out.WriteLine("DEBUG: waiting..");
            _sync.WaitOne();
            Console.Out.WriteLine("DEBUG: trying to dequeue");
        }

        var item = _buffer[head];
        var success = Interlocked.CompareExchange(ref _head, (head + 1) % _capacity, head) == head;
        if (success)
            Console.Out.WriteLine("DEBUG: dequeued");
        return success ? item : default;
    }
}
