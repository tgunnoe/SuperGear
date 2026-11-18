//! `hgbwd`: dump and check Heavy Gear's `BWD` files.
//!
//! Works on files on disk, such as those `hgpack extract` writes. They are
//! game data and stay out of git (AGENTS.md I1).

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use hgbwd::{Bwd, Chunk, Payload, Tag};

const USAGE: &str = "usage:
  hgbwd dump [--json] <file.bwd>
  hgbwd check <file-or-dir> ...";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let mut out = BufWriter::new(io::stdout().lock());
    let result = match args.as_slice() {
        ["dump", file] => dump(&mut out, file.as_ref(), false),
        ["dump", "--json", file] => dump(&mut out, file.as_ref(), true),
        ["check", paths @ ..] if !paths.is_empty() => check(&mut out, paths),
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    match result.and_then(|code| out.flush().map(|()| code).map_err(Failure::from)) {
        Ok(code) => code,
        // The reader has gone (`hgbwd dump | head`): nothing is left to say.
        Err(Failure::Stdout(e)) if e.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(Failure::Stdout(e)) => {
            eprintln!("hgbwd: stdout: {e}");
            ExitCode::from(1)
        }
        Err(Failure::Message(message)) => {
            eprintln!("hgbwd: {message}");
            ExitCode::from(1)
        }
    }
}

/// Why a command stopped: a message for stderr, or a failed write to
/// stdout, which `main` tells apart from the rest. A bare `io::Error`
/// converts to `Stdout`, so other I/O errors become messages first.
enum Failure {
    Message(String),
    Stdout(io::Error),
}

impl From<String> for Failure {
    fn from(message: String) -> Failure {
        Failure::Message(message)
    }
}

impl From<io::Error> for Failure {
    fn from(e: io::Error) -> Failure {
        Failure::Stdout(e)
    }
}

fn read(path: &Path) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))
}

fn lossy(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

fn hex(b: &[u8], sep: &str) -> String {
    let mut s = String::with_capacity(b.len() * (2 + sep.len()));
    for (i, x) in b.iter().enumerate() {
        if i > 0 {
            s.push_str(sep);
        }
        let _ = write!(s, "{x:02x}");
    }
    s
}

/// The decoded part of a chunk as text: a name, the revision, the `ETAG`
/// word, or for a raw chunk its first 16 bytes in hex.
fn detail(c: &Chunk<'_>) -> String {
    match c.payload {
        Payload::Rev(text) => lossy(text),
        Payload::NameRef { name, .. } | Payload::Btag { name } => lossy(name),
        Payload::Etag(value) => value.to_string(),
        Payload::Marker => String::new(),
        Payload::Raw(data) => hex(&data[..data.len().min(16)], " "),
    }
}

/// A chunk as JSON: tag, offset, size and data, plus its decoded fields.
fn chunk_json(c: &Chunk<'_>) -> serde_json::Value {
    let mut v = serde_json::json!({
        "tag": c.tag.to_string(),
        "offset": c.offset,
        "size": c.size(),
        "data": hex(c.data, ""),
    });
    let decoded = match c.payload {
        Payload::Rev(text) => serde_json::json!({ "text": lossy(text) }),
        Payload::NameRef { slot, name } => serde_json::json!({ "slot": slot, "name": lossy(name) }),
        Payload::Btag { name } => serde_json::json!({ "name": lossy(name) }),
        Payload::Etag(value) => serde_json::json!({ "value": value }),
        Payload::Marker | Payload::Raw(_) => serde_json::json!({}),
    };
    if let (Some(v), serde_json::Value::Object(d)) = (v.as_object_mut(), decoded) {
        v.extend(d);
    }
    v
}

fn dump(out: &mut impl Write, file: &Path, json: bool) -> Result<ExitCode, Failure> {
    let bytes = read(file)?;
    let bwd = Bwd::parse(&bytes).map_err(|e| format!("{}: {e}", file.display()))?;
    if json {
        let v = serde_json::json!({
            "size": bytes.len(),
            "largest": bwd.largest(),
            "unclosed_btags": bwd.unclosed_btags(),
            "chunks": bwd.chunks().iter().map(chunk_json).collect::<Vec<_>>(),
        });
        writeln!(out, "{v}")?;
    } else {
        for (i, c) in bwd.chunks().iter().enumerate() {
            let line = format!(
                "{i:>4} {:#06x} {:<4} {:>6} {}",
                c.offset,
                c.tag,
                c.size(),
                detail(c)
            );
            writeln!(out, "{}", line.trim_end())?;
        }
    }
    Ok(ExitCode::SUCCESS)
}

/// The `BWD` files `path` names: itself, or a directory's `*.bwd` entries
/// (any case), sorted.
fn bwd_files(path: &Path) -> Result<Vec<PathBuf>, String> {
    let meta = std::fs::metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if !meta.is_dir() {
        return Ok(vec![path.to_path_buf()]);
    }
    let mut files: Vec<PathBuf> = std::fs::read_dir(path)
        .map_err(|e| format!("{}: {e}", path.display()))?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|x| x.eq_ignore_ascii_case("bwd")))
        .collect();
    files.sort();
    Ok(files)
}

/// A file's name without its extension, upper-cased: how `INCL` names it.
fn stem(path: &Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().to_uppercase())
        .unwrap_or_default()
}

fn check(out: &mut impl Write, paths: &[&str]) -> Result<ExitCode, Failure> {
    let mut files = Vec::new();
    for p in paths {
        files.extend(bwd_files(p.as_ref())?);
    }
    let present: BTreeSet<String> = files.iter().map(|p| stem(p)).collect();
    let (mut chunks, mut failures, mut unclosed, mut unresolved) = (0, 0, 0, 0);
    for path in &files {
        let bytes = read(path)?;
        let bwd = match Bwd::parse(&bytes) {
            Ok(bwd) => bwd,
            Err(e) => {
                eprintln!("hgbwd: {}: {e}", path.display());
                failures += 1;
                continue;
            }
        };
        chunks += bwd.chunks().len();
        for &i in bwd.unclosed_btags() {
            let c = &bwd.chunks()[i];
            writeln!(
                out,
                "note {}: BTAG {} at {:#x} is followed by another BTAG before an ETAG",
                path.display(),
                detail(c),
                c.offset
            )?;
            unclosed += 1;
        }
        for c in bwd.chunks().iter().filter(|c| c.tag == Tag::INCL) {
            let name = detail(c);
            if !present.contains(&name.to_uppercase()) {
                writeln!(
                    out,
                    "note {}: includes {name}, which is not among the checked files",
                    path.display()
                )?;
                unresolved += 1;
            }
        }
    }
    let rows = [
        ("files", files.len()),
        ("chunks", chunks),
        ("failures", failures),
        ("unclosed_btags", unclosed),
        ("unresolved_includes", unresolved),
    ];
    for (k, v) in rows {
        writeln!(out, "{k:<20} {v}")?;
    }
    Ok(if failures == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}
