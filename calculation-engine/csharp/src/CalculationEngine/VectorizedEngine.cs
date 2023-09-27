using System.Diagnostics;
using System.Numerics;
using System.Runtime.CompilerServices;
using System.Runtime.Intrinsics;
using System.Runtime.Intrinsics.X86;

namespace CalculationEngine;

public readonly record struct VectorizedEngine
{
    private readonly Expression _expression;

    internal VectorizedEngine(Expression expression)
    {
        _expression = expression;
    }

    private unsafe double[] Avx2Impl(double[][] input, int expectedCount)
    {
        Debug.Assert(Avx2.IsSupported);

        var results = new double[expectedCount];

        var lanes = Vector256<double>.Count;

        var stack = new Stack<Vector256<double>>();

        var expr = _expression._expression;

        for (int j = 0; j < expectedCount; j += lanes)
        {
            var operandIndex = 0;

            for (int i = 0; i < expr.Count; i++)
            {
                var op = expr[i];

                if (op is Operand)
                {
                    var operand = Avx2.LoadVector256((double*)Unsafe.AsPointer(ref input[operandIndex++][j]));
                    stack.Push(operand);
                }
                else if (op is Operator @operator)
                {
                    var right = stack.Pop();
                    var left = stack.Pop();

                    Unsafe.SkipInit(out Vector256<double> result);
                    if (@operator == Operator.Add)
                        result = Avx2.Add(left, right);
                    else if (@operator == Operator.Sub)
                        result = Avx2.Subtract(left, right);
                    else if (@operator == Operator.Mul)
                        result = Avx2.Multiply(left, right);
                    else if (@operator == Operator.Div)
                        result = Avx2.Divide(left, right);
                    else
                        ThrowHelper.ThrowArgumentException("Invalid operator");

                    stack.Push(result);
                }
            }

            Debug.Assert(stack.Count == 1);
            var passResult = stack.Pop();
            passResult.CopyTo(results, j);

            stack.Clear();
        }

        // TODO - handle remainder

        return results;
    }

    private double[] PortableImpl(double[][] input, int expectedCount)
    {
        var results = new double[expectedCount];

        var lanes = Vector<double>.Count;

        var stack = new Stack<Vector<double>>();

        var expr = _expression._expression;

        for (int j = 0; j < expectedCount; j += lanes)
        {
            var operandIndex = 0;

            for (int i = 0; i < expr.Count; i++)
            {
                var op = expr[i];

                if (op is Operand)
                {
                    stack.Push(new Vector<double>(input[operandIndex++], j));
                }
                else if (op is Operator @operator)
                {
                    var right = stack.Pop();
                    var left = stack.Pop();

                    Vector<double> result;
                    if (@operator == Operator.Add)
                        result = left + right;
                    else if (@operator == Operator.Sub)
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
            passResult.CopyTo(results, j);

            stack.Clear();
        }

        // TODO - handle remainder

        return results;
    }

    public double[] Evaluate(double[][] input)
    {
        if (input.Length != _expression._requiredInputCount)
            throw new ArgumentException();

        var expectedCount = input[0].Length;
        for (int i = 1; i < input.Length; i++)
        {
            if (input[i].Length != expectedCount)
                throw new ArgumentException("Need the same amount of input for all operands");
        }

        if (Avx2.IsSupported)
        {
            return Avx2Impl(input, expectedCount);
        }
        else
        {
            return PortableImpl(input, expectedCount);
        }
    }
}
