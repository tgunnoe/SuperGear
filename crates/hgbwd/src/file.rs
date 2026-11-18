//! The file: a 12-byte header, then chunks to the end.

use crate::payload::{self, Payload};
use crate::{Error, Tag, nesting};

/// Bytes before the first chunk: magic, size, largest-chunk size.
pub const HEADER_LEN: usize = 12;
/// The first four bytes of every `BWD` file.
pub const MAGIC: [u8; 4] = *b"BWD\0";
/// Bytes of a chunk's own header: tag and size.
pub const CHUNK_HEADER_LEN: usize = 8;

/// One chunk, borrowed from the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chunk<'a> {
    /// Where the chunk's tag starts, in bytes from the start of the file.
    pub offset: u32,
    pub tag: Tag,
    /// The bytes after the chunk's 8-byte header.
    pub data: &'a [u8],
    /// `data` decoded, for the tags whose layout is known.
    pub payload: Payload<'a>,
}

impl Chunk<'_> {
    /// The size the chunk's header states: its data and its 8 header bytes.
    pub fn size(&self) -> u32 {
        (CHUNK_HEADER_LEN + self.data.len()) as u32
    }
}

/// The four bytes of `bytes` at `at`, which the caller has bounds-checked.
fn array(bytes: &[u8], at: usize) -> [u8; 4] {
    let mut a = [0; 4];
    a.copy_from_slice(&bytes[at..at + 4]);
    a
}

/// A parsed `BWD` file (spec §5, "BWD files").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bwd<'a> {
    largest: u32,
    chunks: Vec<Chunk<'a>>,
    unclosed_btags: Vec<usize>,
}

