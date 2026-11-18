use std::fmt;

use crate::codec::Codec;
use crate::index::TypeTag;

/// Everything that can be wrong with an index, a pack or a record.
///
/// `entry` fields are positions in [`crate::Index::entries`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The input ends before a structure it must contain.
    Truncated {
        what: &'static str,
        need: u64,
        have: u64,
    },
    /// The index is not exactly as long as its header declares.
    IndexLength { expected: u64, actual: u64 },
    /// The type table's counts do not sum to the number of entries.
    TypeCountSum { sum: u64, entries: u32 },
    /// An entry's type is absent from the type table.
    UnknownType { entry: usize },
    /// A type's tally among the entries differs from its type-table count.
    TypeTally {
        kind: TypeTag,
        table: u32,
        entries: u32,
    },
    /// An entry's raw name sorts before its predecessor's.
    Unsorted { entry: usize },
    /// A record's 12-byte tag is not `C:\PRJ\` + initial + type.
    BadTag { entry: usize },
    /// A record does not start where the previous one ends.
    Tiling {
        entry: usize,
        expected: u64,
        actual: u64,
    },
    /// Bytes follow the last record.
    TrailingBytes { at: u64, len: u64 },
    /// A record's method code names no entry of the codec table.
    UnknownMethod { entry: usize, code: u16 },
    /// The codec table names a DLL this crate has no decoder for.
    UnknownCodec { entry: usize, dll: String },
    /// A record decoded to a length other than its header's.
    SizeMismatch {
        entry: usize,
        expected: u32,
        actual: u64,
    },
    /// A record decodes to more bytes than its header's unpacked size.
    Overlong { entry: usize, expected: u32 },
    /// A codec rejected a record's packed bytes.
    Corrupt {
        entry: usize,
        codec: Codec,
        detail: String,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Truncated { what, need, have } => {
                write!(f, "truncated {what}: need {need} bytes, have {have}")
            }
            Error::IndexLength { expected, actual } => {
                write!(f, "index is {actual} bytes, header declares {expected}")
            }
            Error::TypeCountSum { sum, entries } => {
                write!(f, "type counts sum to {sum}, index has {entries} entries")
            }
            Error::UnknownType { entry } => write!(f, "entry {entry}: type not in type table"),
            Error::TypeTally {
                kind,
                table,
                entries,
            } => write!(
                f,
                "type {kind}: type table says {table}, entries have {entries}"
            ),
            Error::Unsorted { entry } => {
                write!(f, "entry {entry}: name sorts before entry {}", entry - 1)
            }
            Error::BadTag { entry } => write!(
                f,
                "entry {entry}: record tag does not match its name and type"
            ),
            Error::Tiling {
                entry,
                expected,
                actual,
            } => write!(
                f,
                "entry {entry}: record at {actual}, previous record ends at {expected}"
            ),
            Error::TrailingBytes { at, len } => {
                write!(f, "{len} bytes after the last record, from {at}")
            }
            Error::UnknownMethod { entry, code } => {
                write!(f, "entry {entry}: method {code} is not in the codec table")
            }
            Error::UnknownCodec { entry, dll } => {
                write!(f, "entry {entry}: no decoder for codec {dll:?}")
            }
            Error::SizeMismatch {
                entry,
                expected,
                actual,
            } => write!(
                f,
                "entry {entry}: decoded {actual} bytes, header says {expected}"
            ),
            Error::Overlong { entry, expected } => {
                write!(
                    f,
                    "entry {entry}: decodes to more than the header's {expected} bytes"
                )
            }
            Error::Corrupt {
                entry,
                codec,
                detail,
            } => write!(f, "entry {entry}: {codec} stream rejected: {detail}"),
        }
    }
}

impl std::error::Error for Error {}
