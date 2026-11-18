//! The bracket chunks: `MON`…`MOFF` and `BLK`…`ENDB`, which nest, and
//! `BTAG`…`ETAG`, which in the shipped files never do.

use crate::{Chunk, Error, Tag};

/// Checks the brackets of `chunks` and returns the indices of the `BTAG`
/// chunks that another `BTAG` followed before any `ETAG`.
///
/// `MON`/`MOFF` and `BLK`/`ENDB` must nest like parentheses. A `BTAG` may
/// follow an open `BTAG`, as in three shipped files; whether the engine
/// then nests or replaces is open (spec §5), so the earlier one is
/// reported rather than rejected. `ELSB`, `REPR` and `ENDR` read like
/// brackets too, but no shipped file has one, so they are not checked.
pub(crate) fn check(chunks: &[Chunk<'_>]) -> Result<Vec<usize>, Error> {
    let mut open: Vec<&Chunk<'_>> = Vec::new();
    let mut btag: Option<usize> = None;
    let mut unclosed = Vec::new();
    for (i, c) in chunks.iter().enumerate() {
        let opener = match c.tag {
            Tag::MON | Tag::BLK => {
                open.push(c);
                continue;
            }
            Tag::MOFF => Tag::MON,
            Tag::ENDB => Tag::BLK,
            Tag::BTAG => {
                unclosed.extend(btag.replace(i));
                continue;
            }
            Tag::ETAG => match btag.take() {
                Some(_) => continue,
                None => return Err(unopened(c)),
            },
            _ => continue,
        };
        match open.pop() {
            Some(o) if o.tag == opener => {}
            _ => return Err(unopened(c)),
        }
    }
    if let Some(o) = open.last() {
        return Err(Error::Unclosed {
            at: o.offset,
            tag: o.tag,
        });
    }
    if let Some(i) = btag {
        return Err(Error::Unclosed {
            at: chunks[i].offset,
            tag: Tag::BTAG,
        });
    }
    Ok(unclosed)
}

fn unopened(c: &Chunk<'_>) -> Error {
    Error::Unopened {
        at: c.offset,
        tag: c.tag,
    }
}

#[cfg(test)]
mod tests {
    use crate::testutil::{build, data};
    use crate::{Bwd, Error, Tag};

    /// Parses a file of `tags`, each with the data its layout needs, and
    /// gives the unclosed `BTAG` indices or the error.
    fn brackets(tags: &[&str]) -> Result<Vec<usize>, Error> {
        let chunks: Vec<_> = tags
            .iter()
            .map(|t| (Tag::new(t), data(Tag::new(t))))
            .collect();
        let bytes = build(&chunks);
        Bwd::parse(&bytes).map(|b| b.unclosed_btags().to_vec())
    }

    /// Byte offset of chunk `i` in a file of `tags`.
    fn offset(tags: &[&str], i: usize) -> u32 {
        let sizes = tags.iter().take(i).map(|t| 8 + data(Tag::new(t)).len());
        (12 + sizes.sum::<usize>()) as u32
    }

    #[test]
    fn blocks_nest_inside_mangles() {
        let ok = [
            "REV", "MON", "BLK", "OBJ", "BLK", "OBJ", "ENDB", "ENDB", "MOFF",
        ];
        assert_eq!(brackets(&ok), Ok(vec![]));
    }

    #[test]
    fn a_closer_must_match_the_innermost_opener() {
        let t = ["MON", "BLK", "MOFF", "ENDB"];
        assert_eq!(
            brackets(&t),
            Err(Error::Unopened {
                at: offset(&t, 2),
                tag: Tag::MOFF
            })
        );
        let t = ["OBJ", "ENDB"];
        assert_eq!(
            brackets(&t),
            Err(Error::Unopened {
                at: offset(&t, 1),
                tag: Tag::ENDB
            })
        );
    }

    #[test]
    fn an_opener_left_open_is_rejected() {
        let t = ["MON", "BLK", "ENDB"];
        assert_eq!(
            brackets(&t),
            Err(Error::Unclosed {
                at: offset(&t, 0),
                tag: Tag::MON
            })
        );
        let t = ["BTAG", "OBJ"];
        assert_eq!(
            brackets(&t),
            Err(Error::Unclosed {
                at: offset(&t, 0),
                tag: Tag::BTAG
            })
        );
    }

    #[test]
    fn tags_close_in_turn_and_a_reopened_one_is_reported() {
        assert_eq!(
            brackets(&["BTAG", "OBJ", "ETAG", "BTAG", "ETAG"]),
            Ok(vec![])
        );
        let t = ["BTAG", "NAVP", "BTAG", "NAVP", "ETAG", "BTAG", "ETAG"];
        assert_eq!(brackets(&t), Ok(vec![0]));
    }

    #[test]
    fn an_etag_without_a_btag_is_rejected() {
        let t = ["BTAG", "ETAG", "ETAG"];
        assert_eq!(
            brackets(&t),
            Err(Error::Unopened {
                at: offset(&t, 2),
                tag: Tag::ETAG
            })
        );
    }

    #[test]
    fn tag_groups_and_blocks_are_independent() {
        let t = ["BTAG", "BLKX", "MON", "BLK", "OBJ", "ENDB", "MOFF", "ETAG"];
        assert_eq!(brackets(&t), Ok(vec![]));
    }

    #[test]
    fn unshipped_bracket_tags_are_not_checked() {
        assert_eq!(brackets(&["ELSB", "REPR", "ENDR", "ENDR"]), Ok(vec![]));
    }
}
