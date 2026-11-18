//! `contracts/pack.ncl` against manifests from `hgpack list --json`. The
//! contract restates the pack's invariants in Nickel, apart from the Rust
//! reader, so each checks the other. Needs `nickel` on PATH (the flake's
//! shell and checks provide it).

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use common::{Dir, SAMPLE, hgpack, text, write_pack};
use serde_json::Value;

fn contract() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contracts/pack.ncl")
}

/// Applies the contract to `manifest`: Ok, or Err with nickel's stderr.
fn check(dir: &Path, manifest: &Value) -> Result<(), String> {
    let path = dir.join("manifest.json");
    std::fs::write(&path, manifest.to_string()).unwrap();
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

/// The manifest `hgpack list --json` prints for the sample pack.
fn sample(dir: &Dir) -> Value {
    write_pack(&dir.0, &SAMPLE);
    let out = hgpack(&["list", "--json", dir.0.to_str().unwrap()]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    serde_json::from_slice(&out.stdout).unwrap()
}

#[test]
fn accepts_the_manifest_hgpack_prints() {
    let dir = Dir::new("contract-ok");
    let m = sample(&dir);
    assert_eq!(check(&dir.0, &m), Ok(()));
}

/// Each broken manifest fails, and for the reason the case names.
#[test]
fn rejects_each_broken_invariant_with_its_reason() {
    let dir = Dir::new("contract-broken");
    let good = sample(&dir);
    type Edit = fn(&mut Value);
    let cases: [(&str, Edit, &str); 7] = [
        (
            "gap",
            |m| m["entries"][1]["offset"] = (36 + 1).into(),
            "BBB.WAV starts at 37",
        ),
        (
            "trailing",
            |m| m["pack_size"] = (m["pack_size"].as_u64().unwrap() + 2).into(),
            "records end at",
        ),
        (
            "method",
            |m| m["entries"][0]["method"] = 3.into(),
            "AAA.BWD: method 3",
        ),
        (
            "stored size",
            |m| m["entries"][3]["unpacked"] = 99.into(),
            "C/D.PAR: stored",
        ),
        (
            "type sum",
            |m| m["types"][0]["count"] = 2.into(),
            "type counts sum to 5",
        ),
        (
            "type tally",
            |m| m["entries"][0]["type"] = "WAV".into(),
            "type BWD",
        ),
        (
            "shape",
            |m| m["entries"][2]["packed"] = (-1).into(),
            "packed",
        ),
    ];
    for (name, edit, reason) in cases {
        let mut m = good.clone();
        edit(&mut m);
        match check(&dir.0, &m) {
            Ok(()) => panic!("{name}: accepted"),
            Err(e) => assert!(e.contains(reason), "{name}: want {reason:?} in\n{e}"),
        }
    }
}

#[test]
#[ignore = "needs SUPERGEAR_GAME"]
fn accepts_the_shipped_pack() {
    let game =
        std::env::var("SUPERGEAR_GAME").expect("SUPERGEAR_GAME must name the game directory");
    let out = hgpack(&["list", "--json", &game]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    let m: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(m["entries"].as_array().unwrap().len(), 19_545);
    let dir = Dir::new("contract-real");
    assert_eq!(check(&dir.0, &m), Ok(()));
}
