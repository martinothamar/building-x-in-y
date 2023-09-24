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

This benchmark compares four methods, and how they perform computing the same expression over columns of data.
The baseline methods below are manual vectorized and scalar loops, which will essentially be the best case performance for the tested formula: `a + (b - c)`

The two other methods are doing the same calculation based on expression nodes in postfix order.
Effectively, this measures the overhead of the calculation engine

```

BenchmarkDotNet v0.13.8, Pop!_OS 22.04 LTS
AMD Ryzen 5 5600X, 1 CPU, 12 logical and 6 physical cores
.NET SDK 8.0.100-rc.1.23455.8
  [Host]     : .NET 8.0.0 (8.0.23.41904), X64 RyuJIT AVX2
  DefaultJob : .NET 8.0.0 (8.0.23.41904), X64 RyuJIT AVX2


```
| Method             | Size | Mean        | Error     | StdDev    | Ratio         | RatioSD | Rank | Gen0   | Allocated | Alloc Ratio |
|------------------- |----- |------------:|----------:|----------:|--------------:|--------:|-----:|-------:|----------:|------------:|
| VectorizedBaseline | 512  |    473.7 ns |   2.92 ns |   2.44 ns |      baseline |         |    1 | 0.0486 |   4.02 KB |             |
| ScalarBaseline     | 512  |    815.3 ns |   3.37 ns |   2.81 ns |  1.72x slower |   0.01x |    2 | 0.0486 |   4.02 KB |  1.00x more |
| VectorizedEngine   | 512  |  2,202.6 ns |   6.25 ns |   5.22 ns |  4.65x slower |   0.02x |    3 | 0.0496 |    4.2 KB |  1.04x more |
| ScalarEngine       | 512  | 15,888.9 ns | 108.07 ns | 101.09 ns | 33.49x slower |   0.29x |    4 | 0.5798 |  48.02 KB | 11.94x more |