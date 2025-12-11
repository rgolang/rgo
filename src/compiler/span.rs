#[derive(Debug, Clone, Copy, Default)]
pub struct Span {
    pub line: usize,
    pub column: usize,
    pub offset: usize,
}

impl Span {
    pub const fn new(line: usize, column: usize, offset: usize) -> Self {
        Self {
            line,
            column,
            offset,
        }
    }

    pub const fn unknown() -> Self {
        Self {
            line: 0,
            column: 0,
            offset: 0,
        }
    }
}

// 1. Equality ignores the position
impl PartialEq for Span {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl Eq for Span {}

// 2. Hashing ignores the position
impl std::hash::Hash for Span {
    fn hash<H: std::hash::Hasher>(&self, _state: &mut H) {
        // Intentionally do nothing.
        // This ensures that two structs differing only by Span
        // will produce the same hash.
    }
}
