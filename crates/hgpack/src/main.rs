//! `hgpack`: list, extract and verify Heavy Gear's resource pack.
//!
//! Reads `HEAVYG.MAP` and `HEAVYG.PRJ` from a game directory, matching
//! their names without regard to case. Extracted files go wherever the
//! caller says; they are game data and stay out of git (AGENTS.md I1).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use hgpack::{Codec, Index, Method, Pack};

const USAGE: &str = "usage:
  hgpack list [--json] <game-dir>
  hgpack extract <game-dir> <out-dir> [NAME.TYPE ...]
  hgpack verify <game-dir>";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let result = match args.as_slice() {
        ["list", dir] => list(dir.as_ref(), false),
        ["list", "--json", dir] => list(dir.as_ref(), true),
        ["extract", dir, out, keys @ ..] => extract(dir.as_ref(), out.as_ref(), keys),
        ["verify", dir] => verify(dir.as_ref()),
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(code) => code,
        Err(message) => {
            eprintln!("hgpack: {message}");
            ExitCode::from(1)
        }
    }
}

/// The two pack files of a game directory, read and the index parsed.
struct Game {
    index: Index,
    prj: Vec<u8>,
    prj_path: PathBuf,
}

impl Game {
    fn load(dir: &Path) -> Result<Game, String> {
        let map_path = find(dir, "heavyg.map")?;
        let prj_path = find(dir, "heavyg.prj")?;
        let read = |p: &Path| std::fs::read(p).map_err(|e| format!("{}: {e}", p.display()));
        let index =
            Index::parse(&read(&map_path)?).map_err(|e| format!("{}: {e}", map_path.display()))?;
        let prj = read(&prj_path)?;
        Ok(Game {
            index,
            prj,
            prj_path,
        })
    }

    fn pack(&self) -> Result<Pack<'_>, String> {
        Pack::open(&self.index, &self.prj).map_err(|e| self.explain(e))
    }

    /// An error with the entry it names spelled as `NAME.TYPE`.
    fn explain(&self, e: hgpack::Error) -> String {
        let entry = match &e {
            hgpack::Error::BadTag { entry }
            | hgpack::Error::Tiling { entry, .. }
            | hgpack::Error::UnknownMethod { entry, .. }
            | hgpack::Error::UnknownCodec { entry, .. }
            | hgpack::Error::SizeMismatch { entry, .. }
            | hgpack::Error::Overlong { entry, .. }
            | hgpack::Error::Corrupt { entry, .. } => Some(*entry),
            _ => None,
        };
        match entry.and_then(|i| self.index.entries.get(i)) {
            Some(en) => format!("{}: {}: {e}", self.prj_path.display(), en.key()),
            None => format!("{}: {e}", self.prj_path.display()),
        }
    }
}

/// The entry of `dir` whose name is `name` ignoring ASCII case.
fn find(dir: &Path, name: &str) -> Result<PathBuf, String> {
    std::fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find(|p| p.file_name().is_some_and(|n| n.eq_ignore_ascii_case(name)))
        .ok_or_else(|| format!("{}: no {name}", dir.display()))
}

fn method_name(m: Method) -> &'static str {
    match m {
        Method::Stored => "stored",
        Method::Codec(Codec::Pkware) => "pkware",
        Method::Codec(Codec::Zlib) => "zlib",
    }
}

fn list(dir: &Path, json: bool) -> Result<ExitCode, String> {
    let game = Game::load(dir)?;
    let pack = game.pack()?;
    let index = &game.index;
    if json {
        let entries: Vec<_> = pack
            .records()
            .iter()
            .map(|r| {
                let e = &index.entries[r.entry];
                serde_json::json!({
                    "name": e.name.to_string(),
                    "type": e.kind.to_string(),
                    "offset": e.offset,
                    "method": r.code,
                    "unpacked": r.unpacked,
                    "packed": r.packed,
                    "attr": String::from_utf8_lossy(&e.attr),
                })
            })
            .collect();
        let manifest = serde_json::json!({
            "pack_size": game.prj.len(),
            "reserved": index.reserved,
            "codecs": index.codecs.iter().map(|c| c.to_string()).collect::<Vec<_>>(),
            "types": index
                .types
                .iter()
                .map(|t| serde_json::json!({ "type": t.kind.to_string(), "count": t.count }))
                .collect::<Vec<_>>(),
            "entries": entries,
        });
        println!("{manifest}");
    } else {
        for r in pack.records() {
            let e = &index.entries[r.entry];
            println!(
                "{:<13} {:>9} {:<6} {:>8} {:>8}",
                e.key(),
                e.offset,
                method_name(r.method),
                r.packed,
                r.unpacked
            );
        }
    }
    Ok(ExitCode::SUCCESS)
}

/// A file name for `key` that cannot leave the output directory.
fn file_name(key: &str) -> String {
    key.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || "_-.".contains(c) {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn extract(dir: &Path, out: &Path, keys: &[&str]) -> Result<ExitCode, String> {
    let game = Game::load(dir)?;
    let pack = game.pack()?;
    let index = &game.index;
    let wanted = |key: &str| keys.is_empty() || keys.iter().any(|k| k.eq_ignore_ascii_case(key));
    if let Some(missing) = keys.iter().find(|k| {
        !index
            .entries
            .iter()
            .any(|e| e.key().eq_ignore_ascii_case(k))
    }) {
        return Err(format!("no entry {missing}"));
    }
    std::fs::create_dir_all(out).map_err(|e| format!("{}: {e}", out.display()))?;
    let mut seen: HashMap<String, usize> = HashMap::new();
    let (mut written, mut failed) = (0usize, 0usize);
    for r in pack.records() {
        let key = index.entries[r.entry].key();
        if !wanted(&key) {
            continue;
        }
        let base = file_name(&key);
        let copy = seen.entry(base.clone()).or_default();
        let name = match *copy {
            0 => base,
            n => format!("{base}~{n}"),
        };
        *copy += 1;
        match pack.decode(r) {
            Ok(bytes) => {
                let path = out.join(&name);
                std::fs::write(&path, bytes).map_err(|e| format!("{}: {e}", path.display()))?;
                written += 1;
            }
            Err(e) => {
                eprintln!("hgpack: {}", game.explain(e));
                failed += 1;
            }
        }
    }
    println!("extracted {written} records to {}", out.display());
    Ok(if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

fn verify(dir: &Path) -> Result<ExitCode, String> {
    let game = Game::load(dir)?;
    let report = game.pack()?.verify();
    for e in &report.failures {
        eprintln!("hgpack: {}", game.explain(e.clone()));
    }
    let rows = [
        ("records", report.records as u64),
        ("pack_size", game.prj.len() as u64),
        ("accounted", report.pack_bytes),
        ("unpacked", report.unpacked_bytes),
        ("stored", report.stored as u64),
        ("pkware", report.pkware as u64),
        ("zlib", report.zlib as u64),
        ("failures", report.failures.len() as u64),
    ];
    for (k, v) in rows {
        println!("{k:<10} {v}");
    }
    Ok(if report.failures.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}
