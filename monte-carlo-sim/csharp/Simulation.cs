using System.Diagnostics;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using Fast.PRNGs;

[module: SkipLocalsInit]


internal readonly struct Input
{
    public readonly int Simulations;
    public readonly Teams Teams;
    public readonly int TeamCount;

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
    public fixed double PoissonLimit[64];
    public fixed double HomePoissonLimit[64];
}

unsafe internal static class Simulation
{
    internal const double HomeAdvantage = 0.25;

    // [ThreadStatic]
    // private static void* _matchesPtr; // TODO: make sure this is freed at some point

    // [ThreadStatic]
    // private static nuint _matchesPtrLen;

    // [ThreadStatic]
    // private static void* _teamsPtr; // TODO: make sure this is freed at some point

    // [ThreadStatic]
    // private static nuint _teamsPtrLen;

    // private static Span<Match> EnsureMatchesAllocated(int numberOfMatches)
    // {
    //     var matchesPtrLen = (nuint)(sizeof(Match) * numberOfMatches);
    //     if (_matchesPtr is null)
    //     {
    //         _matchesPtrLen = matchesPtrLen;
    //         _matchesPtr = NativeMemory.AlignedAlloc(matchesPtrLen, 64);
    //         if (_matchesPtr is null)
    //             throw new Exception("Couldn't allocate memory");
    //     }
    //     else
    //     {
    //         if (_matchesPtrLen < matchesPtrLen)
    //         {
    //             _matchesPtrLen = matchesPtrLen;
    //             _matchesPtr = NativeMemory.AlignedRealloc(_matchesPtr, matchesPtrLen, 64);
    //             if (_matchesPtr is null)
    //                 throw new Exception("Couldn't allocate memory");
    //         }
    //     }

    //     return new Span<Match>(_matchesPtr, numberOfMatches);
    // }

    // private static Span<Team> EnsureTeamsAllocated(TeamDto[] input)
    // {
    //     var teamsPtrLen = (nuint)(sizeof(Team) * input.Length);
    //     if (_teamsPtr is null)
    //     {
    //         _teamsPtrLen = teamsPtrLen;
    //         _teamsPtr = NativeMemory.AlignedAlloc(teamsPtrLen, 64);
    //         if (_teamsPtr is null)
    //             throw new Exception("Couldn't allocate memory");
    //     }
    //     else
    //     {
    //         if (_teamsPtrLen < teamsPtrLen)
    //         {
    //             _teamsPtrLen = teamsPtrLen;
    //             _teamsPtr = NativeMemory.AlignedRealloc(_teamsPtr, teamsPtrLen, 64);
    //             if (_teamsPtr is null)
    //                 throw new Exception("Couldn't allocate memory");
    //         }
    //     }

    //     return new Span<Team>(_teamsPtr, input.Length);
    // }

    public static void Run(in Input input)
    {
        Debug.Assert(input.TeamCount < 256, "We store team indice as ID as a byte");

        var numberOfMatches = (input.TeamCount - 1) * input.TeamCount;

        var homeRng = Xoshiro256Plus.Create();
        var awayRng = Xoshiro256Plus.Create();

        Span<byte> homeScores = stackalloc byte[NextPow2(numberOfMatches)];
        Span<byte> awayScores = stackalloc byte[NextPow2(numberOfMatches)];

        var homeScoreIndex = 0;
        var awayScoreIndex = 0;

        for (int simulation = 0; simulation < input.Simulations; simulation++)
        {
            for (int i = 0; i < input.TeamCount; i++)
            {
                var home = input.Teams.HomePoissonLimit[i];

                for (int j = 0; j < input.TeamCount; j++)
                {
                    if (i == j)
                        continue;

                    var away = input.Teams.PoissonLimit[j];

                    var homeGoals = Simulate(home, ref homeRng);
                    var awayGoals = Simulate(away, ref awayRng);

                    homeScores[homeScoreIndex++] = homeGoals;
                    awayScores[awayScoreIndex++] = awayGoals;
                }
            }

            homeScoreIndex = 0;
            awayScoreIndex = 0;

            homeScores.Clear();
            awayScores.Clear();
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

    // private static void PopulateTeams(Span<Team> teams, TeamDto[] input)
    // {
    //     for (int i = 0; i < input.Length; i++)
    //         teams[i] = new Team(input[i].ExpectedGoals);
    // }

    // private static void PopulateTeams(ref Teams teams, TeamDto[] input)
    // {
    //     for (int i = 0; i < input.Length; i++)
    //     {
    //         teams.PoissonLimit[i] = double.Exp(-input[i].ExpectedGoals);
    //         teams.HomePoissonLimit[i] = double.Exp(-(input[i].ExpectedGoals + HomeAdvantage));
    //     }
    // }

    // private static void PopulateMatches(Span<Match> matches, Span<Team> teams)
    // {
    //     var matchIndex = 0;
    //     var matchups = new HashSet<(Team Home, Team Away)>();
    //     for (int i = 0; i < teams.Length; i++)
    //     {
    //         ref readonly var home = ref teams[i];

    //         for (int j = 0; j < teams.Length; j++)
    //         {
    //             ref readonly var away = ref teams[j];

    //             if (i == j)
    //                 continue;

    //             if (matchups.Add((home, away)))
    //                 matches[matchIndex++] = new Match((byte)i, (byte)j);
    //         }
    //     }
    // }

    // private readonly record struct Team
    // {
    //     private const double HomeAdvantage = 0.25;

    //     public readonly double PoissonLimit;
    //     public readonly double HomePoissonLimit;

    //     public Team(double expectedGoals)
    //     {
    //         PoissonLimit = double.Exp(-expectedGoals);
    //         HomePoissonLimit = double.Exp(-(expectedGoals + HomeAdvantage));
    //     }
    // }

    // private record struct Match(byte HomeTeam, byte AwayTeam)
    // {
    //     public byte HomeGoals;

    //     public byte AwayGoals;

    //     public readonly bool IsHomeWin => HomeGoals > AwayGoals;

    //     public readonly bool IsDraw => HomeGoals == AwayGoals;

    //     public readonly bool IsAwayWin => AwayGoals > HomeGoals;
    // }

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
