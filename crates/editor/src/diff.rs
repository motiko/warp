use std::fmt;
use std::ops::Range;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffType {
    Create {
        /// The delta representing the creation.
        /// A delta for a file creation has an empty replacement line range.
        delta: DiffDelta,
    },
    Update {
        deltas: Vec<DiffDelta>,
        /// If set, the file should be renamed to this path when applying the diff.
        /// This path should also be a non-existing filepath.
        rename: Option<PathBuf>,
    },
    Delete {
        /// The delta representing the deletion.
        /// A delta for a file deletion has a replacement line range
        /// that spans the entire file and an empty insertion.
        delta: DiffDelta,
    },
}

impl DiffType {
    pub fn creation(content: String) -> Self {
        DiffType::Create {
            delta: DiffDelta {
                replacement_line_range: 0..0,
                insertion: content,
            },
        }
    }

    pub fn deletion(num_lines: usize) -> Self {
        DiffType::Delete {
            delta: DiffDelta {
                replacement_line_range: 1..num_lines.saturating_add(1),
                insertion: String::new(),
            },
        }
    }

    pub fn update(deltas: Vec<DiffDelta>, rename_to: Option<String>) -> Self {
        DiffType::Update {
            deltas,
            rename: rename_to.map(Into::into),
        }
    }
}

/// Visual representation of a single diff hunk.
#[derive(Clone, PartialEq, Eq)]
pub struct DiffDelta {
    pub replacement_line_range: Range<usize>,
    pub insertion: String,
}

impl fmt::Debug for DiffDelta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if cfg!(debug_assertions) {
            write!(
                f,
                "DiffDelta {{\nreplacement_line_range: {:?},",
                &self.replacement_line_range
            )?;
            f.write_str("\n--insertion--\n")?;
            f.write_str(&self.insertion)?;
            f.write_str("\n}")
        } else {
            Ok(())
        }
    }
}
