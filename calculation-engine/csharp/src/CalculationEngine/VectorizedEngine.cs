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

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    private unsafe double[] Avx2Impl(double[][] input, int expectedCount)
    {
        Debug.Assert(Avx2.IsSupported);

        var results = new double[expectedCount];

        var lanes = Vector256<double>.Count;

        var stack = new StackStack<int>(stackalloc int[8]); // TODO hehe

        var expr = _expression._expression;

        var operandIndex = 0;

        for (int i = 0; i < expr.Count; i++)
        {
            var op = expr[i];

            if (op is Operand)
            {
                stack.Push() = operandIndex++;
            }
            else if (op is Operator @operator)
            {
                ref var right = ref stack.Pop();
                ref var left = ref stack.Pop();

                var leftCol = left < input.Length ? input[left] : results;
                var rightCol = right < input.Length ? input[right] : results;

                for (int j = 0; j < expectedCount; j += lanes)
                {
                    var l = Avx2.LoadVector256((double*)Unsafe.AsPointer(ref leftCol[j]));
                    var r = Avx2.LoadVector256((double*)Unsafe.AsPointer(ref rightCol[j]));
                    Unsafe.SkipInit(out Vector256<double> result);
                    if (@operator == Operator.Add)
                        result = Avx2.Add(l, r);
                    else if (@operator == Operator.Sub)
                        result = Avx2.Subtract(l, r);
                    else if (@operator == Operator.Mul)
                        result = Avx2.Multiply(l, r);
                    else if (@operator == Operator.Div)
                        result = Avx2.Divide(l, r);
                    else
                        ThrowHelper.ThrowArgumentException("Invalid operator");

                    Avx2.Store((double*)Unsafe.AsPointer(ref results[j]), result);
                }

                stack.Push() = input.Length;
            }
        }

        Debug.Assert(stack.Count == 1);
        ref var passResult = ref stack.Pop();
        Debug.Assert(passResult == input.Length);

        // TODO - handle remainder

        return results;
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
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
            ThrowHelper.ThrowArgumentException("Need the same amount of input for all operands");

        var expectedCount = input[0].Length;
        for (int i = 1; i < input.Length; i++)
        {
            if (input[i].Length != expectedCount)
                ThrowHelper.ThrowArgumentException("Need the same amount of input for all operands");
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
