package engine

import (
	"testing"
)

func getSimpleExression() []Node {
	// a + (b - c)
	nodes := [...]Node{
		&Operand{},
		AddOperator,
		&LeftParens{},
		&Operand{},
		SubOperator,
		&Operand{},
		&RightParens{},
	}
	return nodes[:]
}

func TestExpressionFromInfix(t *testing.T) {
	nodes := getSimpleExression()

	_, err := FromInfix(nodes)
	if err != nil {
		t.Errorf("Error creating expression: %s", err)
	}
}

func TestExpressionScalar(t *testing.T) {
	nodes := getSimpleExression()

	expression, err := FromInfix(nodes)
	if err != nil {
		t.Errorf("Error creating expression: %s", err)
	}

	// 1 + (2 - 1)
	result, err := expression.ScalarEngine().Evaluate([]float64{1.0, 2.0, 1.0})
	if err != nil {
		t.Errorf("Error evaluating expression: %s", err)
	}

	if result != 2.0 {
		t.Errorf("Expected result to be 2.0, got %f", result)
	}
}

var Size int = 512

func BenchmarkEngine(b *testing.B) {
	nodes := getSimpleExression()

	expression, err := FromInfix(nodes)
	if err != nil {
		b.Errorf("Error creating expression: %s", err)
		return
	}

	engine := expression.ScalarEngine()
	input := []float64{1.0, 2.0, 1.0}

	b.Run("ScalarBaseline", func(b *testing.B) {
		for i := 0; i < b.N; i++ {
			results := make([]float64, Size)
			for j := 0; j < Size; j++ {
				results[j] = input[0] + (input[1] - input[2])
			}
		}
	})

	b.Run("ScalarEngine", func(b *testing.B) {
		for i := 0; i < b.N; i++ {
			results := make([]float64, Size)
			for j := 0; j < Size; j++ {
				result, err := engine.Evaluate(input)
				if err != nil {
					b.Errorf("Error evaluating expression: %s", err)
					return
				}
				results[j] = result
			}
		}
	})
}
