using System.Diagnostics;
using System.Runtime.CompilerServices;
using System.Runtime.Intrinsics;
using System.Runtime.Intrinsics.X86;
using System.Text.Json;
using System.Xml.XPath;
using Fast.PRNGs;

Console.WriteLine("Starting simulation");

// {
//     var poissonVec = Vector256.Create(0.5d);
//     Vector256<double> productVec = Vector256.Create(0.4, 0.4, 0.9, 0.9);

//     Vector256<double> goals = default;

//     var adder = Vector256.Create(1d);
//     while (true)
//     {
//         var condition = Avx2.CompareGreaterThanOrEqual(productVec, poissonVec);
//         goals = Avx2.Add(goals, Avx2.And(adder, condition));
//         productVec = Avx2.Multiply(productVec, Vector256.Create(0.7d));

//         var sub = Avx.Subtract(productVec, poissonVec);
//         var mask = Avx2.MoveMask(sub);
//         if (mask == 0x000F)
//             break;
//     }
// }


// {
//     var poissonVec = 0.5d;
//     var goals = 0d;
//     var productVec = 0.8d;
//     var adder = 1d;
//     var firstResult = goals + adder;
//     var condition = poissonVec;
//     condition =
//         productVec >= condition
//             ? BitConverter.UInt64BitsToDouble(0xFFFFFFFFFFFFFFFF)
//             : BitConverter.UInt64BitsToDouble(0x0000000000000000);
//     firstResult = BitConverter.DoubleToInt64Bits(firstResult) & BitConverter.DoubleToInt64Bits(condition);
//     Console.WriteLine(firstResult.ToString());
// }

await using var file = File.OpenRead("../input.json");
var teams = await JsonSerializer.DeserializeAsync(file, AppJsonSerializerContext.Default.TeamDtoArray);

if (teams is null || teams.Length == 0)
    return 1;

Console.WriteLine($"Loaded {teams.Length} teams");

Run(teams);

return 0;

static void Run(TeamDto[] teams)
{
    const int iterations = 16;
    Span<TimeSpan> elapsed = stackalloc TimeSpan[iterations];

    var state = new State(100_000, teams);
    try
    {
        for (int i = 0; i < iterations; i++)
        {
            var start = Stopwatch.GetTimestamp();
            Simulation.Run(ref state);
            var stop = Stopwatch.GetTimestamp();
            var duration = Stopwatch.GetElapsedTime(start, stop);
            elapsed[i] = duration;
            // state.Reset();
        }
    }
    finally
    {
        state.Dispose();
    }

    for (int i = 0; i < iterations; i++)
        Console.WriteLine($"Elapsed: {elapsed[i].TotalMilliseconds:0.000}ms");
}
