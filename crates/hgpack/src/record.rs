//! `HEAVYG.PRJ`: the records the index points at (spec §5).
//!
//! The pack is the index's records end to end, with nothing between or
//! after them. A record is a 22-byte header and its packed bytes:
//!
//! | Size | Field |
//! | ---: | :--- |
//! | 12 | tag: `C:\PRJ\` + the name's first byte + the type, NUL-padded |
//! | 2 | method: 0 stored, *k* = codec-table entry *k* − 1 |
//! | 4 | unpacked size |
//! | 4 | packed size |
//! | packed size | data |

use crate::codec::Failure;
use crate::index::u32_at;
use crate::{Codec, Entry, Error, Index, Method};

/// Bytes in a record header.
pub const RECORD_HEADER_LEN: usize = 22;

/// One record, located and checked against its entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    /// Position of the record's entry in [`Index::entries`].
    pub entry: usize,
    /// The header's method code, as stored.
    pub code: u16,
    /// What the code resolves to through the codec table.
    pub method: Method,
    pub unpacked: u32,
    pub packed: u32,
    /// Offset of the packed bytes in the pack.
    pub data_offset: u64,
}

/// Totals from decoding every record of a pack.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Report {
    pub records: usize,
    /// Header and packed bytes over all records; equal to the pack's size.
    pub pack_bytes: u64,
    pub unpacked_bytes: u64,
    pub stored: usize,
    pub pkware: usize,
    pub zlib: usize,
    /// Records that failed to decode, in index order.
    pub failures: Vec<Error>,
}

/// A pack opened against its index.
pub struct Pack<'a> {
    bytes: &'a [u8],
    records: Vec<Record>,
}

