package internal

type Stack[T any] struct {
	size int
	data []T
}

func NewStack[T any]() *Stack[T] {
	return &Stack[T]{
		data: nil,
		size: 0,
	}
}

func (s *Stack[T]) Len() int {
	return s.size
}

func (s *Stack[T]) Cap() int {
	return len(s.data)
}

func (s *Stack[T]) ensureCapacity() {
	if s.data == nil {
		s.data = make([]T, 1)
	}

	if s.size == len(s.data) {
		newCap := s.size * 2
		if newCap < 4 {
			newCap = 4
		}
		newData := make([]T, newCap)
		copy(newData, s.data)
		s.data = newData
	}
}

func (s *Stack[T]) Push(value T) {
	s.ensureCapacity()

	size := s.size
	s.data[size] = value
	s.size = size + 1
}

func (s *Stack[T]) Peek() *T {
	if s.size == 0 {
		return nil
	}

	return &s.data[s.size-1]
}

func (s *Stack[T]) Pop() *T {
	if s.size == 0 {
		return nil
	}

	size := s.size
	s.size = size - 1
	return &s.data[size-1]
}
