//! `contracts/bwd.ncl` against dumps from `hgbwd dump --json`. The
//! contract restates the file's invariants in Nickel, apart from the Rust
//! reader, so each checks the other. Needs `nickel` on PATH (the flake's
//! shell and checks provide it).

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use common::{Dir, hgbwd, text, write};
use serde_json::Value;

fn contract() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contracts/bwd.ncl")
}

/// Applies the contract to `dump`: Ok, or Err with nickel's stderr.
fn check(dir: &Path, dump: &Value) -> Result<(), String> {
    let path = dir.join("dump.json");
    std::fs::write(&path, dump.to_string()).unwrap();
    let out = Command::new("nickel")
        .arg("export")
        .arg(&path)
        .arg("--apply-contract")
        .arg(contract())
        .output()
        .expect("nickel must be on PATH");
    if out.status.success() {
        Ok(())
    } else {
        Err(text(&out.stderr))
    }
}

/// What `hgbwd dump --json` prints for `bytes` written to `dir`.
fn dump_of(dir: &Dir, bytes: &[u8]) -> Value {
    let path = dir.path("in.bwd");
    std::fs::write(&path, bytes).unwrap();
    let out = hgbwd(&["dump", "--json", &path]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    serde_json::from_slice(&out.stdout).unwrap()
}

/// A scenario with every bracket kind: a tagged group, then a mangle
/// holding a block.
const SAMPLE: [(&str, &[u8]); 12] = [
    ("REV", b"1.22"),
    ("DTBL", &[0; 20]),
    ("INCL", b"\xff\xffm33_tabl\0\0"),
    ("MUSI", b"\xff\xfftrack2\0\0\0\0\0\0\0\0"),
    ("BTAG", b"trainstr\0\0\0\0\0\0\0\0"),
    ("NAVP", &[7; 72]),
    ("ETAG", &[0; 4]),
    ("MON", b""),
    ("BLK", &[1; 24]),
    ("OBJ", &[2; 60]),
    ("ENDB", b""),
    ("MOFF", b""),
];

fn sample(dir: &Dir) -> Value {
    write(&dir.0, "sample.bwd", &SAMPLE);
    dump_of(dir, &std::fs::read(dir.0.join("sample.bwd")).unwrap())
}

#[test]
fn accepts_the_dump_hgbwd_prints() {
    let dir = Dir::new("contract-ok");
    let d = sample(&dir);
    assert_eq!(check(&dir.0, &d), Ok(()));
}

/// Each broken dump fails, and for the reason the case names.
#[test]
fn rejects_each_broken_invariant_with_its_reason() {
    let dir = Dir::new("contract-broken");
    let good = sample(&dir);
    type Edit = fn(&mut Value);
    let cases: [(&str, Edit, &str); 10] = [
        (
            "gap",
            |d| d["chunks"][3]["offset"] = (72 + 1).into(),
            "MUSI at 73: the previous chunk ends at 72",
        ),
        (
            "trailing",
            |d| d["size"] = (d["size"].as_u64().unwrap() + 4).into(),
            "chunks end at",
        ),
        (
            "unknown tag",
            |d| d["chunks"][5]["tag"] = "XYZ".into(),
            "tag XYZ is not in the engine's table",
        ),
        (
            "short data",
            |d| d["chunks"][5]["data"] = "07".repeat(71).into(),
            "NAVP at 120: size 80 with 71 data bytes",
        ),
        (
            "layout size",
            |d| d["chunks"][5]["tag"] = "INCL".into(),
            "INCL at 120: 80 bytes, its layout has 20",
        ),
        (
            "largest",
            |d| d["largest"] = 99.into(),
            "the header's largest chunk is 99 bytes",
        ),
        (
            "opening",
            |d| d["chunks"][0]["tag"] = "DTBL".into(),
            "the file begins DTBL DTBL, not REV DTBL",
        ),
        (
            "nesting",
            |d| {
                d["chunks"][10]["tag"] = "MOFF".into();
                d["chunks"][11]["tag"] = "ENDB".into();
            },
            "MOFF at 320 closes nothing",
        ),
        (
            "stray etag",
            |d| d["chunks"][4]["tag"] = "MUSI".into(),
            "ETAG at 200 closes nothing",
        ),
        (
            "reopened list",
            |d| d["unclosed_btags"] = serde_json::json!([4]),
            "the dump lists reopened BTAGs [4], the chunks reopen []",
        ),
    ];
    for (name, edit, reason) in cases {
        let mut d = good.clone();
        edit(&mut d);
        match check(&dir.0, &d) {
            Ok(()) => panic!("{name}: accepted"),
            Err(e) => assert!(e.contains(reason), "{name}: wanted {reason:?} in\n{e}"),
        }
    }
}

#[test]
fn a_reopened_btag_must_be_listed() {
    let dir = Dir::new("contract-reopened");
    let mut chunks = SAMPLE.to_vec();
    chunks.insert(5, ("BTAG", b"gouser05\0\0\0\0\0\0\0\0"));
    write(&dir.0, "r.bwd", &chunks);
    let d = dump_of(&dir, &std::fs::read(dir.0.join("r.bwd")).unwrap());
    assert_eq!(d["unclosed_btags"], serde_json::json!([4]));
    assert_eq!(check(&dir.0, &d), Ok(()));
}

/// Shipped files through the contract: the training scenario, which
/// reopens a BTAG, and an object file full of mangles and blocks.
#[test]
#[ignore = "needs the installed game (SUPERGEAR_GAME)"]
fn accepts_shipped_files() {
    let game: PathBuf = std::env::var_os("SUPERGEAR_GAME")
        .expect("set SUPERGEAR_GAME to the installed game directory")
        .into();
    let find = |name: &str| {
        std::fs::read_dir(&game)
            .unwrap()
            .map(|e| e.unwrap().path())
            .find(|p| p.file_name().unwrap().eq_ignore_ascii_case(name))
            .unwrap()
    };
    let map = std::fs::read(find("heavyg.map")).unwrap();
    let prj = std::fs::read(find("heavyg.prj")).unwrap();
    let index = hgpack::Index::parse(&map).unwrap();
    let pack = hgpack::Pack::open(&index, &prj).unwrap();
    let dir = Dir::new("contract-real");
    for key in ["M33_SCN1.BWD", "M01_OBJ1.BWD"] {
        let r = pack
            .records()
            .iter()
            .find(|r| index.entries[r.entry].key() == key)
            .unwrap();
        let d = dump_of(&dir, &pack.decode(r).unwrap());
        assert_eq!(check(&dir.0, &d), Ok(()), "{key}");
    }
}
