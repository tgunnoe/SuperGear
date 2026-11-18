//! The `hgpack` command on synthetic packs (see `common` for the fixture).

mod common;

use common::{Dir, SAMPLE, hgpack, text, write_pack};

/// Whether `out` has a line reading `key value`, spacing aside.
fn has_line(out: &[u8], key: &str, value: &str) -> bool {
    text(out)
        .lines()
        .any(|l| l.split_whitespace().eq([key, value]))
}

#[test]
fn verify_accepts_a_sound_pack() {
    let dir = Dir::new("verify-ok");
    write_pack(&dir.0, &SAMPLE);
    let out = hgpack(&["verify", dir.0.to_str().unwrap()]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    for (k, v) in [
        ("records", "4"),
        ("stored", "3"),
        ("zlib", "1"),
        ("pkware", "0"),
        ("failures", "0"),
    ] {
        assert!(
            has_line(&out.stdout, k, v),
            "no {k} {v} in\n{}",
            text(&out.stdout)
        );
    }
}

#[test]
fn verify_fails_and_names_a_corrupt_record() {
    let dir = Dir::new("verify-corrupt");
    write_pack(&dir.0, &SAMPLE);
    let prj_path = dir.0.join("heavyg.prj");
    let mut prj = std::fs::read(&prj_path).unwrap();
    let bbb = 22 + SAMPLE[0].3.len(); // second record's header
    prj[bbb + 22] = 0; // its zlib CMF byte
    std::fs::write(&prj_path, prj).unwrap();
    let out = hgpack(&["verify", dir.0.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        text(&out.stderr).contains("BBB.WAV"),
        "{}",
        text(&out.stderr)
    );
    assert!(
        has_line(&out.stdout, "failures", "1"),
        "{}",
        text(&out.stdout)
    );
}

#[test]
fn verify_fails_on_a_broken_index() {
    let dir = Dir::new("verify-index");
    write_pack(&dir.0, &SAMPLE);
    std::fs::write(dir.0.join("heavyg.map"), [1, 2, 3]).unwrap();
    let out = hgpack(&["verify", dir.0.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        text(&out.stderr).contains("heavyg.map"),
        "{}",
        text(&out.stderr)
    );
}

#[test]
fn list_prints_one_line_per_entry() {
    let dir = Dir::new("list");
    write_pack(&dir.0, &SAMPLE);
    let out = hgpack(&["list", dir.0.to_str().unwrap()]);
    assert!(out.status.success());
    let keys: Vec<String> = text(&out.stdout)
        .lines()
        .map(|l| l.split_whitespace().next().unwrap().to_string())
        .collect();
    assert_eq!(keys, ["AAA.BWD", "BBB.WAV", "BBB.WAV", "C/D.PAR"]);
}

#[test]
fn list_json_carries_tables_and_raw_codes() {
    let dir = Dir::new("json");
    write_pack(&dir.0, &SAMPLE);
    let out = hgpack(&["list", "--json", dir.0.to_str().unwrap()]);
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let prj_len = std::fs::metadata(dir.0.join("heavyg.prj")).unwrap().len();
    assert_eq!(v["pack_size"], prj_len);
    assert_eq!(v["reserved"], 0);
    assert_eq!(v["codecs"], serde_json::json!(["pkware.dll", "zlib.dll"]));
    assert_eq!(
        v["types"][1],
        serde_json::json!({"type": "WAV", "count": 2})
    );
    let e = &v["entries"][1];
    assert_eq!(e["name"], "BBB");
    assert_eq!(e["type"], "WAV");
    assert_eq!(e["offset"], 22 + 14);
    assert_eq!(e["method"], 2);
    assert_eq!(e["unpacked"], 16);
    assert_eq!(e["attr"], "NONE");
    assert_eq!(v["entries"].as_array().unwrap().len(), 4);
}

#[test]
fn extract_writes_decoded_records_with_safe_unique_names() {
    let dir = Dir::new("extract");
    write_pack(&dir.0, &SAMPLE);
    let out_dir = dir.0.join("out");
    let out = hgpack(&[
        "extract",
        dir.0.to_str().unwrap(),
        out_dir.to_str().unwrap(),
    ]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    let read = |n: &str| std::fs::read(out_dir.join(n)).unwrap();
    assert_eq!(read("AAA.BWD"), b"BWD chunk text");
    assert_eq!(read("BBB.WAV"), b"RIFF....WAVEfmt ");
    assert_eq!(read("BBB.WAV~1"), b"second copy");
    assert_eq!(read("C_D.PAR"), b"# parameters");
    assert_eq!(std::fs::read_dir(&out_dir).unwrap().count(), 4);
}

#[test]
fn extract_selects_keys_ignoring_case_and_rejects_unknown_ones() {
    let dir = Dir::new("extract-keys");
    write_pack(&dir.0, &SAMPLE);
    let out_dir = dir.0.join("out");
    let (d, o) = (dir.0.to_str().unwrap(), out_dir.to_str().unwrap());
    let out = hgpack(&["extract", d, o, "aaa.bwd"]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    assert_eq!(std::fs::read_dir(&out_dir).unwrap().count(), 1);
    let out = hgpack(&["extract", d, o, "NOPE.BWD"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(text(&out.stderr).contains("NOPE.BWD"));
}

/// `hgpack list | head` must stop quietly when its reader goes away. The
/// listing is several pipe buffers long, so the command is still writing
/// when the pipe closes.
#[test]
fn list_survives_a_closed_pipe() {
    use std::io::Read;
    use std::process::{Command, Stdio};

    let dir = Dir::new("pipe");
    let names: Vec<Vec<u8>> = (0..5000).map(|i| format!("N{i:05}").into_bytes()).collect();
    let records: Vec<_> = names
        .iter()
        .map(|n| (n.as_slice(), "BWD", false, b"x".as_slice()))
        .collect();
    write_pack(&dir.0, &records);
    let mut child = Command::new(env!("CARGO_BIN_EXE_hgpack"))
        .args(["list", dir.0.to_str().unwrap()])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut first = [0u8; 64];
    child.stdout.take().unwrap().read_exact(&mut first).unwrap(); // the pipe closes as the handle drops here
    let out = child.wait_with_output().unwrap();
    assert!(text(&first).starts_with("N00000.BWD"), "{}", text(&first));
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    assert_eq!(text(&out.stderr), "");
}

#[test]
fn bad_usage_exits_2() {
    for args in [&[][..], &["frobnicate"][..], &["verify"][..]] {
        assert_eq!(hgpack(args).status.code(), Some(2), "{args:?}");
    }
}
