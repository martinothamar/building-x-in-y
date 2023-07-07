using System.Diagnostics;
using System.Runtime.CompilerServices;
using Fast.PRNGs;

[module: SkipLocalsInit]

internal readonly struct Input
{
    public readonly int Simulations;
    public readonly int TeamCount;
    public readonly Teams Teams;

    public Input(int simulations, TeamDto[] teams)
    {
        Simulations = simulations;
        TeamCount = teams.Length;
        unsafe
        {
            for (int i = 0; i < teams.Length; i++)
            {
                Teams.PoissonLimit[i] = double.Exp(-teams[i].ExpectedGoals);
                Teams.HomePoissonLimit[i] = double.Exp(-(teams[i].ExpectedGoals + Simulation.HomeAdvantage));
            }
        }
    }
}

unsafe internal struct Teams
{
    public fixed double PoissonLimit[32];
    public fixed double HomePoissonLimit[32];
}

internal static class Simulation
{
    internal const double HomeAdvantage = 0.25;
    internal const int MaxNumberOfMatches = 32 * 32 * 2;

    unsafe private struct Matches
    {
        public fixed byte Home[MaxNumberOfMatches];
    }

    unsafe public static void Run(in Input input)
    {
        Debug.Assert(input.TeamCount <= 32, "We store team indice as ID as a byte, and allocate fixed/static buffers");

        var numberOfMatches = (input.TeamCount - 1) * input.TeamCount;
        Span<byte> matches = stackalloc byte[MaxNumberOfMatches];
        var matchIndex = 0;
        var matchups = new HashSet<(byte Home, byte Away)>(numberOfMatches);
        for (int i = 0; i < input.TeamCount; i++)
        {
            for (int j = 0; j < input.TeamCount; j++)
            {
                if (i == j)
                    continue;

                if (matchups.Add(((byte)i, (byte)j)))
                {
                    matches[matchIndex] = (byte)i;
                    matches[matchIndex + 1] = (byte)j;
                    matchIndex += 2;
                }
            }
        }

        var homeRng = Xoshiro256Plus.Create();
        var awayRng = Xoshiro256Plus.Create();

        Span<byte> scores = stackalloc byte[MaxNumberOfMatches];

        for (int simulation = 0; simulation < input.Simulations; simulation++)
        {
            for (int i = 0; i < matchIndex; i += 2)
            {
                var homeId = matches[i];
                var awayId = matches[i + 1];
                var home = input.Teams.HomePoissonLimit[homeId];
                var away = input.Teams.PoissonLimit[awayId];

                var homeGoals = Simulate(home, ref homeRng);
                var awayGoals = Simulate(away, ref awayRng);

                scores[i] = homeGoals;
                scores[i + 1] = awayGoals;
            }

            scores.Clear();
        }
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    private static byte Simulate(double poissonLimit, ref Xoshiro256Plus rng)
    {
        // Knuth's poisson algorithm

        byte goals = 0;

        var product = rng.NextDouble();
        while (product >= poissonLimit)
        {
            goals++;
            product *= rng.NextDouble();
        }

        return goals;
    }

    private static int NextPow2(int v)
    {
        v--;
        v |= v >> 1;
        v |= v >> 2;
        v |= v >> 4;
        v |= v >> 8;
        v |= v >> 16;
        v++;

        return v;
    }
}
