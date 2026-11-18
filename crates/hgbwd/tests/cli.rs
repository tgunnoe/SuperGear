//! The `hgbwd` command on synthetic files (see `common` for the writer).

mod common;

use common::{Dir, SCENARIO, TABLE, bwd, hgbwd, text, write};

/// Whether `out` has a line whose words are `words`, spacing aside.
fn has_line(out: &[u8], words: &[&str]) -> bool {
    text(out)
        .lines()
        .any(|l| l.split_whitespace().eq(words.iter().copied()))
}

#[test]
fn dump_lists_chunks_with_their_decoded_names() {
    let dir = Dir::new("dump");
    write(&dir.0, "M33_SCN1.BWD", &SCENARIO);
    let out = hgbwd(&["dump", &dir.path("M33_SCN1.BWD")]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    let o = &out.stdout;
    assert!(
        has_line(o, &["0", "0x000c", "REV", "12", "1.22"]),
        "{}",
        text(o)
    );
    assert!(
        has_line(o, &["2", "0x0034", "INCL", "20", "m33_tabl"]),
        "{}",
        text(o)
    );
    assert!(
        has_line(o, &["3", "0x0048", "MUSI", "24", "track2"]),
        "{}",
        text(o)
    );
    assert!(
        has_line(o, &["4", "0x0060", "BTAG", "24", "trainstr"]),
        "{}",
        text(o)
    );
    assert!(
        has_line(o, &["6", "0x00c8", "ETAG", "12", "0"]),
        "{}",
        text(o)
    );
    let navp = text(o)
        .lines()
        .find(|l| l.contains("NAVP"))
        .unwrap()
        .to_string();
    assert!(navp.contains("07 07 07"), "{navp}");
    assert_eq!(text(o).lines().count(), 7);
}

#[test]
fn dump_json_carries_every_chunk_and_its_bytes() {
    let dir = Dir::new("json");
    write(&dir.0, "a.bwd", &SCENARIO);
    let out = hgbwd(&["dump", "--json", &dir.path("a.bwd")]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["size"], bwd(&SCENARIO).len());
    assert_eq!(v["largest"], 80);
    assert_eq!(v["unclosed_btags"], serde_json::json!([]));
    let c = v["chunks"].as_array().unwrap();
    assert_eq!(c.len(), 7);
    assert_eq!(
        c[0],
        serde_json::json!({"tag": "REV", "offset": 12, "size": 12, "data": "312e3232", "text": "1.22"})
    );
    assert_eq!(c[2]["name"], "m33_tabl");
    assert_eq!(c[2]["slot"], 0xffff);
    assert_eq!(c[4]["name"], "trainstr");
    assert_eq!(c[6]["value"], 0);
    assert_eq!(c[5]["data"], "07".repeat(72));
    assert_eq!(c[5].as_object().unwrap().len(), 4); // raw: no decoded fields
}

#[test]
fn check_accepts_sound_files_and_resolves_includes_among_them() {
    let dir = Dir::new("check-ok");
    write(&dir.0, "M33_SCN1.BWD", &SCENARIO);
    write(&dir.0, "m33_tabl.bwd", &TABLE);
    std::fs::write(dir.0.join("notes.txt"), "not a BWD").unwrap();
    let out = hgbwd(&["check", dir.0.to_str().unwrap()]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    for row in [
        ["files", "2"],
        ["chunks", "10"],
        ["failures", "0"],
        ["unclosed_btags", "0"],
        ["unresolved_includes", "0"],
    ] {
        assert!(
            has_line(&out.stdout, &row),
            "no {row:?} in\n{}",
            text(&out.stdout)
        );
    }
}

#[test]
fn check_notes_reopened_tags_and_missing_includes() {
    let dir = Dir::new("check-notes");
    let mut chunks = SCENARIO.to_vec();
    chunks.insert(5, ("BTAG", b"gouser05\0\0\0\0\0\0\0\0"));
    write(&dir.0, "M33_SCN1.BWD", &chunks);
    let out = hgbwd(&["check", &dir.path("M33_SCN1.BWD")]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    let o = text(&out.stdout);
    assert!(o.contains("trainstr") && o.contains("0x60"), "{o}");
    assert!(o.contains("m33_tabl"), "{o}");
    assert!(has_line(&out.stdout, &["unclosed_btags", "1"]), "{o}");
    assert!(has_line(&out.stdout, &["unresolved_includes", "1"]), "{o}");
}

#[test]
fn check_fails_and_names_a_bad_file() {
    let dir = Dir::new("check-bad");
    write(&dir.0, "GOOD.BWD", &TABLE);
    let mut bad = bwd(&TABLE);
    bad[8] = 99; // the largest-chunk word
    std::fs::write(dir.0.join("BAD.BWD"), bad).unwrap();
    let out = hgbwd(&["check", dir.0.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(1));
    let e = text(&out.stderr);
    assert!(e.contains("BAD.BWD") && e.contains("largest"), "{e}");
    assert!(
        has_line(&out.stdout, &["files", "2"]),
        "{}",
        text(&out.stdout)
    );
    assert!(
        has_line(&out.stdout, &["failures", "1"]),
        "{}",
        text(&out.stdout)
    );
}

#[test]
fn a_missing_path_is_an_error() {
    let dir = Dir::new("missing");
    for args in [["dump", "nope.bwd"], ["check", "nope"]] {
        let path = dir.path(args[1]);
        let out = hgbwd(&[args[0], &path]);
        assert_eq!(out.status.code(), Some(1), "{args:?}");
        assert!(text(&out.stderr).contains(&path), "{}", text(&out.stderr));
    }
}

/// `hgbwd dump | head` must stop quietly when its reader goes away.
#[test]
fn dump_survives_a_closed_pipe() {
    use std::io::Read;
    use std::process::{Command, Stdio};

    let dir = Dir::new("pipe");
    let mut chunks = vec![("REV", b"1.22".as_slice()), ("DTBL", &[0; 20])];
    chunks.extend(std::iter::repeat_n(("OBJ", [1u8; 60].as_slice()), 5000));
    write(&dir.0, "BIG.BWD", &chunks);
    let mut child = Command::new(env!("CARGO_BIN_EXE_hgbwd"))
        .args(["dump", &dir.path("BIG.BWD")])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut first = [0u8; 32];
    child.stdout.take().unwrap().read_exact(&mut first).unwrap();
    let out = child.wait_with_output().unwrap();
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    assert_eq!(text(&out.stderr), "");
}

#[test]
fn bad_usage_exits_2() {
    for args in [&[][..], &["frobnicate"][..], &["dump"][..], &["check"][..]] {
        assert_eq!(hgbwd(args).status.code(), Some(2), "{args:?}");
    }
}
