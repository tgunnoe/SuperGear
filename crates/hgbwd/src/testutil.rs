//! Builders for synthetic `BWD` files in the unit tests. No game data.

use crate::Tag;
use crate::payload::fixed_size;

/// A file of `chunks`, its header's size and largest-chunk words filled in.
pub(crate) fn build(chunks: &[(Tag, Vec<u8>)]) -> Vec<u8> {
    let largest = chunks.iter().map(|(_, d)| 8 + d.len()).max().unwrap_or(0);
    let mut body = Vec::new();
    for (tag, data) in chunks {
        body.extend(tag.0);
        body.extend(((8 + data.len()) as u32).to_le_bytes());
        body.extend(data);
    }
    let mut file = Vec::new();
    file.extend(b"BWD\0");
    file.extend(((12 + body.len()) as u32).to_le_bytes());
    file.extend((largest as u32).to_le_bytes());
    file.extend(body);
    file
}

/// Data that `tag`'s layout accepts: zeros of its fixed size, or three
/// bytes for a raw tag.
pub(crate) fn data(tag: Tag) -> Vec<u8> {
    vec![0; fixed_size(tag).unwrap_or(3)]
}
