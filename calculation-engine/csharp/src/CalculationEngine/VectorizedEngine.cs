using System.Diagnostics;
using System.Globalization;
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

        const int MaxStackSize = 32;
        var stack = new StackStack<int>(
            input.Length > MaxStackSize ? new int[input.Length] : stackalloc int[MaxStackSize]
        );

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

                int j = 0;
                for (; j < expectedCount && expectedCount - j >= lanes; j += lanes)
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

                ScalarRemainder(j, expectedCount, @operator, leftCol, rightCol, results);

                stack.Push() = input.Length;
            }
        }

        Debug.Assert(stack.Count == 1);
        ref var passResult = ref stack.Pop();
        Debug.Assert(passResult == input.Length);

        return results;
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    private double[] PortableImpl(double[][] input, int expectedCount)
    {
        var results = new double[expectedCount];

        var lanes = Vector<double>.Count;

        const int MaxStackSize = 32;
        var stack = new StackStack<int>(
            input.Length > MaxStackSize ? new int[input.Length] : stackalloc int[MaxStackSize]
        );

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

                int j = 0;
                for (; j < expectedCount && expectedCount - j >= lanes; j += lanes)
                {
                    var l = new Vector<double>(leftCol, j);
                    var r = new Vector<double>(rightCol, j);
                    Unsafe.SkipInit(out Vector<double> result);
                    if (@operator == Operator.Add)
                        result = l + r;
                    else if (@operator == Operator.Sub)
                        result = l - r;
                    else if (@operator == Operator.Mul)
                        result = l * r;
                    else if (@operator == Operator.Div)
                        result = l / r;
                    else
                        ThrowHelper.ThrowArgumentException("Invalid operator");

                    result.CopyTo(results, j);
                }

                ScalarRemainder(j, expectedCount, @operator, leftCol, rightCol, results);

                stack.Push() = input.Length;
            }
        }

        Debug.Assert(stack.Count == 1);
        ref var passResult = ref stack.Pop();
        Debug.Assert(passResult == input.Length);

        return results;
    }

    public double[] Evaluate(double[][] input, bool preferPortable = false)
    {
        if (input.Length != _expression._requiredInputCount)
            ThrowHelper.ThrowArgumentException("Need the same amount of input for all operands");

        var expectedCount = input[0].Length;
        for (int i = 1; i < input.Length; i++)
        {
            if (input[i].Length != expectedCount)
                ThrowHelper.ThrowArgumentException("Need the same amount of input for all operands");
        }

        if (preferPortable)
            return PortableImpl(input, expectedCount);

        if (Avx2.IsSupported)
            return Avx2Impl(input, expectedCount);
        else
            return PortableImpl(input, expectedCount);
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    private static void ScalarRemainder(
        int j,
        int expectedCount,
        Operator @operator,
        double[] leftCol,
        double[] rightCol,
        double[] results
    )
    {
        for (; j < expectedCount; j++)
        {
            var l = leftCol[j];
            var r = rightCol[j];
            Unsafe.SkipInit(out double result);
            if (@operator == Operator.Add)
                result = l + r;
            else if (@operator == Operator.Sub)
                result = l - r;
            else if (@operator == Operator.Mul)
                result = l * r;
            else if (@operator == Operator.Div)
                result = l / r;
            else
                ThrowHelper.ThrowArgumentException("Invalid operator");

            results[j] = result;
        }
    }
}
