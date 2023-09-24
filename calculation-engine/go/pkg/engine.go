package engine

import (
	"calcengine/internal"
	"errors"
)

type ScalarEngine struct {
	expression *Expression
}

func (e ScalarEngine) Evaluate(input []float64) (float64, error) {
	if e.expression.requiredInputCount != len(input) {
		return 0, errors.New("wrong input size")
	}

	stack := internal.NewStack[float64]()

	operandIndex := 0
	for _, op := range e.expression.expression {
		if op.Type() == OperandType {
			stack.Push(input[operandIndex])
			operandIndex += 1
		} else if op.Type() == OperatorType {
			operator := op.(*Operator)

			right := stack.Pop()
			left := stack.Pop()

			var result float64
			switch operator {
			case AddOperator:
				result = left + right
			case SubOperator:
				result = left - right
			case MulOperator:
				result = left * right
			case DivOperator:
				result = left / right
			default:
				return 0, errors.New("unexpected operator")
			}
			stack.Push(result)
		}
	}

	if stack.Len() != 1 {
		return 0, errors.New("invalid expression")
	}

	return stack.Pop(), nil
}

type Expression struct {
	expression         []Node
	requiredInputCount int
}

func FromInfix(expression []Node) (*Expression, error) {
	if len(expression) == 0 {
		return nil, errors.New("expression cannot be nil or empty")
	}

	result := make([]Node, 0, len(expression))
	stack := internal.NewStack[Node]()

	for _, op := range expression {

		nodeType := op.Type()
		if nodeType == OperandType {
			result = append(result, op)
		} else if nodeType == LeftParensType {
			stack.Push(op)
		} else if nodeType == RightParensType {
			for n := stack.Peek(); n != nil && (*n).Type() != LeftParensType; n = stack.Peek() {
				result = append(result, stack.Pop())
			}

			if n := stack.Peek(); n != nil && (*n).Type() != LeftParensType {
				return nil, errors.New("invalid expression")
			}

			stack.Pop()
		} else {
			prec := precedence(op)
			for n := stack.Peek(); n != nil && prec <= precedence(*n); n = stack.Peek() {
				result = append(result, stack.Pop())
			}

			stack.Push(op)
		}
	}

	for stack.Len() > 0 {
		result = append(result, stack.Pop())
	}

	requiredInputCount := 0
	for _, n := range result {
		if n.Type() == OperandType {
			requiredInputCount++
		}
	}

	return &Expression{
		expression:         result,
		requiredInputCount: requiredInputCount,
	}, nil
}

func (e *Expression) ScalarEngine() ScalarEngine {
	return ScalarEngine{expression: e}
}

func precedence(n Node) int {
	nodeType := n.Type()
	if nodeType != OperatorType {
		return -1
	}

	switch n.(*Operator).value {
	case '+':
	case '-':
		return 1
	case '*':
	case '/':
		return 2
	default:
		return -1
	}
	return -1
}

type NodeType int

const (
	OperandType     = iota
	LeftParensType  = iota
	RightParensType = iota
	OperatorType    = iota
)

type Node interface {
	Type() NodeType
}

type Operand struct {
}

func (o *Operand) Type() NodeType {
	return OperandType
}

type LeftParens struct {
}

func (o *LeftParens) Type() NodeType {
	return LeftParensType
}

type RightParens struct {
}

func (o *RightParens) Type() NodeType {
	return RightParensType
}

var AddOperator *Operator = &Operator{value: '+'}
var SubOperator *Operator = &Operator{value: '-'}
var MulOperator *Operator = &Operator{value: '*'}
var DivOperator *Operator = &Operator{value: '/'}

type Operator struct {
	value rune
}

func (o *Operator) Type() NodeType {
	return OperatorType
}
