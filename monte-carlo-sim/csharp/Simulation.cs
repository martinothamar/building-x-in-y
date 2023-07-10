using System.Diagnostics;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using System.Runtime.Intrinsics;
using System.Runtime.Intrinsics.X86;
using Fast.PRNGs;

[module: SkipLocalsInit]

unsafe internal struct State : IDisposable
{
    private const nuint Size = (nuint)PoissonOffset + MatchesOffset + ScoresOffset + Simulation.MaxNumberOfMatches * 2;
    private const int PoissonOffset = 0;
    private const int MatchesOffset = PoissonOffset + (sizeof(double) * (Simulation.MaxNumberOfTeams * 2)); // Home and Away poisson per team
    private const int ScoresOffset = PoissonOffset + MatchesOffset + (Simulation.MaxNumberOfMatches * 2);

    internal Shishua Rng;
    internal readonly HashSet<(byte Home, byte Away)> Matchups;

    private readonly void* _ptr;
    private readonly nuint _ptrSize;

    internal readonly int TeamCount;
    internal readonly int MatchCount;
    internal readonly int Simulations;

    private readonly Span<byte> _raw => new(_ptr, (int)_ptrSize);

    internal readonly Span<double> Poisson => new((byte*)_ptr + PoissonOffset, Simulation.MaxNumberOfTeams * 2);
    internal readonly Span<byte> Matches => new((byte*)_ptr + MatchesOffset, Simulation.MaxNumberOfMatches * 2);
    internal readonly Span<byte> Scores => new(((byte*)_ptr) + ScoresOffset, Simulation.MaxNumberOfMatches * 2);

    public State(int simulations, TeamDto[] teams)
    {
        Debug.Assert(teams.Length <= 32, "We store team indice as ID as a byte, and allocate fixed/static buffers");

        Rng = Shishua.Create();
        Matchups = new(Simulation.MaxNumberOfMatches);
        _ptrSize = (nuint)Simulation.NextPow2((int)Size);
        _ptr = NativeMemory.AlignedAlloc(
            _ptrSize,
            (nuint)(1024 * 4) /* 4k page size */
        );
        if (_ptr is null)
            throw new Exception("Couldn't allocate memory");

        // Make sure we touch all memory backed by _ptr, there were a lot of TLB cache misses...
        // This way, all virtual pages should be mapped to pshycal memory immediately
        for (int i = 0; i < (int)_ptrSize; i++)
            *((byte*)_ptr + i) = 0;

        var poisson = Poisson;

        for (int i = 0; i < teams.Length; i++)
        {
            var poissonIndex = i * 2;
            poisson[poissonIndex + 0] = double.Exp(-(teams[i].ExpectedGoals + Simulation.HomeAdvantage));
            poisson[poissonIndex + 1] = double.Exp(-teams[i].ExpectedGoals);
        }

        var matches = Matches;
        var matchIndex = 0;
        for (int i = 0; i < teams.Length; i++)
        {
            for (int j = 0; j < teams.Length; j++)
            {
                if (i == j)
                    continue;

                if (Matchups.Add(((byte)i, (byte)j)))
                {
                    matches[matchIndex + 0] = (byte)i;
                    matches[matchIndex + 1] = (byte)j;
                    matchIndex += 2;
                }
            }
        }

        TeamCount = teams.Length;
        MatchCount = matchIndex;
        Simulations = simulations;
    }

    public void Dispose()
    {
        NativeMemory.AlignedFree(_ptr);
        Rng.Dispose();
    }
}

internal static class Simulation
{
    internal const double HomeAdvantage = 0.25;
    internal const int MaxNumberOfTeams = 32;
    internal const int MaxNumberOfMatches = 32 * 32;

    unsafe public static void Run(ref State state)
    {
        ref var rng = ref state.Rng;
        var scores = state.Scores;
        var matches = state.Matches;
        var poisson = state.Poisson;

        var matchCount = state.MatchCount;

        double* goalsmem = stackalloc double[4];
        var goals = Vector256.Create(0d);

        for (int simulation = 0; simulation < state.Simulations; simulation++)
        {
            for (int i = 0; i < matchCount; i += 4)
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

        scores.Clear();
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

    internal static int NextPow2(int v)
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
