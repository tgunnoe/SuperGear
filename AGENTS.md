# AGENTS.md — SuperGear

SuperGear reverse engineers **Heavy Gear** (Activision, Windows, 1997) toward
a native reimplementation. This file is the persistent anchor every walk
hydrates from; its contract is predicate's
[boundary skill, §AGENTS.md](https://github.com/nrdxp/predicate/blob/HEAD/skills/boundary/SKILL.md#agentsmd--the-project-scope-boundary),
and the governing invariants are predicate's
[rules.md](https://github.com/nrdxp/predicate/blob/HEAD/rules.md).
Maturity: **molten** (pre-1.0). Refactor and cut freely.

## Goal

The shipped game is specified precisely enough, from its own media and
binaries, that a Rust reimplementation plays the original data on
x86_64-linux and aarch64-linux, with the Wine-hosted original as the
behavioural oracle. Milestones, in order:

1. **Oracle** — the original installed from the owner's disc images and
   running under Wine on this NixOS host from scripts alone. **WIP:** with
   both discs the shell reaches its menus and the training mission's
   simulation starts and renders with the software renderer. Music
   plays from Disc 2's CD audio when the host has CDEmu (spec §7).
   Cutscenes are skipped because their playback stalls Wine's DirectShow
   (U8), and nothing past mission start is verified.
2. **Specification** — [docs/spec/heavy-gear.md](docs/spec/heavy-gear.md)
   describes media, binaries, data formats and runtime behaviour, each
   claim with evidence. **WIP.**
3. **Reimplementation** — Rust crates that read the shipped data and, in
   time, play the game; Nickel carries contracts and configuration. Not
   started.

## Requirements

Each: statement · grounding · signpost (what defeats it).

- **R1 Scripted oracle.** The original runs under Wine from the disc image
  with no manual step. · `scripts/wine-install.sh`, `scripts/wine-run.sh`,
  verified by `scripts/wine-smoke.sh`. · The smoke test fails on a clean
  prefix built from the image.
- **R2 Evidence-backed spec.** Every claim in `docs/spec/` names its
  evidence or is tagged UNVERIFIED. · The spec's own conventions
  paragraph. · A claim with neither.
- **R3 Rust and Nickel.** Project code is Rust; contracts and configuration
  are Nickel; only bootstrap glue under `scripts/` may be shell or Python.
  · Owner's instruction, 2026-10-19. · A source file in another language
  outside `scripts/`, or a configuration format other than Nickel, without
  a decision recorded in the ledger.
- **R4 Two targets.** Everything builds and tests on x86_64-linux and
  aarch64-linux. · `flake.nix` declares both systems; owner's instruction.
  · A dependency without an aarch64 build, or a failing check on aarch64.

## Invariants

- **I1 No game data in git.** Nothing from the discs, the installed tree,
  or the scene release is tracked. · Copyright: Activision published the
  game under licence from Dream Pod 9; neither has released it. ·
  `git ls-files | grep -iE '\.(prj|map|bwd|vdf|exe|dll|iso|mdf|smk|wav|bmp|pcx)$'`
  prints anything.
- **I2 Disc binaries only.** Behaviour is derived from the disc's binaries;
  the scene release's `HGSHELL.DLL` and `HGSIM.DLL` are modified and serve
  only as a pointer to the copy-protection code (spec §6). · SHA-256
  comparison in spec §3 and §6. · A spec claim that cites the scene DLLs'
  code paths.
- **I3 Originals stay unpatched.** The project does not modify the original
  binaries; defeating the disc check is a human decision, not an agent's.
  · Owner's framing of the project as reverse engineering. · A patched
  binary or a hex-edit step anywhere in `scripts/`.

## Constraints

- Every commit passes predicate's Commit Gate (rules.md §3). Commits and
  pushes happen only on the owner's explicit instruction.
- Verification of the oracle is headless where possible (`Xvfb`); see U4
  for the known capture gap.
- Decisions reserved to the owner: anything touching game-data
  distribution, binary patching, the language or configuration choice,
  and legal questions about derived data.

## Unknowns

Each: statement · why open · signpost (what resolves it).

- **U2 PRJ/MAP layout.** Record structure of `HEAVYG.PRJ` and the index in
  `HEAVYG.MAP`. · Only the first record and the type table are decoded. ·
  A Rust parser that enumerates every record and accounts for all
  96,187,673 bytes.
- **U3 Engine lineage.** Whether Interstate '76 / MechWarrior 2 format
  knowledge (Open76, community write-ups) transfers. · Reported by
  secondary sources only. · A byte-level match or mismatch between a
  Heavy Gear format and its claimed ancestor.
- **U4 Headless frames.** The smoke test judges liveness and data loads,
  not pixels. · `ffmpeg -f x11grab` captures Xvfb frames correctly
  (2026-11-09), but the smoke test still captures with ImageMagick
  `import`, which fails on this host. · A non-black frame from
  `scripts/wine-smoke.sh`.
- **U5 aarch64 oracle.** How to run the i386 original on aarch64-linux
  (FEX or box64 with Hangover). · Untested; this host is x86_64. · The
  smoke test passing on an aarch64 host.
- **U6 Official patches.** What versions 1.1 and 1.2 change, and which
  binaries they replace. · Archived zips located but not inspected
  (ledger log, 2026-10-19). · The patch contents hashed and diffed against
  the disc binaries.
- **U7 Legal position.** Who controls the 1997 game today and what derived
  data may be published. · Secondary sources name Activision's 1990s
  licence and Dream Pod 9's ownership of the setting; nothing current. ·
  An owner ruling recorded in the ledger.
- **U8 Cutscene playback.** Why Indeo 5 AVI playback through Wine's
  DirectShow stalls the shell with no further activity (spec §7). ·
  Observed on both displays; `quartz` is disabled as a workaround, so no
  cutscene has played yet. · A cutscene playing to completion under Wine,
  or the blocking call located in a `+quartz,+winegstreamer` trace.

## Spec Pointers

- [docs/spec/heavy-gear.md](docs/spec/heavy-gear.md) — the reverse-engineering specification.
- `.ledger/log/` — flight log (separate git repository, untracked here).
- `.ledger/state/context_map.yaml` — the live tracker hydrated from this file.

## Operational entrypoint

1. Enter the environment: `nix develop` (until the first commit, `nix develop path:.`).
2. Build the oracle: `scripts/wine-install.sh`, then `scripts/wine-run.sh`
   to play or `scripts/wine-smoke.sh` to verify headlessly. Disc 1 is
   read from `~/Downloads/Games/hg/` by default (`SUPERGEAR_MDF`
   overrides); Disc 2 is required for anything past the insert-disc
   screen and is passed as `SUPERGEAR_CD2=<HEAVYG2.mdf>` (both discs are
   in `~/Downloads/HeavyGearISO.zip`, the Internet Archive item
   `HeavyGearISO`). Music needs CDEmu on the host (NixOS
   `programs.cdemu.enable = true`, user in group `cdrom`):
   `scripts/wine-cdaudio.sh`, which the run and smoke scripts call, loads
   Disc 2 into it.
3. Validate before committing: links with predicate's
   `skills/doc-audit/scripts/check_docs.py .`; the tracker with
   `nickel export .ledger/state/context_map.yaml --apply-contract <predicate>/ledger/contracts/context_map_apply.ncl`.
4. Arsenal: predicate's `/spec` for normative statements, `/form` for data
   format models, `/prior-art` before any parser or protocol, `/core` for
   Rust work; Wine's `WINEDEBUG` channels and `winedbg`, `objdump -p`,
   `7z`, `Xvfb` for the oracle.

## Structure

- **Core:** `docs/spec/` (the deliverable), `AGENTS.md` (this anchor).
- **Host integration:** `scripts/` (disc conversion, Wine prefix, smoke
  test), `flake.nix` (toolchain for both targets).
- **Volatile:** `.scratch/` (uncommitted working set), `.ledger/`
  (predicate flight recorder, its own repository).
- **Planned, not present:** a Rust workspace under `crates/` once the first
  format parser is written.
