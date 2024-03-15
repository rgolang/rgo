package omap

type Map[K comparable, V any] struct {
	Index []V
	Map   map[K]int
}

func New[K comparable, V any]() *Map[K, V] {
	return &Map[K, V]{
		Index: make([]V, 0),
		Map:   make(map[K]int),
	}
}

func (m *Map[K, V]) Set(k K, v V) {
	if i, ok := m.Map[k]; ok {
		m.Index[i] = v
		return
	}
	m.Map[k] = len(m.Index)
	m.Index = append(m.Index, v)
}

func (m *Map[K, V]) Get(k K) (V, bool) {
	i, ok := m.Map[k]
	if ok {
		return m.Index[i], true
	}
	var zero V
	return zero, false
}

func (m *Map[K, V]) Each(cb func(k K, v V)) {
	km := make(map[int]K)
	for k, i := range m.Map {
		km[i] = k
	}
	for i, v := range m.Index {
		cb(km[i], v)
	}
}
