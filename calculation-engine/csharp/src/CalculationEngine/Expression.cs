using System.Diagnostics;
using System.Numerics;

namespace CalculationEngine;

public sealed class Expression
{
    private static readonly int _precmin;
    private static readonly int[] _precedence;

    private readonly int _requiredInputCount;
    private readonly List<Node> _expression;

    private static int Prec(Node n) => n is Operator op ? _precedence[(int)op.Id - _precmin] : -1;

    static Expression()
    {
        var ops = new[] { (Op: '+', P: 1), (Op: '-', P: 1), (Op: '*', P: 2), (Op: '/', P: 2), (Op: '^', P: 3), };
        _precmin = ops.Min(o => (int)o.Op);
        var max = ops.Max(o => (int)o.Op);
        _precedence = new int[max - _precmin + 1];
        for (int i = 0; i < max - _precmin + 1; i++)
        {
            (char Op, int P)? op = null;
            for (int j = 0; j < ops.Length; j++)
            {
                if ((int)ops[j].Op - _precmin == i)
                {
                    op = ops[j];
                    break;
                }
            }

            if (op is var (ch, p))
            {
                _precedence[i] = p;
            }
        }
    }

    private Expression(List<Node> expression, int requiredInputCount)
    {
        _expression = expression;
        _requiredInputCount = requiredInputCount;
    }

    public double Evaluate(double[] input)
    {
        if (input.Length != _requiredInputCount)
            throw new ArgumentException();

        var stack = new Stack<double>();

        var operandIndex = 0;
        for (int i = 0; i < _expression.Count; i++)
        {
            var op = _expression[i];

            if (op is Operand)
            {
                stack.Push(input[operandIndex++]);
            }
            else if (op is Operator @operator)
            {
                var right = stack.Pop();
                var left = stack.Pop();

                double result;
                if (@operator == Operator.Plus)
                    result = left + right;
                else if (@operator == Operator.Minus)
                    result = left - right;
                else if (@operator == Operator.Mul)
                    result = left * right;
                else if (@operator == Operator.Div)
                    result = left / right;
                else
                    throw new ArgumentException("Invalid operator");

                stack.Push(result);
            }
        }

        Debug.Assert(stack.Count == 1);
        return stack.Pop();
    }

    public double[] Evaluate(double[][] input)
    {
        if (input.Length != _requiredInputCount)
            throw new ArgumentException();

        var expectedCount = input[0].Length;
        for (int i = 1; i < input.Length; i++)
        {
            if (input[i].Length != expectedCount)
                throw new ArgumentException("Need the same amount of input for all operands");
        }

        var results = new double[expectedCount];

        var lanes = Vector<double>.Count;
        var passes = expectedCount / lanes;

        var stack = new Stack<Vector<double>>();

        for (int j = 0; j < passes; j++)
        {
            var operandIndex = 0;

            for (int i = 0; i < _expression.Count; i++)
            {
                var op = _expression[i];

                if (op is Operand)
                {
                    stack.Push(new Vector<double>(input[operandIndex++], j * lanes));
                }
                else if (op is Operator @operator)
                {
                    var right = stack.Pop();
                    var left = stack.Pop();

                    Vector<double> result;
                    if (@operator == Operator.Plus)
                        result = left + right;
                    else if (@operator == Operator.Minus)
                        result = left - right;
                    else if (@operator == Operator.Mul)
                        result = left * right;
                    else if (@operator == Operator.Div)
                        result = left / right;
                    else
                        throw new ArgumentException("Invalid operator");

                    stack.Push(result);
                }
            }

            Debug.Assert(stack.Count == 1);
            var passResult = stack.Pop();
            passResult.CopyTo(results, j * lanes);

            stack.Clear();
        }

        return results;
    }

    public static Expression FromInfix(IReadOnlyList<Node> expression)
    {
        if (expression is null || expression.Count == 0)
            throw new ArgumentException("Expression is null or empty");

        var result = new List<Node>(expression.Count);
        var stack = new Stack<Node>();

        for (int i = 0; i < expression.Count; i++)
        {
            var op = expression[i];

            if (op is Operand)
                result.Add(op);
            else if (op is LeftParens)
                stack.Push(op);
            else if (op is RightParens)
            {
                while (stack.Count > 0 && stack.Peek() is not LeftParens)
                    result.Add(stack.Pop());

                if (stack.Count > 0 && stack.Peek() is not LeftParens)
                    throw new ArgumentException("Invalid expression");

                stack.Pop();
            }
            else
            {
                var prec = Prec(op);
                while (stack.Count > 0 && prec <= Prec(stack.Peek()))
                    result.Add(stack.Pop());

                stack.Push(op);
            }
        }

        while (stack.TryPop(out var op))
            result.Add(op);

        return new Expression(result, result.Count(n => n is Operand));
    }
}

public abstract record Node { }

public sealed record Operand : Node { }

public sealed record LeftParens : Node { }

public sealed record RightParens : Node { }

public sealed record Operator : Node
{
    public static readonly Operator Plus = new Operator('+', 2);
    public static readonly Operator Minus = new Operator('-', 2);
    public static readonly Operator Mul = new Operator('*', 2);
    public static readonly Operator Div = new Operator('/', 2);

    private readonly char _id;

    public char Id => _id;

    private readonly byte _operands;

    private Operator(char id, byte operands)
    {
        _id = id;
        _operands = operands;
    }
}