impl<'a> Pack<'a> {
    /// Locates every entry's record and checks that the records tile the
    /// pack exactly, that each tag matches its entry, and that each method
    /// names a codec this crate can decode. Decoding is left to
    /// [`Pack::decode`].
    pub fn open(index: &Index, bytes: &'a [u8]) -> Result<Pack<'a>, Error> {
        let have = bytes.len() as u64;
        let mut by_offset: Vec<usize> = (0..index.entries.len()).collect();
        by_offset.sort_by_key(|&i| index.entries[i].offset);

        let mut records: Vec<Option<Record>> = vec![None; index.entries.len()];
        let mut end = 0u64;
        for i in by_offset {
            let entry = &index.entries[i];
            let at = u64::from(entry.offset);
            if at != end {
                return Err(Error::Tiling {
                    entry: i,
                    expected: end,
                    actual: at,
                });
            }
            let data_offset = at + RECORD_HEADER_LEN as u64;
            if have < data_offset {
                return Err(Error::Truncated {
                    what: "record header",
                    need: data_offset,
                    have,
                });
            }
            let at = at as usize;
            if bytes[at..at + 12] != tag(entry) {
                return Err(Error::BadTag { entry: i });
            }
            let code = u16::from_le_bytes([bytes[at + 12], bytes[at + 13]]);
            let unpacked = u32_at(bytes, at + 14);
            let packed = u32_at(bytes, at + 18);
            end = data_offset + u64::from(packed);
            if have < end {
                return Err(Error::Truncated {
                    what: "record data",
                    need: end,
                    have,
                });
            }
            records[i] = Some(Record {
                entry: i,
                code,
                method: method(index, i, code)?,
                unpacked,
                packed,
                data_offset,
            });
        }
        if end != have {
            return Err(Error::TrailingBytes {
                at: end,
                len: have - end,
            });
        }
        Ok(Pack {
            bytes,
            // Tiling visited every entry exactly once.
            records: records.into_iter().flatten().collect(),
        })
    }

    /// The records, in index order: `records()[i].entry == i`.
    pub fn records(&self) -> &[Record] {
        &self.records
    }

    /// The record's bytes, unpacked and checked against its header.
    pub fn decode(&self, record: &Record) -> Result<Vec<u8>, Error> {
        let end = record.data_offset + u64::from(record.packed);
        let data = usize::try_from(record.data_offset)
            .ok()
            .zip(usize::try_from(end).ok())
            .and_then(|(start, end)| self.bytes.get(start..end))
            .ok_or(Error::Truncated {
                what: "record data",
                need: end,
                have: self.bytes.len() as u64,
            })?;
        let (entry, expected) = (record.entry, record.unpacked);
        let out = match record.method {
            Method::Stored => data.to_vec(),
            Method::Codec(codec) => codec.decode(data, expected).map_err(|f| match f {
                Failure::Overlong => Error::Overlong { entry, expected },
                Failure::Rejected(detail) => Error::Corrupt {
                    entry,
                    codec,
                    detail,
                },
            })?,
        };
        if out.len() as u64 != u64::from(expected) {
            return Err(Error::SizeMismatch {
                entry,
                expected,
                actual: out.len() as u64,
            });
        }
        Ok(out)
    }

    /// Decodes every record and tallies the result.
    pub fn verify(&self) -> Report {
        let mut report = Report::default();
        for r in &self.records {
            report.records += 1;
            report.pack_bytes += RECORD_HEADER_LEN as u64 + u64::from(r.packed);
            report.unpacked_bytes += u64::from(r.unpacked);
            match r.method {
                Method::Stored => report.stored += 1,
                Method::Codec(Codec::Pkware) => report.pkware += 1,
                Method::Codec(Codec::Zlib) => report.zlib += 1,
            }
            if let Err(e) = self.decode(r) {
                report.failures.push(e);
            }
        }
        report
    }
}

/// The tag an entry's record must carry.
fn tag(entry: &Entry) -> [u8; 12] {
    let mut t = [0; 12];
    let prefix = b"C:\\PRJ\\";
    t[..7].copy_from_slice(prefix);
    t[7] = entry.name.raw()[0];
    let kind = entry.kind.as_bytes();
    t[8..8 + kind.len()].copy_from_slice(kind);
    t
}

/// Resolves a method code through the codec table.
fn method(index: &Index, entry: usize, code: u16) -> Result<Method, Error> {
    if code == 0 {
        return Ok(Method::Stored);
    }
    let dll = index
        .codecs
        .get(usize::from(code) - 1)
        .ok_or(Error::UnknownMethod { entry, code })?;
    Codec::from_dll(dll.as_bytes())
        .map(Method::Codec)
        .ok_or_else(|| Error::UnknownCodec {
            entry,
            dll: dll.to_string(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Codec;
    use crate::testutil::{RawRecord, build, field, stored};
    use proptest::prelude::*;

    const CODECS: [&[u8]; 2] = [b"pkware.dll", b"zlib.dll"];
    const BLAST_VECTOR: [u8; 8] = [0x00, 0x04, 0x82, 0x24, 0x25, 0x8f, 0x80, 0x7f];

    fn zlib(name: &[u8], kind: &[u8], payload: &[u8]) -> RawRecord {
        RawRecord {
            name: field(name),
            kind: field(kind),
            method: 2,
            packed: miniz_oxide::deflate::compress_to_vec_zlib(payload, 6),
            unpacked: payload.len() as u32,
        }
    }

    fn pkware(name: &[u8]) -> RawRecord {
        RawRecord {
            name: field(name),
            kind: field(b"PCX"),
            method: 1,
            packed: BLAST_VECTOR.to_vec(),
            unpacked: 13,
        }
    }

    fn sample() -> Vec<RawRecord> {
        vec![
            stored(b"CCC", b"PAR", b"# sky parameters"),
            zlib(b"AAA", b"WAV", &[1, 2, 3, 4, 5, 6, 7, 8, 9]),
            pkware(b"BBB"),
        ]
    }

    /// The index and pack bytes for `records`.
    fn files(records: Vec<RawRecord>) -> (Vec<u8>, Vec<u8>) {
        let (index, pack) = build(records, &CODECS);
        (index.to_bytes(), pack)
    }

    #[test]
    fn opens_and_decodes_each_method() {
        let (map, prj) = files(sample());
        let index = Index::parse(&map).unwrap();
        let pack = Pack::open(&index, &prj).unwrap();
        let methods: Vec<_> = pack.records().iter().map(|r| r.method).collect();
        assert_eq!(
            methods,
            [
                Method::Codec(Codec::Zlib),
                Method::Codec(Codec::Pkware),
                Method::Stored
            ]
        );
        let bytes: Vec<_> = pack
            .records()
            .iter()
            .map(|r| pack.decode(r).unwrap())
            .collect();
        assert_eq!(bytes[0], [1, 2, 3, 4, 5, 6, 7, 8, 9]);
        assert_eq!(bytes[1], b"AIAIAIAIAIAIA");
        assert_eq!(bytes[2], b"# sky parameters");
        assert!(pack.records().iter().enumerate().all(|(i, r)| r.entry == i));
    }

    #[test]
    fn rejects_a_tag_that_does_not_match_its_entry() {
        let (map, mut prj) = files(sample());
        prj[7] = b'Z'; // first record's initial: AAA -> ZAA
        let index = Index::parse(&map).unwrap();
        assert_eq!(
            Pack::open(&index, &prj).err(),
            Some(Error::BadTag { entry: 0 })
        );
    }

    #[test]
    fn rejects_a_record_that_does_not_start_where_the_last_ended() {
        let (mut raw, prj) = build(sample(), &CODECS);
        raw.entries[1].offset += 1;
        let index = Index::parse(&raw.to_bytes()).unwrap();
        let expected = u64::from(raw.entries[1].offset) - 1;
        assert_eq!(
            Pack::open(&index, &prj).err(),
            Some(Error::Tiling {
                entry: 1,
                expected,
                actual: expected + 1
            })
        );
    }

    #[test]
    fn rejects_bytes_after_the_last_record() {
        let (map, mut prj) = files(sample());
        let at = prj.len() as u64;
        prj.extend([0, 0]);
        let index = Index::parse(&map).unwrap();
        assert_eq!(
            Pack::open(&index, &prj).err(),
            Some(Error::TrailingBytes { at, len: 2 })
        );
    }

    #[test]
    fn rejects_a_pack_cut_inside_a_header_or_data() {
        let (map, prj) = files(sample());
        let index = Index::parse(&map).unwrap();
        for cut in [prj.len() - 1, prj.len() - 20, 10] {
            assert!(
                matches!(
                    Pack::open(&index, &prj[..cut]),
                    Err(Error::Truncated { .. })
                ),
                "cut at {cut}"
            );
        }
    }

    #[test]
    fn rejects_a_method_beyond_the_codec_table() {
        let mut records = sample();
        records[0].method = 3;
        let (map, prj) = files(records);
        let index = Index::parse(&map).unwrap();
        assert_eq!(
            Pack::open(&index, &prj).err(),
            Some(Error::UnknownMethod { entry: 2, code: 3 })
        );
    }

    #[test]
    fn rejects_a_codec_without_a_decoder() {
        let (raw, prj) = build(vec![pkware(b"BBB")], &[b"lzh.dll"]);
        let index = Index::parse(&raw.to_bytes()).unwrap();
        assert_eq!(
            Pack::open(&index, &prj).err(),
            Some(Error::UnknownCodec {
                entry: 0,
                dll: "lzh.dll".into()
            })
        );
    }

    #[test]
    fn decode_checks_the_unpacked_size() {
        let mut records = sample();
        records[0].unpacked = 15; // stored, 16 bytes
        records[1].unpacked = 10; // zlib, 9 bytes
        records[2].unpacked = 12; // pkware, 13 bytes
        let (map, prj) = files(records);
        let index = Index::parse(&map).unwrap();
        let pack = Pack::open(&index, &prj).unwrap();
        let r = pack.records();
        assert_eq!(
            pack.decode(&r[0]).err(),
            Some(Error::SizeMismatch {
                entry: 0,
                expected: 10,
                actual: 9
            })
        );
        assert_eq!(
            pack.decode(&r[1]).err(),
            Some(Error::Overlong {
                entry: 1,
                expected: 12
            })
        );
        assert_eq!(
            pack.decode(&r[2]).err(),
            Some(Error::SizeMismatch {
                entry: 2,
                expected: 15,
                actual: 16
            })
        );
    }

    #[test]
    fn decode_reports_a_corrupt_stream() {
        let (map, mut prj) = files(sample());
        let index = Index::parse(&map).unwrap();
        prj[RECORD_HEADER_LEN] = 0; // AAA's zlib CMF byte: 0x78 -> 0
        let pack = Pack::open(&index, &prj).unwrap();
        assert!(matches!(
            pack.decode(&pack.records()[0]),
            Err(Error::Corrupt {
                entry: 0,
                codec: Codec::Zlib,
                ..
            })
        ));
    }

    #[test]
    fn verify_tallies_methods_bytes_and_failures() {
        let mut records = sample();
        records[0].unpacked = 15;
        let (map, prj) = files(records);
        let index = Index::parse(&map).unwrap();
        let report = Pack::open(&index, &prj).unwrap().verify();
        assert_eq!(report.records, 3);
        assert_eq!(report.pack_bytes, prj.len() as u64);
        assert_eq!(report.unpacked_bytes, 9 + 13 + 15);
        assert_eq!((report.stored, report.pkware, report.zlib), (1, 1, 1));
        assert_eq!(
            report.failures,
            [Error::SizeMismatch {
                entry: 2,
                expected: 15,
                actual: 16
            }]
        );
    }

    #[test]
    fn an_empty_index_opens_an_empty_pack_only() {
        let index = Index::parse(&build(vec![], &CODECS).0.to_bytes()).unwrap();
        assert!(Pack::open(&index, &[]).unwrap().records().is_empty());
        assert!(matches!(
            Pack::open(&index, &[0]),
            Err(Error::TrailingBytes { at: 0, len: 1 })
        ));
    }

    /// A stored or zlib record, with the payload it must decode to.
    fn arb_record() -> impl Strategy<Value = (RawRecord, Vec<u8>)> {
        (
            "[A-Z0-9_]{1,8}",
            prop::sample::select(vec!["WAV", "PCXX", "BWD", "TEX"]),
            any::<bool>(),
            proptest::collection::vec(any::<u8>(), 0..300),
        )
            .prop_map(|(name, kind, packed, payload)| {
                let record = if packed {
                    zlib(name.as_bytes(), kind.as_bytes(), &payload)
                } else {
                    stored(name.as_bytes(), kind.as_bytes(), &payload)
                };
                (record, payload)
            })
    }

    proptest! {
        #[test]
        #[test]
        fn build_then_parse_round_trips(cases in proptest::collection::vec(arb_record(), 0..24)) {
            // build() lays records out in stable raw-name order.
            let mut expected = cases.clone();
            expected.sort_by_key(|(r, _)| r.name);
            let (map, prj) = files(cases.into_iter().map(|(r, _)| r).collect());
            let index = Index::parse(&map).unwrap();
            let pack = Pack::open(&index, &prj).unwrap();
            prop_assert_eq!(pack.records().len(), expected.len());
            for (r, (want, payload)) in pack.records().iter().zip(&expected) {
                let entry = &index.entries[r.entry];
                prop_assert_eq!(entry.name.raw(), &want.name);
                prop_assert_eq!(&entry.kind.0, &want.kind);
                prop_assert_eq!(&pack.decode(r).unwrap(), payload);
            }
            let report = pack.verify();
            prop_assert_eq!(report.pack_bytes, prj.len() as u64);
            prop_assert!(report.failures.is_empty());
        }

        #[test]
        fn arbitrary_pack_bytes_never_panic(prj in proptest::collection::vec(any::<u8>(), 0..400)) {
            let index = Index::parse(&files(sample()).0).unwrap();
            if let Ok(pack) = Pack::open(&index, &prj) {
                let _ = pack.verify();
            }
        }

        #[test]
        fn a_corrupted_byte_never_panics(at in any::<prop::sample::Index>(), byte in any::<u8>()) {
            let (map, mut prj) = files(sample());
            let i = at.index(prj.len());
            prj[i] = byte;
            let index = Index::parse(&map).unwrap();
            if let Ok(pack) = Pack::open(&index, &prj) {
                let _ = pack.verify();
            }
        }
    }
}
