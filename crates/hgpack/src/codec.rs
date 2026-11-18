//! Record methods and the codecs behind them (spec §5).
//!
//! A record's method is 0 for stored bytes or *k* for the codec-table entry
//! *k* − 1. The shipped table names `pkware.dll` (PKWare Data Compression
//! Library "implode") and `zlib.dll`; both export `Compress`, `GetKey` and
//! `Uncompress`.

use std::fmt;

/// A decoder the codec table can name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Codec {
    /// `pkware.dll`: PKWare DCL implode.
    Pkware,
    /// `zlib.dll`: a zlib stream (RFC 1950).
    Zlib,
}

/// How a record's bytes are packed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Method {
    Stored,
    Codec(Codec),
}

/// Why a codec gave up on a stream.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Failure {
    /// The stream decodes to more bytes than the record header allows.
    Overlong,
    /// The codec rejected the stream.
    Rejected(String),
}

impl Codec {
    /// The codec a codec-table DLL name stands for, ignoring ASCII case.
    pub fn from_dll(name: &[u8]) -> Option<Codec> {
        if name.eq_ignore_ascii_case(b"pkware.dll") {
            Some(Codec::Pkware)
        } else if name.eq_ignore_ascii_case(b"zlib.dll") {
            Some(Codec::Zlib)
        } else {
            None
        }
    }

    /// Decodes `packed`, producing at most `limit` bytes: a stream that
    /// would produce more fails with [`Failure::Overlong`], so a hostile
    /// record cannot exhaust memory.
    pub(crate) fn decode(self, packed: &[u8], limit: u32) -> Result<Vec<u8>, Failure> {
        let limit = limit as usize;
        match self {
            Codec::Pkware => explode_limited(packed, limit, &mut [0; 4096]),
            Codec::Zlib => miniz_oxide::inflate::decompress_to_vec_zlib_with_limit(packed, limit)
                .map_err(|e| match e.status {
                    miniz_oxide::inflate::TINFLStatus::HasMoreOutput => Failure::Overlong,
                    status => Failure::Rejected(format!("{status:?}")),
                }),
        }
    }
}

/// `explode::explode`, stopping once the output passes `limit` bytes.
/// `window` is the decoder's output buffer, drained each time it fills.
fn explode_limited(packed: &[u8], limit: usize, window: &mut [u8]) -> Result<Vec<u8>, Failure> {
    let mut decoder = explode::Explode::new();
    let mut buf = decoder.with_buffer(window);
    let mut out = Vec::new();
    let mut at = 0;
    while let Some(&byte) = packed.get(at) {
        match buf.feed(byte) {
            // The window is full or the stream has ended. The decoder has
            // not finished with `byte`: it must be fed again.
            Ok(()) => {
                out.extend_from_slice(buf.get());
                if out.len() > limit {
                    return Err(Failure::Overlong);
                }
                if buf.done() {
                    return Ok(out);
                }
                buf.reset();
            }
            Err(explode::Error::IncompleteInput) => at += 1,
            Err(e) => return Err(Failure::Rejected(format!("{e:?}"))),
        }
    }
    Err(Failure::Rejected("stream ends before its end code".into()))
}

impl fmt::Display for Codec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Codec::Pkware => "pkware",
            Codec::Zlib => "zlib",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    /// The test vector shipped with blast.c (zlib contrib/blast).
    const BLAST_VECTOR: [u8; 8] = [0x00, 0x04, 0x82, 0x24, 0x25, 0x8f, 0x80, 0x7f];

    #[test]
    fn dll_names_match_ignoring_case() {
        assert_eq!(Codec::from_dll(b"pkware.dll"), Some(Codec::Pkware));
        assert_eq!(Codec::from_dll(b"ZLIB.DLL"), Some(Codec::Zlib));
        assert_eq!(Codec::from_dll(b"lzh.dll"), None);
    }

    #[test]
    fn pkware_decodes_the_blast_vector() {
        assert_eq!(
            Codec::Pkware.decode(&BLAST_VECTOR, 13).unwrap(),
            b"AIAIAIAIAIAIA"
        );
    }

    #[test]
    fn pkware_drains_a_window_smaller_than_the_output() {
        // A one-byte window fills on every output byte, as the real 4 KiB
        // window does on every record larger than it.
        assert_eq!(
            explode_limited(&BLAST_VECTOR, 13, &mut [0; 1]).unwrap(),
            b"AIAIAIAIAIAIA"
        );
    }

    #[test]
    fn pkware_stops_past_the_limit() {
        assert_eq!(
            Codec::Pkware.decode(&BLAST_VECTOR, 12),
            Err(Failure::Overlong)
        );
    }

    #[test]
    fn pkware_rejects_a_cut_stream_and_a_bad_header() {
        assert!(matches!(
            Codec::Pkware.decode(&BLAST_VECTOR[..5], 13),
            Err(Failure::Rejected(_))
        ));
        assert!(matches!(
            Codec::Pkware.decode(&[0x00, 0x09, 0x82, 0x24], 13),
            Err(Failure::Rejected(_))
        ));
    }

    #[test]
    fn zlib_stops_past_the_limit() {
        let packed = miniz_oxide::deflate::compress_to_vec_zlib(&[7; 100], 6);
        assert_eq!(Codec::Zlib.decode(&packed, 99), Err(Failure::Overlong));
        assert_eq!(Codec::Zlib.decode(&packed, 100).unwrap(), [7; 100]);
    }

    #[test]
    fn zlib_rejects_garbage() {
        assert!(matches!(
            Codec::Zlib.decode(b"\x78\x9cnot deflate", 100),
            Err(Failure::Rejected(_))
        ));
    }

    proptest! {
        #[test]
        fn zlib_round_trips(payload in proptest::collection::vec(any::<u8>(), 0..2048)) {
            let packed = miniz_oxide::deflate::compress_to_vec_zlib(&payload, 6);
            prop_assert_eq!(Codec::Zlib.decode(&packed, payload.len() as u32).unwrap(), payload);
        }

        #[test]
        fn arbitrary_streams_never_panic(bytes in proptest::collection::vec(any::<u8>(), 0..256)) {
            let _ = Codec::Pkware.decode(&bytes, 4096);
            let _ = Codec::Zlib.decode(&bytes, 4096);
        }
    }
}
