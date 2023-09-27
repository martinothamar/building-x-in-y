using System.Diagnostics;
using System.Runtime.CompilerServices;

namespace CalculationEngine;

public readonly record struct ScalarEngine
{
    private readonly Expression _expression;

    internal ScalarEngine(Expression expression)
    {
        _expression = expression;
    }

    public double Evaluate(double[] input)
    {
        if (input.Length != _expression._requiredInputCount)
            throw new ArgumentException();

        var stack = new Stack<double>();

        var expr = _expression._expression;

        var operandIndex = 0;
        for (int i = 0; i < expr.Count; i++)
        {
            var op = expr[i];

            if (op is Operand)
            {
                stack.Push(input[operandIndex++]);
            }
            else if (op is Operator @operator)
            {
                var right = stack.Pop();
                var left = stack.Pop();

                Unsafe.SkipInit(out double result);
                if (@operator == Operator.Add)
                    result = left + right;
                else if (@operator == Operator.Sub)
                    result = left - right;
                else if (@operator == Operator.Mul)
                    result = left * right;
                else if (@operator == Operator.Div)
                    result = left / right;
                else
                    ThrowHelper.ThrowArgumentException("Invalid operator");

                stack.Push(result);
            }
        }

        Debug.Assert(stack.Count == 1);
        return stack.Pop();
    }
}
