namespace CalculationEngine.Tests;

public class EngineTests
{
    [Fact]
    public void Scalar()
    {
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
        var engine = expression.ToScalarEngine();
        var result = engine.Evaluate(new[] { 1.0, 2.0, 1.0 });
        Assert.Equal(2, result);
    }

    [Fact]
    public void Vectorized()
    {
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
        const int inputSize = 16;
        double[][] input = new[]
        {
            Enumerable.Repeat(1.0, inputSize).ToArray(), // a
            Enumerable.Repeat(2.0, inputSize).ToArray(), // b
            Enumerable.Repeat(1.0, inputSize).ToArray(), // c
        };
        var expectedResult = Enumerable.Repeat(2.0, inputSize).ToArray();
        var engine = expression.ToVectorizedEngine();
        var result = engine.Evaluate(input);
        Assert.Equal(expectedResult, result);
    }
}
