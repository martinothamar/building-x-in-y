# Calculation engine

I don't know if this is the right word, but I went looking for a calculation engine by these criteria

* Exposes a sort of builder API for building (calculation) expressions
* Vectorized operations for computing the same expression over arrays/columns of operand values

I could not find one. So I started building one here, currently in C# .NET.

It supports an API like this:

```csharp
// a + (b - c)
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

var expression = Expression.FromInfix(nodes);
Assert.NotNull(expression);

// 1 + (2 - 1)
// done 16 times
const int inputSize = 16;
double[][] input = new[]
{
    Enumerable.Repeat(1.0, inputSize).ToArray(), // a
    Enumerable.Repeat(2.0, inputSize).ToArray(), // b
    Enumerable.Repeat(1.0, inputSize).ToArray(), // c
};
var expectedResult = Enumerable.Repeat(2.0, inputSize).ToArray();
var result = expression.Evaluate(input);
Assert.Equal(expectedResult, result);
```

Example use-case would be creating aggregates/calculations based on sensor timeseries.
See the `csharp` folder for implementation.

## Performance

I have not spent a lot of time optimizing this, haven't even looked at the disassembly. But basic benchmark results are below.

This benchmark compares several methods, and how they perform computing the same expression over columns of data.
The baseline methods below are manual/portable vectorized and scalar loops, which will essentially be the best case performance for the tested formula: `a + (b - c)`

The two other methods are doing the same calculation based on expression nodes in postfix order.
Effectively, this measures the overhead of the calculation engine

P.S: the `Microsoft.Data.Analysis.DataFrame` stuff is performing very poorly

```

BenchmarkDotNet v0.13.8, Pop!_OS 22.04 LTS
AMD Ryzen 5 5600X, 1 CPU, 12 logical and 6 physical cores
.NET SDK 8.0.100-rc.1.23455.8
  [Host]     : .NET 8.0.0 (8.0.23.41904), X64 RyuJIT AVX2
  DefaultJob : .NET 8.0.0 (8.0.23.41904), X64 RyuJIT AVX2


```
| Method                     | Size | Mean       | Error     | StdDev    | Ratio         | RatioSD | Rank | Gen0    | Gen1    | Gen2    | Allocated | Alloc Ratio |
|--------------------------- |----- |-----------:|----------:|----------:|--------------:|--------:|-----:|--------:|--------:|--------:|----------:|------------:|
| ManualVectorizedBaseline   | 8192 |   6.768 μs | 0.0185 μs | 0.0155 μs |      baseline |         |    1 |  0.7782 |       - |       - |  64.02 KB |             |
| PortableVectorizedBaseline | 8192 |   7.597 μs | 0.0319 μs | 0.0298 μs |  1.12x slower |   0.01x |    2 |  0.7782 |       - |       - |  64.02 KB |  1.00x more |
| ScalarBaseline             | 8192 |  12.789 μs | 0.0386 μs | 0.0322 μs |  1.89x slower |   0.01x |    3 |  0.7782 |       - |       - |  64.02 KB |  1.00x more |
| VectorizedEngine           | 8192 |  26.993 μs | 0.0544 μs | 0.0483 μs |  3.99x slower |   0.01x |    4 |  0.7629 |       - |       - |  64.02 KB |  1.00x more |
| DataFrame                  | 8192 | 147.448 μs | 0.0798 μs | 0.0707 μs | 21.79x slower |   0.06x |    5 | 41.5039 | 41.5039 | 41.5039 | 261.89 KB |  4.09x more |
| ScalarEngine               | 8192 | 296.051 μs | 1.5972 μs | 1.4940 μs | 43.76x slower |   0.30x |    6 |  9.2773 |  1.9531 |       - | 768.02 KB | 12.00x more |


