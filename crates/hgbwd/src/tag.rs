//! Chunk tags and the engine's table of them.

use std::fmt;

/// A chunk tag: four bytes, the name NUL-padded (`GPS\0`).
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Tag(pub [u8; 4]);

impl Tag {
    /// The tag spelled `name`, padded with NULs. `name` has at most four
    /// bytes; a longer one fails to compile in a constant.
    pub const fn new(name: &str) -> Tag {
        let b = name.as_bytes();
        let mut t = [0u8; 4];
        let mut i = 0;
        while i < b.len() {
            t[i] = b[i];
            i += 1;
        }
        Tag(t)
    }

    /// The name: the bytes before the first NUL.
    pub fn as_bytes(&self) -> &[u8] {
        let end = self.0.iter().position(|&b| b == 0).unwrap_or(4);
        &self.0[..end]
    }

    /// Whether the engine's tag table lists this tag.
    pub fn is_known(self) -> bool {
        TAGS.contains(&self)
    }

    pub const REV: Tag = Tag::new("REV");
    pub const DTBL: Tag = Tag::new("DTBL");
    pub const BLK: Tag = Tag::new("BLK");
    pub const ENDB: Tag = Tag::new("ENDB");
    pub const MON: Tag = Tag::new("MON");
    pub const MOFF: Tag = Tag::new("MOFF");
    pub const INCL: Tag = Tag::new("INCL");
    pub const MUSI: Tag = Tag::new("MUSI");
    pub const TLUT: Tag = Tag::new("TLUT");
    pub const MLUT: Tag = Tag::new("MLUT");
    pub const BTAG: Tag = Tag::new("BTAG");
    pub const ETAG: Tag = Tag::new("ETAG");
}

impl fmt::Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&String::from_utf8_lossy(self.as_bytes()))
    }
}

impl fmt::Debug for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Tag({self})")
    }
}

/// Every chunk tag the engine knows, in the order of the table both
/// `HGSHELL.DLL` and `HGSIM.DLL` carry (spec §5, "BWD files"). The table
/// begins with `BWD` itself, which is the file's magic rather than a
/// chunk, and is left out here.
pub const TAGS: [Tag; 96] = {
    const NAMES: [&str; 96] = [
        "REV", "DTBL", "PLNT", "PALG", "TERR", "CLIM", "LITE", "VWSP", "VWST", "INIT", "STBL",
        "MTBL", "GTBL", "BMPJ", "BMID", "BSEC", "BMEN", "FPRJ", "HRZM", "GNDM", "SKYM", "PLGO",
        "BLKX", "REPR", "ENDR", "BLK", "ELSB", "ENDB", "OBJ", "ANIM", "SCRG", "THNG", "GP", "CPTF",
        "PITF", "VPTF", "HUDF", "MGDF", "EYEO", "GT", "OBJL", "BTHG", "XPLO", "NAVP", "NAVO",
        "LITO", "TSK", "POS", "ROT", "INCL", "GRP", "GPS", "MOFF", "MON", "GON", "GOFF", "ANM2",
        "STAR", "VIEW", "POFO", "AFFL", "ORDR", "MUSI", "ASND", "LTBL", "PTBL", "FTBL", "HTXT",
        "PDSC", "SDSC", "SUPS", "CPGN", "PILT", "TECH", "NEWS", "CTNT", "DSSR", "MHLT", "WNLT",
        "THLT", "SMHL", "SWNL", "PTLT", "SLST", "FLST", "WTHR", "AMMO", "MLIT", "CAMO", "EQUP",
        "MLUT", "TLUT", "BTAG", "ETAG", "HGXP", "HGBT",
    ];
    let mut tags = [Tag([0; 4]); 96];
    let mut i = 0;
    while i < 96 {
        tags[i] = Tag::new(NAMES[i]);
        i += 1;
    }
    tags
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_has_96_distinct_well_formed_tags() {
        let mut sorted = TAGS.to_vec();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), 96);
        for t in TAGS {
            let name = t.as_bytes();
            assert!((2..=4).contains(&name.len()), "{t:?}");
            assert!(
                name.iter()
                    .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit()),
                "{t:?}"
            );
            assert!(t.0[name.len()..].iter().all(|&b| b == 0), "{t:?}");
        }
    }

    #[test]
    fn new_pads_and_as_bytes_trims() {
        assert_eq!(Tag::new("GPS").0, *b"GPS\0");
        assert_eq!(Tag::new("GPS").as_bytes(), b"GPS");
        assert_eq!(Tag::new("NAVP").as_bytes(), b"NAVP");
        assert_eq!(Tag::new("GT").to_string(), "GT");
    }

    #[test]
    fn known_means_listed() {
        assert!(Tag::BTAG.is_known());
        assert!(Tag::new("GT").is_known());
        assert!(!Tag::new("BWD").is_known());
        assert!(!Tag::new("XXXX").is_known());
        assert!(!Tag(*b"GPS ").is_known());
    }
}
