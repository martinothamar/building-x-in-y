using System.Numerics;

namespace CalculationEngine.Benchmarks;

[Config(typeof(Config))]
public class CalcBenchmarks
{
    [Params(512)]
    public int Size { get; set; }

    private double[][] _vectorInput;
    private double[] _scalarInput;
    private Expression _expression;

    [GlobalSetup]
    public void Setup()
    {
        var nodes = new Node[]
        {
            new Operand(),
            Operator.Add,
            new LeftParens(),
            new Operand(),
            Operator.Sub,
            new Operand(),
            new RightParens(),
        };

        _expression = Expression.FromInfix(nodes);

        // 1 + (2 - 1)
        _vectorInput = new[]
        {
            Enumerable.Repeat(1.0, Size).ToArray(), // a
            Enumerable.Repeat(2.0, Size).ToArray(), // b
            Enumerable.Repeat(1.0, Size).ToArray(), // c
        };

        _scalarInput = new[]
        {
            1.0, // a
            2.0, // b
            1.0, // c
        };
    }

    [Benchmark(Baseline = true)]
    public double[] VectorizedBaseline()
    {
        var results = new double[Size];
        var lanes = Vector<double>.Count;
        for (int i = 0; i < Size; i += lanes)
        {
            var a = new Vector<double>(_vectorInput[0], i);
            var b = new Vector<double>(_vectorInput[1], i);
            var c = new Vector<double>(_vectorInput[2], i);
            var result = a + (b - c);
            result.CopyTo(results, i);
        }

        return results;
    }

    [Benchmark]
    public double[] ScalarBaseline()
    {
        var results = new double[Size];
        for (int i = 0; i < Size; i++)
        {
            var a = _vectorInput[0][i];
            var b = _vectorInput[1][i];
            var c = _vectorInput[2][i];
            var result = a + (b - c);
            results[i] = result;
        }

        return results;
    }

    [Benchmark]
    public double[] ScalarEngine()
    {
        var results = new double[Size];
        for (int i = 0; i < Size; i++)
        {
            var result = _expression.Evaluate(_scalarInput);
            results[i] = result;
        }

        return results;
    }

    [Benchmark]
    public double[] VectorizedEngine() => _expression.Evaluate(_vectorInput);

    private class Config : ManualConfig
    {
        public Config()
        {
            this.SummaryStyle = SummaryStyle.Default.WithRatioStyle(RatioStyle.Trend);
            this.AddColumn(RankColumn.Arabic);
            this.Orderer = new DefaultOrderer(SummaryOrderPolicy.FastestToSlowest, MethodOrderPolicy.Declared);
            this.AddDiagnoser(MemoryDiagnoser.Default);
        }
    }
}