impl<'a> Bwd<'a> {
    /// Parses `bytes`, accepting exactly the files whose header is sound,
    /// whose chunks have known tags and tile the rest of the file, whose
    /// decoded chunks have their layout's size, and whose brackets nest.
    pub fn parse(bytes: &'a [u8]) -> Result<Bwd<'a>, Error> {
        let len = bytes.len();
        if len < HEADER_LEN {
            return Err(Error::Truncated { have: len });
        }
        let found = array(bytes, 0);
        if found != MAGIC {
            return Err(Error::Magic { found });
        }
        let stated = u32::from_le_bytes(array(bytes, 4));
        if stated as usize != len {
            return Err(Error::Length {
                stated,
                actual: len,
            });
        }
        // From here every offset is below `stated`, so it fits a u32.
        let mut chunks = Vec::new();
        let mut at = HEADER_LEN;
        while at < len {
            let offset = at as u32;
            if len - at < CHUNK_HEADER_LEN {
                return Err(Error::ChunkHeader { at: offset });
            }
            let tag = Tag(array(bytes, at));
            let size = u32::from_le_bytes(array(bytes, at + 4));
            if (size as usize) < CHUNK_HEADER_LEN || size as usize > len - at {
                return Err(Error::ChunkSize { at: offset, size });
            }
            if !tag.is_known() {
                return Err(Error::UnknownTag { at: offset, tag });
            }
            let data = &bytes[at + CHUNK_HEADER_LEN..at + size as usize];
            let payload = payload::decode(tag, data).map_err(|expected| Error::PayloadSize {
                at: offset,
                tag,
                expected: expected as u32,
                actual: data.len() as u32,
            })?;
            chunks.push(Chunk {
                offset,
                tag,
                data,
                payload,
            });
            at += size as usize;
        }
        let largest = u32::from_le_bytes(array(bytes, 8));
        let actual = chunks.iter().map(Chunk::size).max().unwrap_or(0);
        if largest != actual {
            return Err(Error::Largest {
                stated: largest,
                actual,
            });
        }
        let unclosed_btags = nesting::check(&chunks)?;
        Ok(Bwd {
            largest,
            chunks,
            unclosed_btags,
        })
    }

    /// The header's third word, the size of the largest chunk (0 when
    /// there is none).
    pub fn largest(&self) -> u32 {
        self.largest
    }

    pub fn chunks(&self) -> &[Chunk<'a>] {
        &self.chunks
    }

    /// Indices into [`chunks`](Self::chunks) of each `BTAG` that another
    /// `BTAG` followed before any `ETAG`.
    pub fn unclosed_btags(&self) -> &[usize] {
        &self.unclosed_btags
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::TAGS;
    use crate::testutil::{build, data};

    fn sample() -> Vec<(Tag, Vec<u8>)> {
        vec![
            (Tag::REV, b"1.22".to_vec()),
            (Tag::DTBL, vec![0; 20]),
            (Tag::INCL, b"\xff\xffm33_tabl\0\0".to_vec()),
            (Tag::BTAG, b"trainstr\0\0\0\0\0\0\0\0".to_vec()),
            (Tag::new("GPS"), vec![9; 92]),
            (Tag::ETAG, vec![0; 4]),
        ]
    }

    #[test]
    fn parses_a_built_file() {
        let bytes = build(&sample());
        let bwd = Bwd::parse(&bytes).unwrap();
        assert_eq!(bwd.largest(), 100);
        let c = bwd.chunks();
        assert_eq!(c.len(), 6);
        assert_eq!(c[0].offset, 12);
        assert_eq!(c[1].offset, 12 + 12);
        assert_eq!(c[2].offset, 12 + 12 + 28);
        assert_eq!(c[4].size(), 100);
        assert_eq!(c[0].payload, Payload::Rev(b"1.22"));
        assert_eq!(
            c[2].payload,
            Payload::NameRef {
                slot: 0xffff,
                name: b"m33_tabl"
            }
        );
        assert_eq!(c[4].payload, Payload::Raw(&[9; 92]));
        assert_eq!(c[4].data, &[9; 92]);
        assert!(bwd.unclosed_btags().is_empty());
    }

    #[test]
    fn a_header_alone_is_an_empty_file() {
        let bytes = build(&[]);
        let bwd = Bwd::parse(&bytes).unwrap();
        assert!(bwd.chunks().is_empty());
        assert_eq!(bwd.largest(), 0);
    }

    #[test]
    fn rejects_a_short_or_foreign_header() {
        assert_eq!(Bwd::parse(b"BWD\0"), Err(Error::Truncated { have: 4 }));
        let mut bytes = build(&sample());
        bytes[3] = b' ';
        assert_eq!(Bwd::parse(&bytes), Err(Error::Magic { found: *b"BWD " }));
    }

    #[test]
    fn rejects_a_wrong_stated_length() {
        let mut bytes = build(&sample());
        let actual = bytes.len();
        bytes[4..8].copy_from_slice(&(actual as u32 - 1).to_le_bytes());
        assert_eq!(
            Bwd::parse(&bytes),
            Err(Error::Length {
                stated: actual as u32 - 1,
                actual
            })
        );
    }

    /// `bytes` with `extra` appended and the size word updated.
    fn grown(mut bytes: Vec<u8>, extra: &[u8]) -> Vec<u8> {
        bytes.extend(extra);
        let n = bytes.len() as u32;
        bytes[4..8].copy_from_slice(&n.to_le_bytes());
        bytes
    }

    #[test]
    fn rejects_bytes_too_few_for_a_chunk_header() {
        let base = build(&sample());
        let at = base.len() as u32;
        assert_eq!(
            Bwd::parse(&grown(base, b"OBJ\0")),
            Err(Error::ChunkHeader { at })
        );
    }

    #[test]
    fn rejects_a_chunk_size_below_8_or_past_the_end() {
        let base = build(&sample());
        let at = base.len() as u32;
        for size in [0u32, 7, 9] {
            let extra = [b"OBJ\0".as_slice(), &size.to_le_bytes()].concat();
            assert_eq!(
                Bwd::parse(&grown(base.clone(), &extra)),
                Err(Error::ChunkSize { at, size }),
                "{size}"
            );
        }
    }

    #[test]
    fn rejects_a_tag_outside_the_table() {
        let mut chunks = sample();
        chunks[4].0 = Tag::new("XYZ");
        assert_eq!(
            Bwd::parse(&build(&chunks)),
            Err(Error::UnknownTag {
                at: 12 + 12 + 28 + 20 + 24,
                tag: Tag::new("XYZ")
            })
        );
    }

    #[test]
    fn rejects_a_wrong_largest_chunk_word() {
        let mut bytes = build(&sample());
        bytes[8..12].copy_from_slice(&99u32.to_le_bytes());
        assert_eq!(
            Bwd::parse(&bytes),
            Err(Error::Largest {
                stated: 99,
                actual: 100
            })
        );
    }

    #[test]
    fn rejects_a_decoded_chunk_of_the_wrong_size() {
        let mut chunks = sample();
        chunks[2].1.push(0);
        assert_eq!(
            Bwd::parse(&build(&chunks)),
            Err(Error::PayloadSize {
                at: 12 + 12 + 28,
                tag: Tag::INCL,
                expected: 12,
                actual: 13
            })
        );
    }

    /// Tags whose order the parser does not constrain: no brackets.
    fn free_tag() -> impl Strategy<Value = Tag> {
        let brackets = [
            Tag::MON,
            Tag::MOFF,
            Tag::BLK,
            Tag::ENDB,
            Tag::BTAG,
            Tag::ETAG,
        ];
        proptest::sample::select(
            TAGS.iter()
                .copied()
                .filter(|t| !brackets.contains(t))
                .collect::<Vec<_>>(),
        )
    }

    /// A chunk with data its layout accepts: the fixed size, or any bytes.
    fn chunk() -> impl Strategy<Value = (Tag, Vec<u8>)> {
        (free_tag(), proptest::collection::vec(any::<u8>(), 0..64)).prop_map(|(tag, raw)| {
            match payload::fixed_size(tag) {
                Some(n) => (
                    tag,
                    raw.into_iter()
                        .chain(std::iter::repeat(0))
                        .take(n)
                        .collect(),
                ),
                None => (tag, raw),
            }
        })
    }

    proptest! {
        #[test]
        fn built_files_parse_back(chunks in proptest::collection::vec(chunk(), 0..24)) {
            let bytes = build(&chunks);
            let bwd = Bwd::parse(&bytes).unwrap();
            prop_assert_eq!(bwd.chunks().len(), chunks.len());
            let mut at = HEADER_LEN as u32;
            for (c, (tag, d)) in bwd.chunks().iter().zip(&chunks) {
                prop_assert_eq!(c.tag, *tag);
                prop_assert_eq!(c.data, d.as_slice());
                prop_assert_eq!(c.offset, at);
                at += c.size();
            }
            prop_assert_eq!(at as usize, bytes.len());
        }

        #[test]
        fn arbitrary_bytes_never_panic(bytes in proptest::collection::vec(any::<u8>(), 0..256)) {
            let _ = Bwd::parse(&bytes);
        }

        #[test]
        fn a_corrupted_file_never_panics(
            tags in proptest::collection::vec(free_tag(), 1..12),
            at in any::<proptest::sample::Index>(),
            value in any::<u32>(),
        ) {
            let chunks: Vec<_> = tags.into_iter().map(|t| (t, data(t))).collect();
            let mut bytes = build(&chunks);
            let i = at.index(bytes.len() - 3);
            bytes[i..i + 4].copy_from_slice(&value.to_le_bytes());
            let _ = Bwd::parse(&bytes);
        }
    }
}
