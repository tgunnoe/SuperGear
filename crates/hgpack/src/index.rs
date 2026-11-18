//! `HEAVYG.MAP`: the index of the resource pack (spec §5).
//!
//! Layout, all integers little-endian:
//!
//! | Size | Field |
//! | ---: | :--- |
//! | 4 | entry count *n* |
//! | 4 | type count *t* |
//! | 8 × *t* | type table: char\[4\] type, u32 entry count |
//! | 4 | reserved, zero in the shipped index |
//! | 4 | codec count *c* |
//! | 16 × *c* | codec table: char\[16\] DLL name |
//! | 20 × *n* | entries: char\[8\] name, char\[4\] type, u32 pack offset, char\[4\] attribute |
//!
//! Entries are in ascending order of the raw 8-byte name field.

use std::fmt;

use crate::Error;

/// A fixed-width, NUL-padded text field kept exactly as stored.
///
/// The text is the bytes before the first NUL. Bytes after it are kept
/// because they take part in the index's sort order (`ALLWHT\0Y`).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Field<const N: usize>(pub [u8; N]);

/// An entry's 8-byte name field.
pub type Name = Field<8>;
/// A resource type such as `WAV` or `PCXX`.
pub type TypeTag = Field<4>;
/// A codec table entry: the DLL that decodes one method.
pub type CodecName = Field<16>;

impl<const N: usize> Field<N> {
    /// The text: the bytes before the first NUL.
    pub fn as_bytes(&self) -> &[u8] {
        let end = self.0.iter().position(|&b| b == 0).unwrap_or(N);
        &self.0[..end]
    }

    /// The whole field, including any bytes after the first NUL.
    pub fn raw(&self) -> &[u8; N] {
        &self.0
    }
}

impl<const N: usize> fmt::Display for Field<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&String::from_utf8_lossy(self.as_bytes()))
    }
}

impl<const N: usize> fmt::Debug for Field<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", String::from_utf8_lossy(&self.0))
    }
}

/// A type-table row: a resource type and how many entries have it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TypeCount {
    pub kind: TypeTag,
    pub count: u32,
}

/// One index entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entry {
    pub name: Name,
    pub kind: TypeTag,
    /// Offset of the entry's record in the pack.
    pub offset: u32,
    /// Four bytes of unknown meaning: `NONE` in every shipped entry.
    pub attr: [u8; 4],
}

impl Entry {
    /// `NAME.TYPE`, the name the game's loose-file probe uses (spec §4).
    pub fn key(&self) -> String {
        format!("{}.{}", self.name, self.kind)
    }
}

/// A parsed `HEAVYG.MAP`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Index {
    pub types: Vec<TypeCount>,
    /// The word between the type table and the codec count; zero in the
    /// shipped index, meaning unknown.
    pub reserved: u32,
    /// Method *k* (1-based) of a record is decoded by `codecs[k - 1]`.
    pub codecs: Vec<CodecName>,
    pub entries: Vec<Entry>,
}

