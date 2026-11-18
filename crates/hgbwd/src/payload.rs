//! Chunk data for the tags whose layout the shipped files settle.

use crate::Tag;

/// What a chunk's data decodes to. Only the layouts that spec §5 shows
/// from the shipped files are decoded; every other tag stays raw.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Payload<'a> {
    /// `REV`: the format revision as text, `1.22` in every shipped file.
    Rev(&'a [u8]),
    /// `INCL`, `MUSI`, `TLUT`, `MLUT`: a u16 slot, 0xFFFF in every shipped
    /// file, and a NUL-padded name (here without its padding).
    NameRef { slot: u16, name: &'a [u8] },
    /// `BTAG`: a NUL-padded 16-byte name for the chunks up to the `ETAG`.
    Btag { name: &'a [u8] },
    /// `ETAG`: one word, 0 in every shipped file.
    Etag(u32),
    /// `MON`, `MOFF`, `ENDB`: no data.
    Marker,
    /// Any other tag: its data as stored.
    Raw(&'a [u8]),
}

/// The bytes of `field` before its first NUL.
fn trim(field: &[u8]) -> &[u8] {
    let end = field.iter().position(|&b| b == 0).unwrap_or(field.len());
    &field[..end]
}

/// The data size a decoded tag's layout fixes, or `None` for a raw tag.
pub(crate) fn fixed_size(tag: Tag) -> Option<usize> {
    Some(match tag {
        Tag::REV | Tag::ETAG => 4,
        Tag::INCL | Tag::TLUT | Tag::MLUT => 12,
        Tag::MUSI | Tag::BTAG => 16,
        Tag::MON | Tag::MOFF | Tag::ENDB => 0,
        _ => return None,
    })
}

/// Decodes `data` as `tag`'s layout. A decoded tag whose data has the
/// wrong size is an error carrying the size its layout fixes.
pub(crate) fn decode(tag: Tag, data: &[u8]) -> Result<Payload<'_>, usize> {
    let Some(size) = fixed_size(tag) else {
        return Ok(Payload::Raw(data));
    };
    if data.len() != size {
        return Err(size);
    }
    Ok(match tag {
        Tag::REV => Payload::Rev(data),
        Tag::ETAG => Payload::Etag(u32::from_le_bytes([data[0], data[1], data[2], data[3]])),
        Tag::BTAG => Payload::Btag { name: trim(data) },
        Tag::MON | Tag::MOFF | Tag::ENDB => Payload::Marker,
        // The rest of `fixed_size`'s tags: INCL, MUSI, TLUT, MLUT.
        _ => Payload::NameRef {
            slot: u16::from_le_bytes([data[0], data[1]]),
            name: trim(&data[2..]),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_references_are_a_slot_and_a_padded_name() {
        let incl = b"\xff\xffm33_tabl\0\0";
        assert_eq!(
            decode(Tag::INCL, incl),
            Ok(Payload::NameRef {
                slot: 0xffff,
                name: b"m33_tabl"
            })
        );
        let musi = b"\x02\x00track2\0\0\0\0\0\0\0\0";
        assert_eq!(
            decode(Tag::MUSI, musi),
            Ok(Payload::NameRef {
                slot: 2,
                name: b"track2"
            })
        );
        for tag in [Tag::TLUT, Tag::MLUT] {
            assert_eq!(
                decode(tag, b"\xff\xffgearxpl1\0\0"),
                Ok(Payload::NameRef {
                    slot: 0xffff,
                    name: b"gearxpl1"
                })
            );
        }
    }

    #[test]
    fn a_full_name_field_has_no_nul() {
        assert_eq!(
            decode(Tag::BTAG, b"0123456789abcdef"),
            Ok(Payload::Btag {
                name: b"0123456789abcdef"
            })
        );
    }

    #[test]
    fn fixed_layouts_decode() {
        assert_eq!(decode(Tag::REV, b"1.22"), Ok(Payload::Rev(b"1.22")));
        assert_eq!(
            decode(Tag::BTAG, b"trainstr\0\0\0\0\0\0\0\0"),
            Ok(Payload::Btag { name: b"trainstr" })
        );
        assert_eq!(decode(Tag::ETAG, &[1, 0, 0, 0]), Ok(Payload::Etag(1)));
        for tag in [Tag::MON, Tag::MOFF, Tag::ENDB] {
            assert_eq!(decode(tag, b""), Ok(Payload::Marker));
        }
    }

    #[test]
    fn a_wrong_size_names_the_layout_size() {
        assert_eq!(decode(Tag::REV, b"1.2"), Err(4));
        assert_eq!(decode(Tag::INCL, &[0xff; 13]), Err(12));
        assert_eq!(decode(Tag::MUSI, &[0xff; 12]), Err(16));
        assert_eq!(decode(Tag::MON, b"x"), Err(0));
    }

    #[test]
    fn other_tags_stay_raw() {
        let data = [7u8; 92];
        assert_eq!(decode(Tag::new("GPS"), &data), Ok(Payload::Raw(&data)));
        assert_eq!(decode(Tag::BLK, &data), Ok(Payload::Raw(&data)));
        assert_eq!(decode(Tag::new("ELSB"), b""), Ok(Payload::Raw(b"")));
    }
}
