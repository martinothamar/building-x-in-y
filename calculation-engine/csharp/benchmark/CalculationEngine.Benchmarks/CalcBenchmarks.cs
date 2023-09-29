using System.Numerics;
using System.Runtime.CompilerServices;
using System.Runtime.Intrinsics.X86;
using Microsoft.Data.Analysis;
using System.Linq;

namespace CalculationEngine.Benchmarks;

[Config(typeof(Config))]
public class CalcBenchmarks
{
    [Params(8192)]
    public int Size { get; set; }

    private double[][] _vectorInput;
    private double[] _scalarInput;
    private DoubleDataFrameColumn _dataFrameA;
    private DoubleDataFrameColumn _dataFrameB;
    private DoubleDataFrameColumn _dataFrameC;
    private DoubleDataFrameColumn _dataFrameResult;
    private DataFrame _dataFrame;
    private Expression _expression;
    private ScalarEngine _scalarEngine;
    private VectorizedEngine _vectorizedEngine;
    private double[] _results;

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
        _scalarEngine = _expression.ToScalarEngine();
        _vectorizedEngine = _expression.ToVectorizedEngine();

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

        _dataFrameA = new DoubleDataFrameColumn("a", _vectorInput[0]);
        _dataFrameB = new DoubleDataFrameColumn("b", _vectorInput[1]);
        _dataFrameC = new DoubleDataFrameColumn("c", _vectorInput[2]);
        _dataFrameResult = new DoubleDataFrameColumn("result", new double[Size]);
        _dataFrame = new DataFrame(_dataFrameA, _dataFrameB, _dataFrameC);

        _results = new double[Size];
    }

    [Benchmark(Baseline = true)]
    public double[] ManualVectorizedBaseline()
    {
        var results = _results;
        unsafe
        {
            for (int i = 0; i < Size; i += 4)
            {
                var a = Avx2.LoadVector256((double*)Unsafe.AsPointer(ref _vectorInput[0][i]));
                var b = Avx2.LoadVector256((double*)Unsafe.AsPointer(ref _vectorInput[1][i]));
                var c = Avx2.LoadVector256((double*)Unsafe.AsPointer(ref _vectorInput[2][i]));

                var result = Avx2.Add(a, Avx2.Subtract(b, c));
                Avx2.Store((double*)Unsafe.AsPointer(ref results[i]), result);
            }
        }

        return results;
    }

    [Benchmark]
    public double[] PortableVectorizedBaseline()
    {
        var results = _results;
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
    public IReadOnlyList<double?> DataFrame()
    {
        // Dont know if this copy is efficient but... found no other way
        var results = _dataFrameResult;
        for (int i = 0; i < results.Length; i++)
            results[i] = _dataFrameB[i];

        results.Subtract(_dataFrameC, inPlace: true);
        results.Add(_dataFrameA, inPlace: true);
        return results[0, Size];
    }

    [Benchmark]
    public double[] ScalarBaseline()
    {
        var results = _results;
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
        var engine = _scalarEngine;
        var results = _results;
        for (int i = 0; i < Size; i++)
        {
            var result = engine.Evaluate(_scalarInput);
            results[i] = result;
        }

        return results;
    }

    [Benchmark]
    public double[] VectorizedEngine()
    {
        var results = _results;
        _vectorizedEngine.Evaluate(_vectorInput, results);
        return results;
    }

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
