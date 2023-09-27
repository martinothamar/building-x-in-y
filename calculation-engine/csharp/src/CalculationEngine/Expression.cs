namespace CalculationEngine;

public sealed class Expression
{
    private static readonly int _precmin;
    private static readonly int[] _precedence;

    internal readonly int _requiredInputCount;
    internal readonly List<Node> _expression;

    private static int Prec(Node n) => n is Operator op ? _precedence[(int)op.Value - _precmin] : -1;

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

    public ScalarEngine ToScalarEngine() => new ScalarEngine(this);

    public VectorizedEngine ToVectorizedEngine() => new VectorizedEngine(this);

    public static Expression FromInfix(IReadOnlyList<Node> expression)
    {
        if (expression is null || expression.Count == 0)
            ThrowHelper.ThrowArgumentException("Invalid expression");

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
                    ThrowHelper.ThrowArgumentException("Invalid expression");

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
    public static readonly Operator Add = new Operator('+');
    public static readonly Operator Sub = new Operator('-');
    public static readonly Operator Mul = new Operator('*');
    public static readonly Operator Div = new Operator('/');

    public char Value { get; }

    private Operator(char value)
    {
        Value = value;
    }
}
