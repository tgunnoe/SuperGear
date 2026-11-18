//! Every `BWD` file of the installed game: the 705 in the resource pack
//! and the loose ones the game writes. Needs the game, so the tests are
//! ignored by default:
//!
//!     SUPERGEAR_GAME=<game-dir> cargo test -p hgbwd --release -- --include-ignored
//!
//! The expected numbers are the survey of spec §5, "BWD files".

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use hgbwd::{Bwd, Payload, TAGS, Tag};

fn game() -> PathBuf {
    std::env::var_os("SUPERGEAR_GAME")
        .expect("set SUPERGEAR_GAME to the installed game directory")
        .into()
}

/// The entry of `dir` named `name`, ignoring ASCII case.
fn find(dir: &Path, name: &str) -> PathBuf {
    std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.file_name().unwrap().eq_ignore_ascii_case(name))
        .unwrap_or_else(|| panic!("no {name} in {}", dir.display()))
}

/// The game's `BWD` files by upper-case name without the extension, from
/// the pack and loose, plus every pack entry's name by type.
struct Corpus {
    files: BTreeMap<String, Vec<u8>>,
    packed: usize,
    names_by_type: HashMap<String, BTreeSet<String>>,
}

fn corpus() -> Corpus {
    let dir = game();
    let map = std::fs::read(find(&dir, "heavyg.map")).unwrap();
    let prj = std::fs::read(find(&dir, "heavyg.prj")).unwrap();
    let index = hgpack::Index::parse(&map).unwrap();
    let pack = hgpack::Pack::open(&index, &prj).unwrap();
    let mut names_by_type: HashMap<String, BTreeSet<String>> = HashMap::new();
    let mut files = BTreeMap::new();
    for r in pack.records() {
        let e = &index.entries[r.entry];
        let (name, kind) = (e.name.to_string().to_uppercase(), e.kind.to_string());
        names_by_type
            .entry(kind.clone())
            .or_default()
            .insert(name.clone());
        if kind == "BWD" {
            assert!(files.insert(name, pack.decode(r).unwrap()).is_none());
        }
    }
    let packed = files.len();
    for e in std::fs::read_dir(&dir).unwrap() {
        let p = e.unwrap().path();
        let n = p.file_name().unwrap().to_string_lossy().to_uppercase();
        if let Some(stem) = n.strip_suffix(".BWD") {
            let prior = files.insert(stem.to_string(), std::fs::read(&p).unwrap());
            assert!(prior.is_none(), "{n} is both packed and loose");
        }
    }
    Corpus {
        files,
        packed,
        names_by_type,
    }
}

fn text(b: &[u8]) -> String {
    String::from_utf8_lossy(b).to_uppercase()
}

#[test]
#[ignore = "needs the installed game (SUPERGEAR_GAME)"]
fn every_file_parses_and_starts_with_rev_and_dtbl() {
    let c = corpus();
    assert_eq!(c.packed, 705);
    assert_eq!(c.files.len(), 714);
    for (name, bytes) in &c.files {
        let bwd = Bwd::parse(bytes).unwrap_or_else(|e| panic!("{name}: {e}"));
        let tags: Vec<Tag> = bwd.chunks().iter().take(2).map(|ch| ch.tag).collect();
        assert_eq!(tags, [Tag::REV, Tag::DTBL], "{name}");
        assert_eq!(bwd.chunks()[0].payload, Payload::Rev(b"1.22"), "{name}");
    }
}

