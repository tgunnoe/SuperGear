//! Shared by the integration tests: a fixture writer for synthetic packs,
//! independent of the library's own test builder so that a layout mistake
//! shared by the reader and one writer still shows up, and helpers for
//! running the `hgpack` binary. No game data.
#![allow(dead_code)] // each test crate uses a different subset

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// A fresh directory under the system temp dir, removed on drop.
pub struct Dir(pub PathBuf);

impl Dir {
    pub fn new(name: &str) -> Dir {
        let p = std::env::temp_dir().join(format!("hgpack-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        Dir(p)
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Writes `heavyg.map` and `heavyg.prj` (lower case, as the game opens
/// them) for records given as (raw name, type, zlib?, payload), which must
/// already be in raw-name order.
pub fn write_pack(dir: &Path, records: &[(&[u8], &str, bool, &[u8])]) {
    let mut types: Vec<(String, u32)> = Vec::new();
    let (mut map_entries, mut prj) = (Vec::new(), Vec::new());
    for &(name, kind, zlib, payload) in records {
        match types.iter_mut().find(|(k, _)| k == kind) {
            Some((_, n)) => *n += 1,
            None => types.push((kind.to_string(), 1)),
        }
        let mut name8 = [0u8; 8];
        name8[..name.len()].copy_from_slice(name);
        let mut kind4 = [0u8; 4];
        kind4[..kind.len()].copy_from_slice(kind.as_bytes());
        map_entries.extend(name8);
        map_entries.extend(kind4);
        map_entries.extend((prj.len() as u32).to_le_bytes());
        map_entries.extend(b"NONE");

        let mut tag = [0u8; 12];
        let text = [b"C:\\PRJ\\".as_slice(), &name[..1], kind.as_bytes()].concat();
        tag[..text.len()].copy_from_slice(&text);
        let packed = if zlib {
            miniz_oxide::deflate::compress_to_vec_zlib(payload, 6)
        } else {
            payload.to_vec()
        };
        prj.extend(tag);
        prj.extend(if zlib { 2u16 } else { 0 }.to_le_bytes());
        prj.extend((payload.len() as u32).to_le_bytes());
        prj.extend((packed.len() as u32).to_le_bytes());
        prj.extend(packed);
    }
    let mut map = Vec::new();
    map.extend((records.len() as u32).to_le_bytes());
    map.extend((types.len() as u32).to_le_bytes());
    for (kind, n) in &types {
        let mut kind4 = [0u8; 4];
        kind4[..kind.len()].copy_from_slice(kind.as_bytes());
        map.extend(kind4);
        map.extend(n.to_le_bytes());
    }
    map.extend(0u32.to_le_bytes());
    map.extend(2u32.to_le_bytes());
    for dll in [b"pkware.dll".as_slice(), b"zlib.dll"] {
        let mut d = [0u8; 16];
        d[..dll.len()].copy_from_slice(dll);
        map.extend(d);
    }
    map.extend(map_entries);
    std::fs::write(dir.join("heavyg.map"), map).unwrap();
    std::fs::write(dir.join("heavyg.prj"), prj).unwrap();
}

pub const SAMPLE: [(&[u8], &str, bool, &[u8]); 4] = [
    (b"AAA", "BWD", false, b"BWD chunk text"),
    (b"BBB", "WAV", true, b"RIFF....WAVEfmt "),
    (b"BBB", "WAV", false, b"second copy"),
    (b"C/D", "PAR", false, b"# parameters"),
];

pub fn hgpack(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_hgpack"))
        .args(args)
        .output()
        .unwrap()
}

pub fn text(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}
