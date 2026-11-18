//! Reader for Heavy Gear's `BWD` files.
//!
//! Scenarios and the files they include (object lists, art settings,
//! texture tables) are all `BWD` files: a 12-byte header, then tagged
//! chunks from a vocabulary of 96 tags the engine carries. The layout is
//! specified in `docs/spec/heavy-gear.md` §5, "BWD files"; this crate
//! implements that section and decodes only the chunk layouts it settles.
//!
//! ```no_run
//! # fn main() -> Result<(), hgbwd::Error> {
//! let bytes = std::fs::read("M33_SCN1.BWD").unwrap();
//! let bwd = hgbwd::Bwd::parse(&bytes)?;
//! for chunk in bwd.chunks() {
//!     if let hgbwd::Payload::NameRef { name, .. } = chunk.payload {
//!         println!("{} {}", chunk.tag, String::from_utf8_lossy(name));
//!     }
//! }
//! # Ok(())
//! # }
//! ```

mod error;
mod file;
mod nesting;
mod payload;
mod tag;
#[cfg(test)]
mod testutil;

pub use error::Error;
pub use file::{Bwd, CHUNK_HEADER_LEN, Chunk, HEADER_LEN, MAGIC};
pub use payload::Payload;
pub use tag::{TAGS, Tag};
