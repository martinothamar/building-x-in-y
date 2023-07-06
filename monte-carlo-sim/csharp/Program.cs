using System.Diagnostics;
using System.Text.Json;

await using var file = File.OpenRead("../input.json");
var teams = await JsonSerializer.DeserializeAsync(file, AppJsonSerializerContext.Default.TeamDtoArray);

if (teams is null || teams.Length == 0)
    return 1;

const int iterations = 8;
var elapsed = new TimeSpan[iterations];

var timer = new Stopwatch();

for (int i = 0; i < iterations; i++)
{
    timer.Start();
    Simulation.Run(100_000, teams);
    timer.Stop();
    elapsed[i] = timer.Elapsed;
    timer.Reset();
}

for (int i = 0; i < iterations; i++)
    Console.WriteLine($"Elapsed: {elapsed[i].TotalMilliseconds:0.000}ms");

return 0;
