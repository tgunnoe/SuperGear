//! Shared by the integration tests: a `BWD` writer independent of the
//! library's own test builder, so that a layout mistake shared by the
//! reader and one writer still shows up, and helpers for running the
//! `hgbwd` binary. No game data.
#![allow(dead_code)] // each test crate uses a different subset

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// A fresh directory under the system temp dir, removed on drop.
pub struct Dir(pub PathBuf);

impl Dir {
    pub fn new(name: &str) -> Dir {
        let p = std::env::temp_dir().join(format!("hgbwd-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        Dir(p)
    }

    pub fn path(&self, name: &str) -> String {
        self.0.join(name).to_str().unwrap().to_string()
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// The bytes of a `BWD` file holding `chunks` as (tag, data).
pub fn bwd(chunks: &[(&str, &[u8])]) -> Vec<u8> {
    let mut body = Vec::new();
    let mut largest = 0u32;
    for (tag, data) in chunks {
        let mut t = [0u8; 4];
        t[..tag.len()].copy_from_slice(tag.as_bytes());
        let size = 8 + data.len() as u32;
        largest = largest.max(size);
        body.extend(t);
        body.extend(size.to_le_bytes());
        body.extend(*data);
    }
    let mut out = b"BWD\0".to_vec();
    out.extend((12 + body.len() as u32).to_le_bytes());
    out.extend(largest.to_le_bytes());
    out.extend(body);
    out
}

pub fn write(dir: &Path, name: &str, chunks: &[(&str, &[u8])]) {
    std::fs::write(dir.join(name), bwd(chunks)).unwrap();
}

/// A small scenario: revision, table, an include of `M33_TABL`, music,
/// and one tagged group.
pub const SCENARIO: [(&str, &[u8]); 7] = [
    ("REV", b"1.22"),
    ("DTBL", &[0; 20]),
    ("INCL", b"\xff\xffm33_tabl\0\0"),
    ("MUSI", b"\xff\xfftrack2\0\0\0\0\0\0\0\0"),
    ("BTAG", b"trainstr\0\0\0\0\0\0\0\0"),
    ("NAVP", &[7; 72]),
    ("ETAG", &[0; 4]),
];

pub const TABLE: [(&str, &[u8]); 3] = [
    ("REV", b"1.22"),
    ("DTBL", &[0; 20]),
    ("TLUT", b"\xff\xffgearxpl1\0\0"),
];

pub fn hgbwd(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_hgbwd"))
        .args(args)
        .output()
        .unwrap()
}

pub fn text(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}
