//! Types for source location tracking.

/// A unique identifier for a source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SourceId(pub u32);

/// A byte offset into a source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BytePos(pub u32);

/// A span of source code, defined by a starting and ending byte position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Span {
    pub lo: BytePos,
    pub hi: BytePos,
    pub source_id: SourceId,
}

impl Span {
    pub fn new(lo: BytePos, hi: BytePos, source_id: SourceId) -> Self {
        Self { lo, hi, source_id }
    }

    pub fn to(self, other: Span) -> Span {
        assert_eq!(self.source_id, other.source_id);
        Span::new(self.lo, other.hi, self.source_id)
    }
}
