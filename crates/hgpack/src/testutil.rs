//! Test-only writer for synthetic packs. Every byte it produces is made up
//! by the tests; no game data is involved (AGENTS.md I1).

/// A NUL-padded fixed-width field holding `s`.
pub fn field<const N: usize>(s: &[u8]) -> [u8; N] {
    let mut f = [0; N];
    f[..s.len()].copy_from_slice(s);
    f
}

pub struct RawEntry {
    pub name: [u8; 8],
    pub kind: [u8; 4],
    pub offset: u32,
    pub attr: [u8; 4],
}

/// An index laid out field by field, consistent or not as a test needs.
pub struct RawIndex {
    pub types: Vec<([u8; 4], u32)>,
    pub reserved: u32,
    pub codecs: Vec<[u8; 16]>,
    pub entries: Vec<RawEntry>,
}

impl RawIndex {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend((self.entries.len() as u32).to_le_bytes());
        b.extend((self.types.len() as u32).to_le_bytes());
        for (kind, count) in &self.types {
            b.extend(kind);
            b.extend(count.to_le_bytes());
        }
        b.extend(self.reserved.to_le_bytes());
        b.extend((self.codecs.len() as u32).to_le_bytes());
        for c in &self.codecs {
            b.extend(c);
        }
        for e in &self.entries {
            b.extend(e.name);
            b.extend(e.kind);
            b.extend(e.offset.to_le_bytes());
            b.extend(e.attr);
        }
        b
    }
}

/// One record to lay out in a synthetic pack.
#[derive(Clone, Debug)]
pub struct RawRecord {
    pub name: [u8; 8],
    pub kind: [u8; 4],
    pub method: u16,
    pub packed: Vec<u8>,
    pub unpacked: u32,
}

/// The record tag the game writes: `C:\PRJ\` + the name's first byte + the
/// type, NUL-padded to 12 bytes.
pub fn tag(name: &[u8; 8], kind: &[u8; 4]) -> [u8; 12] {
    let mut s = b"C:\\PRJ\\".to_vec();
    s.push(name[0]);
    s.extend(kind.iter().take_while(|&&c| c != 0));
    field(&s)
}

/// A consistent index and its pack: entries sorted by raw name (stable),
/// records laid out in that order, the type table tallied in order of
/// first appearance.
pub fn build(mut records: Vec<RawRecord>, codecs: &[&[u8]]) -> (RawIndex, Vec<u8>) {
    records.sort_by_key(|r| r.name);
    let mut types: Vec<([u8; 4], u32)> = Vec::new();
    let mut entries = Vec::new();
    let mut pack = Vec::new();
    for r in &records {
        match types.iter_mut().find(|(k, _)| *k == r.kind) {
            Some((_, n)) => *n += 1,
            None => types.push((r.kind, 1)),
        }
        entries.push(RawEntry {
            name: r.name,
            kind: r.kind,
            offset: pack.len() as u32,
            attr: *b"NONE",
        });
        pack.extend(tag(&r.name, &r.kind));
        pack.extend(r.method.to_le_bytes());
        pack.extend(r.unpacked.to_le_bytes());
        pack.extend((r.packed.len() as u32).to_le_bytes());
        pack.extend(&r.packed);
    }
    let index = RawIndex {
        types,
        reserved: 0,
        codecs: codecs.iter().map(|c| field(c)).collect(),
        entries,
    };
    (index, pack)
}

/// A stored (method 0) record.
pub fn stored(name: &[u8], kind: &[u8], payload: &[u8]) -> RawRecord {
    RawRecord {
        name: field(name),
        kind: field(kind),
        method: 0,
        packed: payload.to_vec(),
        unpacked: payload.len() as u32,
    }
}
