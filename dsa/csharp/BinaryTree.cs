using System.Diagnostics;
using System.Runtime.ExceptionServices;
using System.Security.Cryptography;
using System.Text;

namespace csharp;

public static class ReferenceBased
{
    public sealed class BinaryTree<T>
        where T : IComparable<T>, IEquatable<T>
    {
        private Node<T>? _root;

        public BinaryTree() { }

        public void Insert(T value)
        {
            if (_root is null)
            {
                _root = new Node<T>(value);
                return;
            }

            var queue = new Queue<Node<T>>(1);
            queue.Enqueue(_root);

            while (queue.Count > 0)
            {
                var node = queue.Dequeue();

                if (node._left is null)
                {
                    node._left = new Node<T>(value);
                    return;
                }
                else
                {
                    queue.Enqueue(node._left);
                }

                if (node._right is null)
                {
                    node._right = new Node<T>(value);
                    return;
                }
                else
                {
                    queue.Enqueue(node._right);
                }
            }
        }
    }

    public sealed class Node<T>
    {
        private T _value;
        internal Node<T>? _left;
        internal Node<T>? _right;

        internal Node(T value)
        {
            _value = value;
        }
    }
}

public static class ArrayBased
{
    public sealed class BinaryTree<T>
        where T : IComparable<T>, IEquatable<T>
    {
        private Node<T>?[] _data;
        public int Count { get; private set; }

        public BinaryTree()
        {
            _data = new Node<T>?[3];
        }

        public void Insert(T value)
        {
            if (_data[0] is null)
            {
                _data[0] = new Node<T>(value);
                Count++;
                return;
            }

            var queue = new Queue<int>();
            queue.Enqueue(0);

            var depth = 0;

            while (queue.Count > 0)
            {
                var i = queue.Dequeue();
                var node = _data[i];
                Debug.Assert(node is not null);

                var leftI = (2 * i) + 1;
                if (_data.Length <= leftI)
                {
                    var newData = new Node<T>?[_data.Length + (depth * 2 * 3)];
                    Array.Copy(_data, newData, _data.Length);
                    _data = newData;
                }

                ref var left = ref _data[leftI];
                if (left is null)
                {
                    left = new Node<T>(value);
                    Count++;
                    return;
                }

                var rightI = (2 * i) + 2;
                ref var right = ref _data[rightI];
                if (right is null)
                {
                    right = new Node<T>(value);
                    Count++;
                    return;
                }

                queue.Enqueue(leftI);
                queue.Enqueue(rightI);

                depth++;
            }
        }

        public IEnumerable<T> InOrder()
        {
            var p = 0;
            var s = new Stack<int>();

            var data = _data;
            while (data[p] is not null || s.Count > 0)
            {
                while (data[p] is not null)
                {
                    s.Push(p);
                    p = (2 * p) + 1;
                }

                p = s.Pop();
                var v = data[p];
                Debug.Assert(v is not null);
                yield return v.Value.Value;

                p = (2 * p) + 2;
            }
        }

        public override string ToString()
        {
            if (_data[0] is null)
                return string.Empty;

            var builder = new StringBuilder();

            const int spacer = 4;

            var queue = new Queue<int>();
            queue.Enqueue(0);

            var depth = (Count / 3) + 1;
            var currentWidth = 1;
            var maxWidth = depth * 2;
            var currentDepth = 0;

            while (currentWidth > 0)
            {
                if (queue.Count == 0)
                    break;

                for (int w = 0; w < currentWidth; w++)
                {
                    if (queue.Count == 0)
                        break;

                    var i = queue.Dequeue();
                    if (_data[i] is null)
                        continue;

                    var leftI = (2 * i) + 1;
                    var rightI = (2 * i) + 2;
                    queue.Enqueue(leftI);
                    queue.Enqueue(rightI);

                    if (w == 0)
                    {
                        var offset = spacer * (maxWidth - currentWidth);
                        for (int k = 0; k < offset; k++)
                            builder.Append(' ');
                    }
                    else
                    {
                        var offset = spacer * (maxWidth - currentWidth);
                        for (int k = 0; k < offset - 1; k++)
                            builder.Append(' ');
                    }

                    builder.Append(_data[i]?.Value?.ToString() ?? "?");
                }

                currentWidth *= 2;
                currentDepth++;
                builder.Append('\n');
            }

            return builder.ToString();
        }
    }

    private readonly record struct Node<T>(T Value);
}

public class Tests
{
    [Fact]
    public void ArrayBased()
    {
        var tree = new ArrayBased.BinaryTree<int>();

        for (int i = 0; i < 10; i++)
            tree.Insert(i);

        Assert.Equal(10, tree.Count);

        using var enumerator = tree.InOrder().GetEnumerator();
        var order = new int[tree.Count];
        for (int i = 0; i < 10; i++)
        {
            Assert.True(enumerator.MoveNext());
            order[i] = enumerator.Current;
        }

        Assert.False(enumerator.MoveNext());

        var str = tree.ToString();
        Debug.WriteLine(str);
        Assert.NotNull(str);
    }
}