#[test]
#[ignore = "needs the installed game (SUPERGEAR_GAME)"]
fn tags_have_the_surveyed_sizes() {
    let fixed: BTreeMap<&str, u32> = [
        ("REV", 12),
        ("DTBL", 28),
        ("INCL", 20),
        ("MUSI", 24),
        ("CAMO", 16),
        ("BTAG", 24),
        ("ETAG", 12),
        ("MON", 8),
        ("BLK", 32),
        ("ENDB", 8),
        ("MOFF", 8),
        ("BLKX", 44),
        ("OBJ", 68),
        ("GT", 76),
        ("GPS", 100),
        ("NAVP", 80),
        ("TLUT", 20),
        ("MLUT", 20),
        ("PLNT", 80),
        ("PALG", 328),
        ("CLIM", 32),
        ("LITE", 40),
        ("INIT", 28),
        ("VIEW", 16),
        ("HRZM", 32),
        ("HTXT", 24),
        ("LTBL", 24),
        ("ORDR", 12),
        ("AFFL", 12),
        ("HGXP", 597),
        ("HGBT", 601),
        ("MLIT", 24),
    ]
    .into();
    let variable = ["MTBL", "STAR", "FTBL", "TSK", "PTBL"];
    let mut sizes: BTreeMap<String, BTreeSet<u32>> = BTreeMap::new();
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for bytes in corpus().files.values() {
        for ch in Bwd::parse(bytes).unwrap().chunks() {
            sizes
                .entry(ch.tag.to_string())
                .or_default()
                .insert(ch.size());
            *counts.entry(ch.tag.to_string()).or_default() += 1;
        }
    }
    assert_eq!(sizes.len(), 37);
    for (tag, seen) in &sizes {
        match fixed.get(tag.as_str()) {
            Some(&size) => assert_eq!(seen, &BTreeSet::from([size]), "{tag}"),
            None => assert!(variable.contains(&tag.as_str()) && seen.len() > 1, "{tag}"),
        }
    }
    assert_eq!(counts["OBJ"], 11_959);
    assert_eq!(counts["BTAG"], 2_077);
    assert_eq!(counts["ETAG"], 2_074);
    assert_eq!(counts["MON"], 531);
}

#[test]
#[ignore = "needs the installed game (SUPERGEAR_GAME)"]
fn three_files_leave_one_btag_unclosed() {
    let mut unclosed = BTreeMap::new();
    for (name, bytes) in &corpus().files {
        let bwd = Bwd::parse(bytes).unwrap();
        for &i in bwd.unclosed_btags() {
            let Payload::Btag { name: tag } = bwd.chunks()[i].payload else {
                panic!("{name}: chunk {i} is not a BTAG");
            };
            unclosed.insert(name.clone(), (i, text(tag)));
        }
    }
    assert_eq!(
        unclosed.keys().collect::<Vec<_>>(),
        ["M01_OBJ1", "M03_OBJ1", "M33_SCN1"]
    );
    assert_eq!(unclosed["M33_SCN1"], (41, "GOUSER05".to_string()));
}

#[test]
#[ignore = "needs the installed game (SUPERGEAR_GAME)"]
fn name_references_resolve_except_to_runtime_files() {
    let c = corpus();
    let has = |kind: &str, name: &str| c.names_by_type.get(kind).is_some_and(|s| s.contains(name));
    let mut includes = 0;
    let mut unresolved: BTreeMap<String, usize> = BTreeMap::new();
    for (file, bytes) in &c.files {
        for ch in Bwd::parse(bytes).unwrap().chunks() {
            let Payload::NameRef { slot, name } = ch.payload else {
                continue;
            };
            assert_eq!(slot, 0xffff, "{file}");
            let name = text(name);
            match ch.tag {
                Tag::INCL => {
                    includes += 1;
                    if !c.files.contains_key(&name) {
                        *unresolved.entry(name).or_default() += 1;
                    }
                }
                Tag::TLUT | Tag::MLUT => {
                    assert!(has("TLT", &name) && has("MLT", &name), "{file}: {name}");
                }
                _ => {}
            }
        }
    }
    assert_eq!(includes, 1_719);
    let expected: BTreeMap<String, usize> = [
        ("OBJECT01", 12),
        ("OBJECT02", 9),
        ("OBJECT03", 3),
        ("EN00STAR", 31),
        ("EN01STAR", 31),
        ("EN02STAR", 1),
        ("EN03STAR", 1),
        ("EN04STAR", 1),
        ("EN05STAR", 1),
        ("EN06STAR", 1),
        ("EN07STAR", 1),
    ]
    .into_iter()
    .map(|(n, k)| (n.to_string(), k))
    .collect();
    assert_eq!(unresolved, expected);
}

/// `TAGS` is a transcription; both DLLs must hold it, after `BWD`, as
/// one run of four-byte entries.
#[test]
#[ignore = "needs the installed game (SUPERGEAR_GAME)"]
fn the_tag_table_matches_both_dlls() {
    let needle: Vec<u8> = std::iter::once(Tag::new("BWD"))
        .chain(TAGS)
        .flat_map(|t| t.0)
        .collect();
    for dll in ["hgshell.dll", "hgsim.dll"] {
        let bytes = std::fs::read(find(&game(), dll)).unwrap();
        let hits = bytes.windows(needle.len()).filter(|w| *w == needle).count();
        assert_eq!(hits, 1, "{dll}");
    }
}
