//! Acceptance against the shipped pack (spec §5). The game data is not in
//! the repository (AGENTS.md I1), so these tests read the installed game
//! named by `SUPERGEAR_GAME` and are ignored by default:
//!
//! ```text
//! SUPERGEAR_GAME=~/.local/share/supergear/wine/drive_c/Games/HeavyGear \
//!   cargo test -p hgpack --test real_pack -- --ignored
//! ```

use std::collections::{BTreeMap, BTreeSet};

use hgpack::{Codec, Error, Index, Method, Pack};

struct Game {
    map: Vec<u8>,
    prj: Vec<u8>,
}

fn game() -> Game {
    let dir = std::env::var_os("SUPERGEAR_GAME")
        .expect("SUPERGEAR_GAME must name the installed game directory");
    let dir = std::path::Path::new(&dir);
    Game {
        map: std::fs::read(dir.join("HEAVYG.MAP")).expect("HEAVYG.MAP"),
        prj: std::fs::read(dir.join("HEAVYG.PRJ")).expect("HEAVYG.PRJ"),
    }
}

#[test]
#[ignore = "needs SUPERGEAR_GAME"]
fn every_record_is_accounted_for_and_decodes() {
    let g = game();
    assert_eq!(g.map.len(), 391_108);
    assert_eq!(g.prj.len(), 96_187_673);
    let index = Index::parse(&g.map).unwrap();
    assert_eq!(index.entries.len(), 19_545);
    assert_eq!(index.types.len(), 20);
    assert_eq!(index.reserved, 0);
    let codecs: Vec<_> = index.codecs.iter().map(|c| c.to_string()).collect();
    assert_eq!(codecs, ["pkware.dll", "zlib.dll"]);
    assert!(index.entries.iter().all(|e| &e.attr == b"NONE"));

    let pack = Pack::open(&index, &g.prj).unwrap();
    let report = pack.verify();
    assert_eq!(report.failures, []);
    assert_eq!(report.records, 19_545);
    assert_eq!(report.pack_bytes, 96_187_673);
    assert_eq!(
        (report.stored, report.pkware, report.zlib),
        (2_302, 1_419, 15_824)
    );
}

#[test]
#[ignore = "needs SUPERGEAR_GAME"]
fn methods_per_type_are_as_specified() {
    let g = game();
    let index = Index::parse(&g.map).unwrap();
    let pack = Pack::open(&index, &g.prj).unwrap();
    let mut codes: BTreeMap<String, BTreeSet<u16>> = BTreeMap::new();
    for r in pack.records() {
        let kind = index.entries[r.entry].kind.to_string();
        codes.entry(kind).or_default().insert(r.code);
    }
    let using = |want: &[u16]| -> Vec<&str> {
        codes
            .iter()
            .filter(|(_, s)| s.iter().eq(want))
            .map(|(t, _)| t.as_str())
            .collect()
    };
    assert_eq!(using(&[0]), ["FNT", "PAR"]);
    assert_eq!(using(&[1]), ["PCX", "PCXX"]);
    assert_eq!(
        using(&[2]),
        ["3DI", "COL", "GDF", "SDF", "TBL", "TEX", "WAV", "WAVX"]
    );
    assert_eq!(
        using(&[0, 2]),
        ["BRF", "BWD", "MLT", "ODF", "SHP", "TLT", "WTB", "XEL"]
    );
}

#[test]
#[ignore = "needs SUPERGEAR_GAME"]
fn duplicate_keys_and_bytes_after_the_nul_are_as_observed() {
    let index = Index::parse(&game().map).unwrap();
    let mut keys: BTreeMap<String, usize> = BTreeMap::new();
    for e in &index.entries {
        *keys.entry(e.key()).or_default() += 1;
    }
    let dups: Vec<_> = keys.into_iter().filter(|(_, n)| *n > 1).collect();
    let want = [
        ("AVOM204F.WAV", 2),
        ("AVOSCDEC.WAVX", 2),
        ("B20UNDER.PCX", 2),
        ("B47UNDER.PCX", 6),
        ("B48UNDER.PCX", 6),
        ("O_1CRGZW.SDF", 2),
    ];
    assert_eq!(dups, want.map(|(k, n)| (k.to_string(), n)));
    let trailing = index
        .entries
        .iter()
        .filter(|e| {
            e.name.raw()[e.name.as_bytes().len()..]
                .iter()
                .any(|&b| b != 0)
        })
        .count();
    assert_eq!(trailing, 33);
}

#[test]
#[ignore = "needs SUPERGEAR_GAME"]
fn a_corrupted_record_is_reported_by_name() {
    let mut g = game();
    let index = Index::parse(&g.map).unwrap();
    // First zlib record and first PKWare record: break each stream header.
    let (zlib, pkware) = {
        let pack = Pack::open(&index, &g.prj).unwrap();
        let first = |c| {
            pack.records()
                .iter()
                .find(|r| r.method == Method::Codec(c))
                .unwrap()
                .clone()
        };
        (first(Codec::Zlib), first(Codec::Pkware))
    };
    g.prj[zlib.data_offset as usize] = 0; // zlib CMF
    g.prj[pkware.data_offset as usize + 1] = 9; // DCL dictionary size
    let pack = Pack::open(&index, &g.prj).unwrap();
    let failed: Vec<_> = pack
        .verify()
        .failures
        .iter()
        .map(|e| match e {
            Error::Corrupt { entry, codec, .. } => (index.entries[*entry].key(), *codec),
            other => panic!("unexpected {other}"),
        })
        .collect();
    assert_eq!(
        failed,
        [
            (index.entries[zlib.entry].key(), Codec::Zlib),
            (index.entries[pkware.entry].key(), Codec::Pkware),
        ]
    );
}
