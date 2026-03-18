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

    pub fn dummy() -> Self {
        Self {
            lo: BytePos(0),
            hi: BytePos(0),
            source_id: SourceId(0),
        }
    }
}

pub struct SourceFile {
    pub name: String,
    pub source: String,
    pub line_starts: Vec<usize>,
}

impl SourceFile {
    pub fn new(name: String, source: String) -> Self {
        let line_starts = codespan_reporting::files::line_starts(&source).collect();
        Self {
            name,
            source,
            line_starts,
        }
    }
}

#[derive(Default)]
pub struct SourceMap {
    files: Vec<SourceFile>,
}

impl SourceMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, name: String, source: String) -> SourceId {
        let id = SourceId(self.files.len() as u32);
        self.files.push(SourceFile::new(name, source));
        id
    }

    pub fn get_file(&self, id: SourceId) -> Option<&SourceFile> {
        self.files.get(id.0 as usize)
    }
}

impl<'a> codespan_reporting::files::Files<'a> for SourceMap {
    type FileId = SourceId;
    type Name = &'a str;
    type Source = &'a str;

    fn name(&'a self, id: Self::FileId) -> Result<Self::Name, codespan_reporting::files::Error> {
        self.files
            .get(id.0 as usize)
            .map(|f| f.name.as_str())
            .ok_or(codespan_reporting::files::Error::FileMissing)
    }

    fn source(
        &'a self,
        id: Self::FileId,
    ) -> Result<Self::Source, codespan_reporting::files::Error> {
        self.files
            .get(id.0 as usize)
            .map(|f| f.source.as_str())
            .ok_or(codespan_reporting::files::Error::FileMissing)
    }

    fn line_index(
        &'a self,
        id: Self::FileId,
        byte_index: usize,
    ) -> Result<usize, codespan_reporting::files::Error> {
        let file = self
            .files
            .get(id.0 as usize)
            .ok_or(codespan_reporting::files::Error::FileMissing)?;
        Ok(file
            .line_starts
            .binary_search(&byte_index)
            .unwrap_or_else(|x| x - 1))
    }

    fn line_range(
        &'a self,
        id: Self::FileId,
        line_index: usize,
    ) -> Result<std::ops::Range<usize>, codespan_reporting::files::Error> {
        let file = self
            .files
            .get(id.0 as usize)
            .ok_or(codespan_reporting::files::Error::FileMissing)?;
        let start = *file.line_starts.get(line_index).ok_or(
            codespan_reporting::files::Error::LineTooLarge {
                given: line_index,
                max: file.line_starts.len(),
            },
        )?;
        let end = file
            .line_starts
            .get(line_index + 1)
            .cloned()
            .unwrap_or(file.source.len());
        Ok(start..end)
    }
}