impl Index {
    /// Parses and checks an index: exact length, a type table that agrees
    /// with the entries, and entries in raw-name order.
    pub fn parse(bytes: &[u8]) -> Result<Index, Error> {
        let have = bytes.len() as u64;
        let need = |what, need: u64| {
            if have < need {
                Err(Error::Truncated { what, need, have })
            } else {
                Ok(())
            }
        };
        need("index header", 8)?;
        let n = u32_at(bytes, 0);
        let t = u64::from(u32_at(bytes, 4));
        let types_end = 8 + 8 * t;
        need("type table", types_end + 8)?;
        let codec_count = u64::from(u32_at(bytes, types_end as usize + 4));
        let codecs_end = types_end + 8 + 16 * codec_count;
        need("codec table", codecs_end)?;
        let expected = codecs_end + 20 * u64::from(n);
        if have != expected {
            return Err(Error::IndexLength {
                expected,
                actual: have,
            });
        }
        // Every offset below is now within `bytes`.
        let (types_end, codecs_end) = (types_end as usize, codecs_end as usize);

        let types: Vec<TypeCount> = (0..t as usize)
            .map(|i| TypeCount {
                kind: Field(array(bytes, 8 + 8 * i)),
                count: u32_at(bytes, 12 + 8 * i),
            })
            .collect();
        let reserved = u32_at(bytes, types_end);
        let codecs = (0..codec_count as usize)
            .map(|i| Field(array(bytes, types_end + 8 + 16 * i)))
            .collect();
        let entries: Vec<Entry> = (0..n as usize)
            .map(|i| {
                let at = codecs_end + 20 * i;
                Entry {
                    name: Field(array(bytes, at)),
                    kind: Field(array(bytes, at + 8)),
                    offset: u32_at(bytes, at + 12),
                    attr: array(bytes, at + 16),
                }
            })
            .collect();

        let sum: u64 = types.iter().map(|t| u64::from(t.count)).sum();
        if sum != u64::from(n) {
            return Err(Error::TypeCountSum { sum, entries: n });
        }
        let mut tally = vec![0u32; types.len()];
        for (i, e) in entries.iter().enumerate() {
            let j = types
                .iter()
                .position(|t| t.kind == e.kind)
                .ok_or(Error::UnknownType { entry: i })?;
            tally[j] += 1;
        }
        if let Some((t, &entries)) = types.iter().zip(&tally).find(|(t, n)| t.count != **n) {
            return Err(Error::TypeTally {
                kind: t.kind,
                table: t.count,
                entries,
            });
        }
        if let Some(i) = (1..entries.len()).find(|&i| entries[i - 1].name > entries[i].name) {
            return Err(Error::Unsorted { entry: i });
        }

        Ok(Index {
            types,
            reserved,
            codecs,
            entries,
        })
    }
}

/// The little-endian u32 at `at`; the caller has checked the bounds.
pub(crate) fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(array(b, at))
}

