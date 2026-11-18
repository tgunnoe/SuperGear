use std::fmt;

use crate::Tag;

/// Why a `BWD` file was rejected. Offsets are bytes from the start of
/// the file; a chunk's offset is where its tag starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Shorter than the 12-byte header.
    Truncated { have: usize },
    /// The first four bytes are not `BWD\0`.
    Magic { found: [u8; 4] },
    /// The header's size word differs from the file's length.
    Length { stated: u32, actual: usize },
    /// Fewer than 8 bytes remain where a chunk header should start.
    ChunkHeader { at: u32 },
    /// A chunk's size is below its own 8 header bytes or runs past the end.
    ChunkSize { at: u32, size: u32 },
    /// A tag the engine's table does not list.
    UnknownTag { at: u32, tag: Tag },
    /// The header's third word differs from the size of the largest chunk.
    Largest { stated: u32, actual: u32 },
    /// A decoded chunk whose data is not the size its layout fixes.
    PayloadSize {
        at: u32,
        tag: Tag,
        expected: u32,
        actual: u32,
    },
    /// A closing chunk (`MOFF`, `ENDB`, `ETAG`) without its opener.
    Unopened { at: u32, tag: Tag },
    /// An opening chunk (`MON`, `BLK`, `BTAG`) still open at the end.
    Unclosed { at: u32, tag: Tag },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Truncated { have } => {
                write!(f, "{have} bytes, shorter than the 12-byte header")
            }
            Error::Magic { found } => write!(f, "magic {found:02x?}, not BWD\\0"),
            Error::Length { stated, actual } => {
                write!(f, "header says {stated} bytes, the file has {actual}")
            }
            Error::ChunkHeader { at } => write!(f, "at {at:#x}: no room for a chunk header"),
            Error::ChunkSize { at, size } => {
                write!(f, "at {at:#x}: chunk size {size} out of range")
            }
            Error::UnknownTag { at, tag } => write!(f, "at {at:#x}: unknown tag {:02x?}", tag.0),
            Error::Largest { stated, actual } => write!(
                f,
                "header says the largest chunk is {stated} bytes, it is {actual}"
            ),
            Error::PayloadSize {
                at,
                tag,
                expected,
                actual,
            } => write!(
                f,
                "at {at:#x}: {tag} has {actual} data bytes, its layout has {expected}"
            ),
            Error::Unopened { at, tag } => write!(f, "at {at:#x}: {tag} closes nothing"),
            Error::Unclosed { at, tag } => write!(f, "at {at:#x}: {tag} is never closed"),
        }
    }
}

impl std::error::Error for Error {}
