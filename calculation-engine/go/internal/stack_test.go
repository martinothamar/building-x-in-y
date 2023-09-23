package internal

import (
	"testing"
)

func TestStackInit(t *testing.T) {
	stack := NewStack[int]()
	if stack == nil {
		t.Errorf("NewStack() returned nil")
	}
}

func TestPush(t *testing.T) {
	stack := NewStack[int]()
	stack.Push(1)
}

func TestLen(t *testing.T) {
	stack := NewStack[int]()
	expected := 0
	if stack.Len() != expected {
		t.Errorf("Len() returned %v, not %v", stack.Len(), expected)
	}
	expected = 1
	stack.Push(0)
	if stack.Len() != expected {
		t.Errorf("Len() returned %v, not %v", stack.Len(), expected)
	}

	expected = 2
	stack.Push(0)
	if stack.Len() != expected {
		t.Errorf("Len() returned %v, not %v", stack.Len(), expected)
	}
}

func TestCap(t *testing.T) {
	stack := NewStack[int]()
	expected := 0
	if stack.Cap() != expected {
		t.Errorf("Cap() returned %v, not %v", stack.Cap(), expected)
	}
	expected = 1
	stack.Push(0)
	if stack.Cap() != expected {
		t.Errorf("Cap() returned %v, not %v", stack.Cap(), expected)
	}

	expected = 4
	stack.Push(0)
	if stack.Cap() != expected {
		t.Errorf("Cap() returned %v, not %v", stack.Cap(), expected)
	}
}

func TestPeek(t *testing.T) {
	stack := NewStack[int]()

	value := stack.Peek()
	if value != nil {
		t.Errorf("Peek() returned %v, not %v", value, nil)
	}

	expected := 1
	stack.Push(expected)
	value = stack.Peek()
	if value == nil || *value != expected {
		t.Errorf("Peek() returned %v, not %v", value, expected)
	}

	if stack.Len() != 1 {
		t.Errorf("Len() returned %v, not %v", stack.Len(), 1)
	}
}

func TestPop(t *testing.T) {
	stack := NewStack[int]()

	value := stack.Pop()
	if value != nil {
		t.Errorf("Pop() returned %v, not %v", value, nil)
	}

	expected := 1
	stack.Push(expected)
	value = stack.Pop()
	if value == nil || *value != expected {
		t.Errorf("Pop() returned %v, not %v", value, expected)
	}

	if stack.Len() != 0 {
		t.Errorf("Len() returned %v, not %v", stack.Len(), 0)
	}
}