/// The `N` bytes at `at`; the caller has checked the bounds.
pub(crate) fn array<const N: usize>(b: &[u8], at: usize) -> [u8; N] {
    let mut a = [0; N];
    a.copy_from_slice(&b[at..at + N]);
    a
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{RawEntry, RawIndex, build, field, stored};
    use proptest::prelude::*;

    fn sample() -> RawIndex {
        build(
            vec![
                stored(b"BETA", b"PCX", b"pcx bytes"),
                stored(b"ALPHA", b"WAV", b"wav bytes"),
                stored(b"ALPHA", b"PCXX", b"pcxx"),
            ],
            &[b"pkware.dll", b"zlib.dll"],
        )
        .0
    }

    #[test]
    fn parses_header_tables_and_entries() {
        let index = Index::parse(&sample().to_bytes()).unwrap();
        let types: Vec<_> = index
            .types
            .iter()
            .map(|t| (t.kind.to_string(), t.count))
            .collect();
        assert_eq!(
            types,
            [("WAV".into(), 1), ("PCXX".into(), 1), ("PCX".into(), 1)]
        );
        assert_eq!(index.reserved, 0);
        let codecs: Vec<_> = index.codecs.iter().map(|c| c.to_string()).collect();
        assert_eq!(codecs, ["pkware.dll", "zlib.dll"]);
        let keys: Vec<_> = index.entries.iter().map(Entry::key).collect();
        assert_eq!(keys, ["ALPHA.WAV", "ALPHA.PCXX", "BETA.PCX"]);
        let offsets: Vec<_> = index.entries.iter().map(|e| e.offset).collect();
        assert_eq!(offsets, [0, 22 + 9, 22 + 9 + 22 + 4]);
        assert!(index.entries.iter().all(|e| &e.attr == b"NONE"));
    }

    #[test]
    fn name_is_bytes_before_nul_and_raw_field_is_kept() {
        let (raw, _) = build(vec![stored(b"ALLWHT\0Y", b"COL", b"x")], &[]);
        let index = Index::parse(&raw.to_bytes()).unwrap();
        let name = index.entries[0].name;
        assert_eq!(name.as_bytes(), b"ALLWHT");
        assert_eq!(name.raw(), b"ALLWHT\0Y");
        assert_eq!(index.entries[0].key(), "ALLWHT.COL");
    }

    #[test]
    fn rejects_short_header() {
        assert!(matches!(
            Index::parse(&[0; 7]),
            Err(Error::Truncated {
                need: 8,
                have: 7,
                ..
            })
        ));
    }

    #[test]
    fn rejects_type_table_cut_short() {
        let mut b = sample().to_bytes();
        b.truncate(8 + 8); // header plus one of three type rows
        assert!(matches!(Index::parse(&b), Err(Error::Truncated { .. })));
    }

    #[test]
    fn rejects_length_other_than_declared() {
        let b = sample().to_bytes();
        let n = b.len() as u64;
        let mut long = b.clone();
        long.push(0);
        assert_eq!(
            Index::parse(&long),
            Err(Error::IndexLength {
                expected: n,
                actual: n + 1
            })
        );
        assert_eq!(
            Index::parse(&b[..b.len() - 1]),
            Err(Error::IndexLength {
                expected: n,
                actual: n - 1
            })
        );
    }

    #[test]
    fn rejects_type_counts_not_summing_to_entries() {
        let mut raw = sample();
        raw.types[0].1 = 2;
        assert_eq!(
            Index::parse(&raw.to_bytes()),
            Err(Error::TypeCountSum { sum: 4, entries: 3 })
        );
    }

    #[test]
    fn rejects_entry_type_missing_from_table() {
        let mut raw = sample();
        raw.entries[2].kind = field(b"XYZ");
        assert_eq!(
            Index::parse(&raw.to_bytes()),
            Err(Error::UnknownType { entry: 2 })
        );
    }

    #[test]
    fn rejects_type_tally_differing_from_table() {
        let mut raw = sample();
        raw.entries[2].kind = field(b"WAV"); // two WAV, no PCX
        assert!(matches!(
            Index::parse(&raw.to_bytes()),
            Err(Error::TypeTally {
                table: 1,
                entries: 2,
                ..
            })
        ));
    }

    #[test]
    fn rejects_names_out_of_raw_order() {
        let mut raw = sample();
        raw.entries.swap(1, 2); // ALPHA, BETA, ALPHA
        assert_eq!(
            Index::parse(&raw.to_bytes()),
            Err(Error::Unsorted { entry: 2 })
        );
    }

    #[test]
    fn sorts_by_bytes_after_the_nul_too() {
        let raw = RawIndex {
            types: vec![(field(b"COL"), 2)],
            reserved: 0,
            codecs: vec![],
            entries: vec![
                RawEntry {
                    name: field(b"ALLWHT\0Y"),
                    kind: field(b"COL"),
                    offset: 0,
                    attr: *b"NONE",
                },
                RawEntry {
                    name: field(b"ALLWHT"),
                    kind: field(b"COL"),
                    offset: 23,
                    attr: *b"NONE",
                },
            ],
        };
        assert_eq!(
            Index::parse(&raw.to_bytes()),
            Err(Error::Unsorted { entry: 1 })
        );
    }

    #[test]
    fn accepts_duplicate_keys() {
        let (raw, _) = build(
            vec![
                stored(b"B47UNDER", b"PCX", b"one"),
                stored(b"B47UNDER", b"PCX", b"two"),
            ],
            &[],
        );
        let index = Index::parse(&raw.to_bytes()).unwrap();
        assert_eq!(index.entries[0].key(), index.entries[1].key());
    }

    proptest! {
        #[test]
        fn arbitrary_bytes_never_panic(bytes in proptest::collection::vec(any::<u8>(), 0..512)) {
            let _ = Index::parse(&bytes);
        }

        #[test]
        fn a_corrupted_word_never_panics(at in 0usize..200, word in any::<u32>()) {
            let mut b = sample().to_bytes();
            let at = (at * 4) % (b.len() - 3);
            b[at..at + 4].copy_from_slice(&word.to_le_bytes());
            let _ = Index::parse(&b);
        }
    }
}
