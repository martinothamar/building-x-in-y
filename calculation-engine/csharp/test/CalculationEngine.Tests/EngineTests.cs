namespace CalculationEngine.Tests;

public class EngineTests
{
    private static readonly Node[] _simpleExpression = new Node[]
    {
        new Operand(),
        Operator.Add,
        new LeftParens(),
        new Operand(),
        Operator.Sub,
        new Operand(),
        new RightParens(),
    };

    [Fact]
    public void Scalar()
    {
        // a + (b - c)
        var nodes = _simpleExpression;

        var expression = Expression.FromInfix(nodes);
        Assert.NotNull(expression);

        // 1 + (2 - 1)
        var engine = expression.ToScalarEngine();
        var result = engine.Evaluate(new[] { 1.0, 2.0, 1.0 });
        Assert.Equal(2, result);
    }

    [Theory]
    [InlineData(15)]
    [InlineData(16)]
    [InlineData(17)]
    [InlineData(18)]
    [InlineData(19)]
    [InlineData(20)]
    [InlineData(63)]
    public void VectorizedOptimal(int inputSize)
    {
        // a + (b - c)
        var nodes = _simpleExpression;

        var expression = Expression.FromInfix(nodes);
        Assert.NotNull(expression);

        // 1 + (2 - 1)
        double[][] input = new[]
        {
            Enumerable.Range(0, inputSize).Select(i => 1.0 + i).ToArray(), // a
            Enumerable.Repeat(2.0, inputSize).ToArray(), // b
            Enumerable.Repeat(1.0, inputSize).ToArray(), // c
        };
        var expectedResult = Enumerable.Range(0, inputSize).Select(i => 2.0 + i).ToArray();
        var engine = expression.ToVectorizedEngine();
        var result = engine.Evaluate(input);
        Assert.Equal(expectedResult, result);
    }

    [Theory]
    [InlineData(15)]
    [InlineData(16)]
    [InlineData(17)]
    [InlineData(18)]
    [InlineData(19)]
    [InlineData(20)]
    [InlineData(63)]
    public void VectorizedPortable(int inputSize)
    {
        // a + (b - c)
        var nodes = _simpleExpression;

        var expression = Expression.FromInfix(nodes);
        Assert.NotNull(expression);

        // 1 + (2 - 1)
        double[][] input = new[]
        {
            Enumerable.Range(0, inputSize).Select(i => 1.0 + i).ToArray(), // a
            Enumerable.Repeat(2.0, inputSize).ToArray(), // b
            Enumerable.Repeat(1.0, inputSize).ToArray(), // c
        };
        var expectedResult = Enumerable.Range(0, inputSize).Select(i => 2.0 + i).ToArray();
        var engine = expression.ToVectorizedEngine();
        var result = engine.Evaluate(input, preferPortable: true);
        Assert.Equal(expectedResult, result);
    }
}
