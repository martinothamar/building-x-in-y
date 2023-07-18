using System.Diagnostics;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using System.Runtime.Intrinsics;
using System.Runtime.Intrinsics.X86;
using Fast.PRNGs;

[module: SkipLocalsInit]

internal struct State : IDisposable
{
    internal Shishua Rng;
    internal readonly int Simulations;

    internal readonly double[] Poisson;
    internal readonly byte[] Matches;
    internal readonly byte[] Scores;

    public State(int simulations, TeamDto[] teams)
    {
        Debug.Assert(teams.Length <= 32, "We store team indice as ID as a byte, and allocate fixed/static buffers");

        var numberOFMatches = (teams.Length - 1) * teams.Length;

        Rng = Shishua.Create();
        var matchups = new HashSet<(byte, byte)>(numberOFMatches);

        Poisson = new double[teams.Length * 2];
        Matches = new byte[numberOFMatches * 2];
        Scores = new byte[numberOFMatches * 2];

        for (int i = 0; i < teams.Length; i++)
        {
            var poissonIndex = i * 2;
            Poisson[poissonIndex + 0] = double.Exp(-(teams[i].ExpectedGoals + Simulation.HomeAdvantage));
            Poisson[poissonIndex + 1] = double.Exp(-teams[i].ExpectedGoals);
        }

        var matchIndex = 0;
        for (int i = 0; i < teams.Length; i++)
        {
            for (int j = 0; j < teams.Length; j++)
            {
                if (i == j)
                    continue;

                if (matchups.Add(((byte)i, (byte)j)))
                {
                    Matches[matchIndex + 0] = (byte)i;
                    Matches[matchIndex + 1] = (byte)j;
                    matchIndex += 2;
                }
            }
        }

        Simulations = simulations;
    }

    public void Dispose()
    {
        Rng.Dispose();
    }
}

internal static class Simulation
{
    internal const double HomeAdvantage = 0.25;

    unsafe public static void Run(ref State state)
    {
        ref var rng = ref state.Rng;
        var scores = state.Scores;
        var matches = state.Matches;
        var poisson = state.Poisson;

        double* goalsmem = stackalloc double[4];
        var goals = Vector256.Create(0d);

        for (int simulation = 0; simulation < state.Simulations; simulation++)
        {
            for (int i = 0; i < matches.Length; i += 4)
            {
                var homeId1 = matches[i + 0];
                var awayId1 = matches[i + 1];
                var homePoissonIndex1 = homeId1 * 2;
                var awayPoissonIndex1 = awayId1 * 2;
                var home1 = poisson[homePoissonIndex1 + 0];
                var away1 = poisson[awayPoissonIndex1 + 1];
                Debug.Assert(home1 != 0, "Home poisson limit should not be 0");
                Debug.Assert(away1 != 0, "Away poisson limit should not be 0");

                var homeId2 = matches[i + 2];
                var awayId2 = matches[i + 3];
                var homePoissonIndex2 = homeId2 * 2;
                var awayPoissonIndex2 = awayId2 * 2;
                var home2 = poisson[homePoissonIndex2 + 0];
                var away2 = poisson[awayPoissonIndex2 + 1];
                Debug.Assert(home2 != 0, "Home poisson limit should not be 0");
                Debug.Assert(away2 != 0, "Away poisson limit should not be 0");

                var poissonVec = Vector256.Create(home1, away1, home2, away2);

                goals = default;
                Simulate(poissonVec, ref goals, ref rng);

                Avx2.Store(goalsmem, goals);

                scores[i + 0] = (byte)goalsmem[0];
                scores[i + 1] = (byte)goalsmem[1];
                scores[i + 2] = (byte)goalsmem[2];
                scores[i + 3] = (byte)goalsmem[3];
            }
        }

        System.Array.Clear(scores);
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    private static void Simulate(Vector256<double> poissonVec, ref Vector256<double> goals, ref Shishua rng)
    {
        Vector256<double> productVec = default;
        rng.NextDoubles256(ref productVec);

        while (true)
        {
            // The traditional knuth algo for poisson does '>=' comparisons
            // but to make this SIMD friendly we can do subtraction instead
            var sub = Avx2.Subtract(productVec, poissonVec);
            // MoveMask extracts sign bits from the floats into the lower bits of the mask
            // So if all 4 lower bits are set, we can exit (no goals can be added)
            var mask = Avx2.MoveMask(sub);
            if (mask == 0x000F)
                break;

            // If the product - poisson limit >= 0 we should add the goal
            // Ceiling it will bring negative values to -0 and 0/positive values to 1
            goals = Avx2.Add(goals, Avx2.Ceiling(sub));

            Vector256<double> nextProductVec = default;
            rng.NextDoubles256(ref nextProductVec);
            productVec = Avx2.Multiply(productVec, nextProductVec);
        }

        // NOTE: Original algorithm by Knuth
        // var product = rng.NextDouble();

        // while (product >= poissonLimit)
        // {
        //     goals++;
        //     product *= rng.NextDouble();
        // }
    }
}
