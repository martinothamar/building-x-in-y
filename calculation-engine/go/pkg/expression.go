package expression

import (
	"calcengine/internal"
	"errors"
)

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
			for n := stack.Peek(); n != nil && (*n).Type() != LeftParensType; {
				result = append(result, *stack.Pop())
			}

			if n := stack.Peek(); n != nil && (*n).Type() != LeftParensType {
				return nil, errors.New("invalid expression")
			}

			stack.Pop()
		} else {
			prec := precedence(op)
			for n := stack.Peek(); n != nil && prec <= precedence(*n); {
				result = append(result, *stack.Pop())
			}

			stack.Push(op)
		}
	}

	for stack.Len() > 0 {
		result = append(result, *stack.Pop())
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

var Add Operator = Operator{value: '+'}
var Sub Operator = Operator{value: '-'}
var Mul Operator = Operator{value: '*'}
var Div Operator = Operator{value: '/'}

type Operator struct {
	value rune
}

func (o *Operator) Type() NodeType {
	return OperatorType
}
