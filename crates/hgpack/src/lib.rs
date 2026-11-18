//! Reader for Heavy Gear's resource pack.
//!
//! The game keeps nearly all of its data in two files: `HEAVYG.MAP`, an
//! index of named, typed entries, and `HEAVYG.PRJ`, the records those
//! entries point at. The layout is specified in `docs/spec/heavy-gear.md`
//! §5; this crate implements that section and nothing beyond it.
//!
//! ```no_run
//! # fn main() -> Result<(), hgpack::Error> {
//! let map = std::fs::read("HEAVYG.MAP").unwrap();
//! let prj = std::fs::read("HEAVYG.PRJ").unwrap();
//! let index = hgpack::Index::parse(&map)?;
//! let pack = hgpack::Pack::open(&index, &prj)?;
//! for record in pack.records() {
//!     let bytes = pack.decode(record)?;
//!     println!("{} {}", index.entries[record.entry].key(), bytes.len());
//! }
//! # Ok(())
//! # }
//! ```

mod codec;
mod error;
mod index;
mod record;
#[cfg(test)]
mod testutil;

pub use codec::{Codec, Method};
pub use error::Error;
pub use index::{CodecName, Entry, Index, Name, TypeCount, TypeTag};
pub use record::{Pack, RECORD_HEADER_LEN, Record, Report};
