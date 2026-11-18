# Heavy Gear (Activision, 1997): reverse-engineering specification

**Status: WIP.** This document records what has been established about the
shipped game by direct inspection of the media and binaries and by tracing
the game under Wine. Every statement names its evidence. A statement marked
**UNVERIFIED** is a lead, not a fact, and must not be built on until
verified. The AGENTS.md Unknowns section tracks the open questions.

Evidence conventions: `iso:` paths are on Disc 1; `game:` paths are in the
installed tree; traces are Wine 11.0 `WINEDEBUG` logs produced by
`scripts/wine-smoke.sh` or ad-hoc runs with the channels named.

## 1. Media

### Disc 1, "Install"

| Property | Value | Evidence |
| :--- | :--- | :--- |
| Volume identifier | `HEAVYG` | ISO 9660 primary volume descriptor, sector 16 |
| Mastering | CeQuadrat 32bit ISO-9660 Formatter, 1997-11-21 | same descriptor |
| Size | 113915 sectors × 2048 B = 233,297,920 B | `7z l`; `scripts/mdf2iso.py` sector count |
| Source image | Alcohol `.mdf`/`.mds`, 1 session, 1 Mode 1 track, 2448 B raw sectors (2352 + 96 subchannel) | `.mds` header; sync pattern at every sector |

Top-level layout:

| Entry | Role |
| :--- | :--- |
| `AUTORUN.INF`, `SETUP.EXE` | 16-bit NE splash launcher (`Setup.exe` → `SPLASH/`) |
| `SPLASH/` | Launcher UI (`SPLASH.EXE`, `REGISTER.EXE`, `ERRATA.EXE`, bitmaps, WAVs, `3D.TXT`) |
| `INSTALL/` | InstallShield 3 setup: NE `SETUP.EXE` stub, `_INST32I.EX_`, `SETUP.INS`, `DATA1.CAB` (262 KB), `DATA/` payload |
| `INSTALL/DATA/` | The game, 145 files, stored uncompressed (see §2) |
| `DIRECTX/` | DirectX 5 redistributable plus vendor display and audio drivers |
| `DRIVERS/` | 3dfx Voodoo, Voodoo Rush and Rendition V1000 reference drivers |
| `HELP/` | WinHelp files and `README.DOC` |
| `ZGIDEMO/` | Zork Grand Inquisitor demo |

### Disc 2, "Gameplay CD"

Required at runtime. The prompt string lives in `HG.EXE` at file offset
`0x92bd` ("Please insert Heavy Gear CD 2: Gameplay CD into your CD-ROM
drive.") and in `HGSIM.DLL` at `0x14ba95`; `HGSHELL.DLL` carries the
two-line variant at `0x23261c`.

Image: `HEAVYG2.mdf`/`.mds` from the Internet Archive item `HeavyGearISO`
(uploaded 2019-03-05; its Disc 1 image is byte-identical to the owner's).
Mixed-mode CD, 245056 raw sectors of 2448 bytes, one session, twelve
tracks (from the `.mds` track table and the sync-pattern scan):

| Track | Type | Start sector | Length | Content |
| ---: | :--- | ---: | ---: | :--- |
| 1 | Mode 1 data | 0 | 91193 sectors (data ends at 91043) | ISO 9660 volume `HEAVYG`, mastered 1997-12-22, 23 files |
| 2–12 | CD audio | 91193 … 231075 | 13681–14514 sectors each (3.0–3.2 min) | Soundtrack, eleven tracks, 34.2 min |

Data track contents: `AUTORUN.INF` (`open=Splash.exe`), `SPLASH.EXE`,
`SPLASH.DAT`, `HGEAR.ICO`, and `VIDEO\` with twenty AVI files
(186 MB): `B51MOVIE.AVI` (intro, 320×240), `NDEATH.AVI`, `SDEATH.AVI`,
`ZL_CUT01.AVI` … `ZL_CUT16.AVI` (320×192). All are Indeo Video 5
(`IV50`) at 15 fps with 11025 Hz mono 8-bit PCM audio.

The shell's disc test is the presence of `VIDEO\`; the simulation's is the
existence of `VIDEO\B51MOVIE.AVI` (§4). The soundtrack being Redbook
audio means the oracle needs a CD-ROM device under Wine for music; see §4
for how the game picks tracks and §7 for the virtual drive.

## 2. Installed tree

`INSTALL/DATA` copied verbatim is a complete installation: the InstallShield
script only copies files (`strings INSTALL/DATA1.CAB` shows directory
tokens and the install-type names `Zero-Install` and `Typical`). The scene
release of 1997-11-25 confirms the layout a real installation produces: the
chosen language's `ACTION?/*` and `SIM?/CURRENT.CCT` promoted to the game
root, plus `HELP/` and `SPLASH/` copied from the disc.

Root files (sizes in bytes, from the disc):

| File | Size | Role (evidence) |
| :--- | ---: | :--- |
| `HG.EXE` | 60928 | Launcher, see §3 |
| `HGSHELL.DLL` | 2523136 | Front end / menus |
| `HGSIM.DLL` | 1540096 | Simulation |
| `HEAVYG.PRJ` | 96187673 | Resource pack, see §5 |
| `HEAVYG.MAP` | 391108 | Resource index, see §5 |
| `ANET2.DLL` | 57280 | Activision ANet 2 networking |
| `DKUPDDLL.DLL` | 72192 | Patch download client (`UPDATE.TXT` strings, `SERVERS.DAT` host list) |
| `FFORCE.DLL` | 51200 | Force feedback (`HG_*_F.FRC` are RIFF `FORC` effect files) |
| `KEYBOARD.DLL` | 77312 | Keyboard layout per `LANG.INI` (`dll = keyboard.dll`) |
| `PKWARE.DLL`, `ZLIB.DLL`, `SMACKW32.DLL`, `MSVCRT.DLL` | | Vendor libraries: PKWare compression, zlib, RAD Smacker video, MS C runtime |
| `GEAR.VDF`, `MAMMOTH.VDF`, `NAGA.VDF` | | Vehicle definition files (loose) |
| `*.MAP` (`KEYBOARD`, `MOUSE`, `JOYSTICK`, `MSJSTICK`, `FLTSTCK`, `GAMEKEY`, `INPUTW`) | | Text input maps |
| `HG.INI` | 4958 | `[SystemError]` table, 79 codes, see §5 |
| `CACHE.INI` | 709 | Per-resource-type hash and cache budgets, see §5 |
| `LANG.INI`, `LANGUAGE.INI` | | Language letter (x/s/g) and names |
| `SHELLDLL.INI` | 13 | `i76draw.dll`: the shell's renderer plugin |
| `DLLFILE.INI` | 14 | `3DFXGEAR.DLL`: the sim's preferred renderer plugin |
| `SHELLNET.INI`, `COOKIE.INI`, `ANET.INF`, `SERVERS.DAT`, `FEIF.DAT` | | Network configuration (`heavygear.activision.com`, master-server list) |
| `B20UNDER.PCX`, `SKYGND.PAR` | | Loose art and sky parameters |

Subdirectories: `ACTION{E,F,I,S}/`, `SIM{E,F,I,S}/` (per-language
`ACTION.CCT`, `CURRENT.CCT`, `SIM.CCT`, `INPUTW.MAP`); `ANET/` (ANet
transports `WINETS2`, `WIPX2`, `WMODEM2`, `WSERIAL2`); `PLUGIN/` (four
renderer DLLs, §3); `SMK/` (16 Smacker videos `B02…B15*.SMK`); `VFX/`
(1994 `MCGA.DLL`, `VESA480.DLL`, `VESA768.DLL`); `CFGS/` (joystick
profiles); `GIDDIWIN/`; `INSTALL/`.

## 3. Executables

All are PE32 for Windows 4.0, i386. SHA-256 of the disc copies:

| File | Linked | SHA-256 |
| :--- | :--- | :--- |
| `HG.EXE` | 1997-11-17 | `1e1735f5a5989f0136da17d811660ff7bc5371c78c7a82cd55efb74b82568187` |
| `HGSHELL.DLL` | 1997-11-18 | `d84aa95eef169e0bf9c36be339487e110485fecc5c907017a38f7dde6e5a1c89` |
| `HGSIM.DLL` | 1997-11-17 | `02a9c05a86827685624e9af6ee71cb536a0d69b368d09b1241796ef65cd1db32` |
| `PLUGIN/3DFXGEAR.DLL` | 1997-11-11 | `f077a431d6f7fac37fc66a1072ffc61cc5730febd873ddfae12f2385b7a499a7` |
| `PLUGIN/DX5DRAW.DLL` | 1997-11-09 | `f5563731408357d9239537e53637ff69ac241433d31509ab52216b99556c67e8` |
| `PLUGIN/I76DRAW.DLL` | 1997-11-10 | `0b660689e43a89d19338d21c3764fc12b5eab897988ca5f7b83c9a73c2740a72` |
| `PLUGIN/RENDDRAW.DLL` | 1997-11-09 | `cdc694fc041b2ab0309bc2898da8aa56293dd310042cf7b13361620ac0955bbd` |

Imports (`objdump -p`):

| Module | Imports |
| :--- | :--- |
| `HG.EXE` | KERNEL32, USER32, GDI32; loads `HGSHELL.DLL`, `HGSIM.DLL`, `ANET2.DLL` by name at runtime |
| `HGSHELL.DLL` | USER32, WINMM, DDRAW, DSOUND, KERNEL32, GDI32, ADVAPI32, WSOCK32, anet2, smackw32, ole32, MSVCRT |
| `HGSIM.DLL` | WINMM, DDRAW, DSOUND, KERNEL32, USER32, GDI32, ADVAPI32, ole32 |
| `3DFXGEAR.DLL` | glide2x, KERNEL32 |
| `DX5DRAW.DLL` | DDRAW, KERNEL32 |
| `I76DRAW.DLL` | KERNEL32 |
| `RENDDRAW.DLL` | KERNEL32, USER32; loads `verite.dll` and probes `kernel32!IsTNT` at runtime |

`HG.EXE` strings (`strings -t x`) name the launcher's responsibilities:
display-depth checks ("requires a color palette of 256 colors"), the
single-instance guard ("Heavy Gear is already running"), language file
resolution (`lang.ini`, `shell.txt`), the CD 2 prompt, and the two entry
points it drives: `ShellWindowProc` / "Entering Shell..." and `SimMain` /
`SimWindowProc` / "Entering Sim..." with "Returned from Sim, retval=%d".
`HGSHELL.DLL` embeds the source path `C:\Project\HeavyGear\Mem\Cache.c`.
`HGSIM.DLL` reads the environment variable `HEAVYGEARPATH`.

## 4. Runtime behaviour observed under Wine

Startup order from a `+loaddll,+file` trace: `HG.EXE` → `DKUPDDLL.dll` →
`ANET2.DLL` → `smackw32.DLL` → `HGSHELL.DLL` → `plugin\i76draw.dll` →
`keyboard.dll` → `pkware.dll` → `zlib.dll`. Files read before the first
frame: `nmsave.cfg` and `hgprm.cfg` (absent on a fresh install),
`lang.ini`, `shelldll.ini`, `shellnet.ini`, `anet/*.dll` (via
`FindFirstFileA("anet/*.dll")`), the CD-ROM root, `heavyg.map`,
`heavyg.prj`, `cache.ini`, then fonts and tile sets.

Working directory: the shell opens `lang.ini`, `shell.txt`,
`shelldll.ini`, `shellnet.ini`, `nmsave.cfg`, `hgprm.cfg`, `heavyg.map`,
`where`, `cache.ini` and the `anet\` and `plugin\` directories by bare
name, so they resolve against the process's working directory, which
`HG.EXE` inherits from its launcher and never changes (neither `HG.EXE`
nor `HGSHELL.DLL` imports `SetCurrentDirectoryA`; the only
`RtlSetCurrentDirectory_U` in the game process is Wine's own at start).
Started from another directory the shell probes every one of those names
there (`warn+file`: `Z:\home\…\heavyg.map not found`), stops loading
after `cache.ini` (no font probes, no `pkware.dll`/`zlib.dll` load; only
`DKUPDDLL`, `ANET2`, `smackw32`, `HGSHELL`, `keyboard` are loaded) while
still presenting front-buffer updates, and on the owner's desktop crashed
one second after launch. That crash: `Unhandled page fault on read access
to 00000000 at address 01E154C2`, `HGSHELL.DLL` loaded at `01C90000`
(`+loaddll`), so RVA `0x1854C2` = `idiv dword [ecx]` in the cache-release
routine at RVA `0x185484`, where `ecx` is the per-type table returned by
the type lookup (RVA `0x1859C0`, list head `0x250B54`) with no null check.
The backtrace runs `ShellMain` (`0x16FE90`) → `0x128819` → `0x133402`
(loads `gear.vdf`, `naga.vdf`, `mammoth.vdf`) → `0x131CB1` → `0x12C9A5`
(resource `SA11`, type `ODF`) → `0x185484`. Whether the headless stall
and the desktop crash diverge at the same point is **UNVERIFIED**; both
disappear when the game directory is the working directory, which
`scripts/wine-run.sh` and `scripts/wine-smoke.sh` now ensure.

Loose-file override: for each resource the shell first tries
`<NAME>.<TYPE>X` then `<NAME>.<TYPE>` in the game directory (observed for
`F0E0607A.FNTX`/`.FNT`, `ANTELOPE.TLTX`/`.TLT`, and every other font and
tile set) before reading the pack. Types ending in `X` are the compressed
variants (`PCXX`, `WAVX` in the §5 type table).

Renderer plugins: the shell loads `i76draw.dll` (software), then
enumerates `plugin\*.dll` with `FindFirstFileA` and probes each: `3DFXGEAR`
(fails without `glide2x.dll`), `DX5DRAW` (creates a DirectDraw object,
queries caps, releases), `RENDDRAW` (fails without `verite.dll`).

Video: DirectDraw 1 interface. `SetCooperativeLevel(EXCLUSIVE | FULLSCREEN |
ALLOWMODEX)`, `SetDisplayMode(640, 480, 8)`, three surfaces (primary and
two 640×480 `DDSCAPS_OFFSCREENPLAIN | DDSCAPS_SYSTEMMEMORY`
`DDPF_PALETTEINDEXED8`), one palette. Render loop: `Lock`/`Unlock` an
offscreen surface, `BltFast` to the primary.

Audio: DirectSound at `DSSCL_EXCLUSIVE`; Wine mixes 480-frame periods.

Disc check (relay trace of calls from `HGSHELL.DLL`):
`GetLogicalDriveStringsA`, then `GetDriveTypeA` on each drive; for every
drive of type `DRIVE_CDROM` (5) the shell calls
`GetFileAttributesW(L"<X>:\\video\\")`. Disc 2 therefore carries a
`VIDEO\` directory, and the test is that directory's existence, not a
volume label. When no drive passes, the shell draws "Please insert Heavy
Gear CD 2." in its own DirectDraw surface (screen art `B57UNDER.SHP`,
`B57_TEXT.PCX`, `B57CONTR.PCX`; not a Win32 message box) and repeats the
scan every few seconds. The scan runs from two call sites in
`HGSHELL.DLL`; the second returns to relocated address `0x01d8a06b`,
which maps to file offset 1021035, four bytes before the first patched
byte of the second crack cluster in §6.

With a `VIDEO\` directory present (an empty one suffices for the test),
the shell opens `<X>:\video\B51MOVIE.AVI`, continues when it is absent,
and shows the main menu: *story*, *tour of duty*, *training*,
*multiplayer*, *exit to windows*, beside an animated gear render. With
Disc 2 present the intro movie is played through DirectShow: Wine loads
`quartz.dll`, `msvfw32.dll` and `msacm32.dll` right after the file is
opened (traced 2026-10-22).

Simulation launch (*training* → briefing "Gear Piloting", terrain Arctic,
call-sign Cadet → *launch*): `HG.EXE` loads `HGSIM.DLL`, which reads
`lang.ini`, probes `sim.txt`, reads `hgsnd.cfg` and `HG.INI`, then stops
with a Win32 message box titled "Heavy Gear Fatal Error": "HG.EXE - fatal
error #53: SYSERR_CDNOTFOUND: CD-ROM not mounted.: Please insert Heavy
Gear CD 2: Gameplay CD into your CD-ROM drive." The simulation's own disc
test (relay trace of calls from `HGSIM.DLL`): `GetLogicalDriveStringsA`,
`GetDriveTypeA` per drive, then `FindFirstFileA("<X>:\video\B51MOVIE.AVI")`
on the CD-ROM drive; the file must exist. Before the test the sim scans
the process environment for `HEAVYGEARPATH` (a `CompareStringA` per
variable) and sets `FX_GLIDE_NO_SPLASH=1`; after a failed test it reads
`HG.INI` twice to format the message. Error 0x53 is outside the
0x00–0x4e range listed in `HG.INI`. A zero-byte `B51MOVIE.AVI` is not a
substitute: with one present the shell goes black at the intro-movie
step and never reaches the menu (observed for 50 s).

With Disc 2 present and DirectShow disabled (§7), the simulation starts
(traced 2026-10-22 under Xvfb): `HGSIM.DLL` reads `dllfile.ini` and loads
`plugin\I76DRAW.DLL`, the software renderer, creates its own DirectDraw
object with `SetCooperativeLevel(EXCLUSIVE | FULLSCREEN | ALLOWMODEX)`,
`SetDisplayMode(640, 480, 8)`, two surfaces and one palette
(`CreatePalette` flags `0x44`), loads `mcicda.dll` (music is MCI CD
audio), and renders continuously (tens of thousands of front-buffer
updates within 80 s, process alive). The training mission's data is
read through the loose-file probe and then the pack, in this order:
`LAUNCH_H.SHP`, `TERR0001.SHP`, `M33_SCN1.BWD`, `M33_TABL.BWD`, then
tile sets as `.TLT` followed by `.MLT` for `GEARXPL1`, `GENERICS`,
`WEAPONS1`, `TERRAI33`, `MAMMOTH1`, `HUNTER_1`, `OFUKLEMM`, `OFUKALLR`,
`OBACCHUS`, `BOULDERS`, `OHELIPAD`, `SCORPION`, `OBHANGER`, `OUPLNKBB`,
`OOBSTC19`, `FIVETON1`, `OBSTACLE`, then `FORMATNS.BWD`, `GDTRAINR.GDF`,
`GD14TRHP.WTB`, `GDPEPEFF.XEL`, `GCHEADTP.XEL`, `GD13TRHP.WTB`. Mission
resources follow the pattern `M<nn>_SCN<n>.BWD` (scenario) with
`M<nn>_TABL.BWD` and terrain tiles `TERRAI<nn>`; the training mission is
`M33`, matching `hgprm.cfg`. Frames grabbed from Xvfb with
`ffmpeg -f x11grab` show the menu, the Tour of Duty hub and briefing, and
a running mission with its HUD (2026-11-09, §7).

Music (traced 2026-11-09 with `+mci,+mcicda` and with `+relay` limited to
calls made from `HGSIM.DLL`, Disc 2 in a CDEmu virtual drive, §7): at
mission start the simulation opens MCI `cdaudio` (`MCI_SYSINFO` for
device type 516, `MCI_OPEN`), sets `MCI_FORMAT_TMSF`, reads
`MCI_STATUS_NUMBER_OF_TRACKS` (12), calls `auxGetNumDevs` (presumably for
CD volume through the imported `auxSetVolume`; Wine reports no device),
reads the start position of each of the 12 tracks to find the range of
audio tracks, and then polls `MCI_STATUS_MODE` roughly every two seconds.
It sends `MCI_PLAY` from track *n* to *n*+1 only when both of these hold:
bit 0x8 of the sound-flags word, which is dword 4 of `hgsnd.cfg` (the
value written on first run, `0x0b`, has it set), and a track *n* inside
the audio range taken from the mission's `MUSI` chunk. The scenario
loader copies 8 bytes of that chunk's text from chunk offset 0x0a, and
the music start routine parses the integer from its sixth character
(`sscanf` with `"%d"` at RVA 0x1F29A). The Tour of Duty mission the shell
generates, `RAN1SCN1.BWD`, carries `track2` and played CD track 2. The
training mission opened the device and polled it for 40 s without
sending `MCI_PLAY`. Its scenario, `M33_SCN1.BWD`, has no `MUSI` chunk,
and neither do the six files it includes (§5, extracted from the pack
on 2026-11-18), so that mission plays no music by design. Code: CD init
at RVA 0x9E1EF, play
at 0x9E1A9, mode poll at 0x9E2E3, mission music start at 0x1F26E,
periodic retry at 0x1F3AC; `hgsnd.cfg` is read by the call at 0x85227.

First-run files written by this sequence, with sizes: `hgprm.cfg` (836 B;
holds the mission id `M33_SCN1`), `hgsnd.cfg` (108 B; names
`vesa480.dll`, one of the `VFX/` drivers), `hgdif.cfg` (23 B),
`hgnet.cfg` (23 B), `CHARINFO.HG1` (20 B), and `DLLFILE.INI` rewritten
from `3DFXGEAR.DLL` to `I76DRAW.DLL` after the 3dfx plugin failed to load.

Input: the shell loads `dinput8` and enumerates HID joysticks.

## 5. Data formats

### HEAVYG.MAP and HEAVYG.PRJ

`crates/hgpack` implements this subsection. `hgpack verify <game-dir>`
and `contracts/pack.ncl` check it against the shipped files: on
2026-11-18, 19,545 records accounted for all 96,187,673 bytes of the
pack, and every record decoded to its stated size. All integers are
little-endian. Text fields are NUL-padded.

`HEAVYG.MAP`, the index (391,108 B):

| Offset | Size | Field | Shipped value |
| ---: | ---: | :--- | :--- |
| 0x00 | 4 | entry count *n* | 19,545 |
| 0x04 | 4 | type count *t* | 20 |
| 0x08 | 8 × *t* | type table: char\[4\] type, u32 entries of that type | below |
| 0xA8 | 4 | reserved | 0, meaning **UNVERIFIED** |
| 0xAC | 4 | codec count *c* | 2 |
| 0xB0 | 16 × *c* | codec table: char\[16\] DLL name | `pkware.dll`, `zlib.dll` |
| 0xD0 | 20 × *n* | entries: char\[8\] name, char\[4\] type, u32 pack offset, char\[4\] attribute | |

The offsets after 0x08 are the shipped ones (*t* = 20, *c* = 2), and
0xD0 + 20 × 19,545 = 391,108, the file's size. The type table, in file
order: `PCXX` 630, `WAVX` 1,053, `BRF` 229, `BWD` 705, `WAV` 103, `PAR`
1, `FNT` 27, `TBL` 20, `PCX` 789, `SHP` 788, `SDF` 451, `3DI` 147, `GDF`
76, `MLT` 255, `TLT` 219, `ODF` 1,027, `WTB` 10,345, `XEL` 1,508, `TEX`
1,119, `COL` 53. The counts sum to 19,545 and equal each type's tally
among the entries.

Entries ascend by the raw 8-byte name field, including any bytes after
its NUL: 33 names carry such bytes (`ALLWHT\0Y`). Offsets ascend
strictly in the same order. Whether the engine binary-searches the field
is **UNVERIFIED**. The attribute reads `NONE` in every entry. It is not
a codec key (see below), and its meaning is **UNVERIFIED**. Six keys
occur more than once:
- with identical headers: `AVOM204F.WAV` ×2 and `AVOSCDEC.WAVX` ×2;
- with differing sizes: `B20UNDER.PCX` ×2, `B47UNDER.PCX` ×6,
  `B48UNDER.PCX` ×6 and `O_1CRGZW.SDF` ×2.

Which copy the engine loads is **UNVERIFIED**.

`HEAVYG.PRJ`, the pack (96,187,673 B), is exactly the index's records
end to end, in index order, with nothing between or after them. Each
record is:

| Size | Field |
| ---: | :--- |
| 12 | tag: `C:\PRJ\` + the name's first byte + the type, NUL-padded (`C:\PRJ\AWAV`); four-letter types fill the field (`C:\PRJ\AWAVX`) |
| 2 | method: 0 stored; *k* is codec-table entry *k* − 1 |
| 4 | unpacked size |
| 4 | packed size |
| packed size | data |

The tag reads like a directory on the build machine, one per initial
letter and type. This is an inference from its form.

Methods in the shipped pack:

| Method | Records | Streams | Types |
| :--- | ---: | :--- | :--- |
| 0, stored (unpacked = packed) | 2,302 | | `FNT`, `PAR` only |
| 1, PKWare DCL implode | 1,419 | all start `00 04` (binary literals, 1 KiB dictionary) | `PCX`, `PCXX` only |
| 2, zlib (RFC 1950) | 15,824 | all start `78 9c` | `3DI`, `COL`, `GDF`, `SDF`, `TBL`, `TEX`, `WAV`, `WAVX` only |

`BRF`, `BWD`, `MLT`, `ODF`, `SHP`, `TLT`, `WTB` and `XEL` mix stored and
zlib records. Unpacked, the records total 204,010,594 bytes. The pack
compresses `PCX` and `PCXX`, and `WAV` and `WAVX`, alike. Whatever the
`X` suffix marks is therefore inside the payloads, which is
**UNVERIFIED**.

The codec mapping rests on four observations:
- `PKWARE.DLL` and `ZLIB.DLL` both export `Compress`, `GetKey` and
  `Uncompress` (`objdump -p`).
- `GetKey` copies `PKWARE` (`PKWARE.DLL` `.data` 0x1001CAB0) or `ZLIB`
  (`ZLIB.DLL` 0x1000D030).
- The codec table's order matches the streams' formats.
- `HGSHELL.DLL` and `HGSIM.DLL` both contain the string `Uncompress`.

Every PKWare record also decodes to its stated size with no input left
over under `blast.c`, the reference decoder in zlib's `contrib/blast`.

`CACHE.INI` gives per-type hash-table sizes and cache byte budgets for
`tex pcx pcxx bmp tbl col clr shp xel wav 3di wtb gdf vdf odf sdf bwd fnt
mgi mlt tlt par`, which is the authoritative list of resource types the
engine caches.

### HG.INI [SystemError]

Seventy-nine error names document the engine's subsystems: BWD world
files are chunked (`SYSERR_BADBWDCHUNK`, `ckSize`, "View Spec chunk", "BM
PRJ Chunk", `SYSERR_INCOMPATIBLEBWDREV`); PRJ packs hold audio and "bpl
polygon" resources; there are mech-spec, GAMEPIECE class, scenario and
mission tables, Gouraud / unmapped / mapped vertex records, a geometry
cache with block nesting, and DOS-era leftovers (`digpak driver`, `x/ems`,
`xems swap pages`). The engine lineage to Interstate '76 and MechWarrior 2
is reported by community sources and is **UNVERIFIED** here.

### BWD files

First look, from two files walked on 2026-11-18. The two are the
training scenario `M33_SCN1.BWD`, extracted with `hgpack`, and the
generated Tour of Duty scenario `RAN1SCN1.BWD`, a loose file. A `BWD`
file is a 12-byte header followed by chunks:
- The header is char\[4\] `BWD\0`, a u32 total size equal to the file's
  length, and a u32 of unknown meaning (0x1855 and 0x0575 in the two
  files).
- Each chunk is a char\[4\] tag (NUL-padded) and a u32 size that counts
  the chunk's own 8 bytes, then its data.

The chunks tile both files exactly. `M33_SCN1.BWD` (10,483 B) holds 74
top-level chunks:

| Tag | Count | Notes |
| :--- | ---: | :--- |
| `REV` | 1 | format revision, the text `1.22` |
| `DTBL` | 1 | |
| `INCL` | 6 | names another `BWD` to load; text at chunk offset 0x0A |
| `CAMO` | 3 | |
| `STAR` | 1 | |
| `MTBL` | 3 | |
| `BTAG` | 20 | |
| `ETAG` | 19 | |
| `GPS` | 3 | |
| `NAVP` | 17 | |

The six `INCL` targets are `m33_tabl`, `formatns`, `m33_art1`,
`gearsht1`, `m33_grnd` and `m33_obj1`.

`RAN1SCN1.BWD` (2,502 B) holds `REV` `1.22`, `DTBL`, `INCL`, `MUSI`
`track2`, `STAR`, `MTBL` and more `INCL` chunks. `MUSI` puts its text at
offset 0x0A too, which is where the scenario loader copies it from
(§4). Whether `BTAG` and `ETAG` bracket nested groups, given 20 of one
against 19 of the other, is **UNVERIFIED**.

### Loose files

`*.VDF` vehicle definitions; `*.BWD` world/scene files; `*.BRF`; `*.RMG`;
`*.GDF` (`USERGEAR.GDF`); `*.CCT` control configurations; `*.MAP` text
input maps; `*.FRC` RIFF `FORC` force-feedback effects; `*.PAR`; `*.SMK`
Smacker video; `FEIF.DAT`. Files the game generates after first run, from
the scene release tree: `HGCAR.CFG`, `HGDIF.CFG`, `HGMSN.CFG`, `HGNET.CFG`,
`HGPRM.CFG`, `HGSND.CFG`, `CHARINFO.HG1`, `USERGEAR.GDF`, `USERSTAR.BWD`,
`RAND*.BWD`, `RAND*.BRF`, `RAN1SCN1.BWD`, `COMPUT0?.BWD`, `OBJECT0?.BWD`,
`ARTRECRD.RMG`, `MESHRCRD.RMG`, `SEEDFILE.RMG`.

## 6. Copy protection

The scene release's `HG.EXE` is byte-identical to the disc's, while its
`HGSHELL.DLL` and `HGSIM.DLL` (dated 1997-11-26) differ in a few bytes:

| File | Differing file offsets (runs) | Bytes | Section |
| :--- | :--- | ---: | :--- |
| `HGSHELL.DLL` | 1020505; 1020604; 1020629–1020634; 1020639–1020640; 1021039; 1021138; 1021163–1021168; 1021173–1021174 | 20 | `.text` (vma `0x10001000` + offset − `0x400`) |
| `HGSIM.DLL` | 625052–625053; 625099; 625116–625117 | 5 | `.text` (same mapping) |

The two HGSHELL clusters are 534 bytes apart with identical shapes, which
suggests two copies or two call sites of one check. These offsets in the
clean binaries are the starting point for locating the Disc 2 check. The
cracked binaries are not used by this project for anything else.

## 7. Host integration notes (Wine 11.0, NixOS)

- The fullscreen 640×480×8 mode switch is rejected on Xwayland
  (`NtUserChangeDisplaySettings` returns `DISP_CHANGE_BADMODE`); the game
  falls back to a black window. A Wine virtual desktop (`explorer
  /desktop=…`) makes the switch succeed.
- `HKCU\Software\Wine\Direct3D\renderer = gdi` lets the palettised
  DirectDraw path run without OpenGL.
- Frames from a headless Xvfb display are captured correctly by
  `ffmpeg -f x11grab` (menu, Tour of Duty hub, briefing and mission,
  2026-11-09), so the uniformly black captures recorded earlier came from
  the capture path, not from rendering. `scripts/wine-smoke.sh` still
  captures with ImageMagick `import`, which fails outright on this host
  ("missing an image filename").
- With Disc 2 present the shell plays `VIDEO\B51MOVIE.AVI` (Indeo 5)
  through DirectShow. Under Wine 11.0 the trace ends inside graph
  construction (`quartz.dll`, `msvfw32.dll`, `msdmo.dll`, `devenum.dll`
  loaded, DirectSound devices enumerated) and the shell then produces no
  front-buffer update, file access, or exit for the following two
  minutes, on both the real display and Xvfb: the intro playback stalls
  the shell. Cause **UNRESOLVED**. Disabling Wine's DirectShow
  (`WINEDLLOVERRIDES=quartz=d`) makes the movie's `CoCreateInstance` fail
  at once (`err:ole:com_get_class_object` for the filter graph) and the
  shell continues to the menu, so cutscenes are skipped rather than
  played. `scripts/wine-run.sh` and `scripts/wine-smoke.sh` set this
  override unless `SUPERGEAR_MOVIES=1`.
- The soundtrack is CD audio on Disc 2 (§1, §4). Wine's `mcicda` reads
  audio tracks digitally (`IOCTL_CDROM_RAW_READ`) and plays them through
  DirectSound, but only from a CD-ROM device; a drive letter mapped to an
  extracted directory has no audio tracks. `scripts/wine-cdaudio.sh`
  loads the Disc 2 `.mds` into a CDEmu virtual drive (NixOS
  `programs.cdemu.enable`, user in group `cdrom`), mounts it through
  udisks, and points `r:` at the mount and `r::` at the device. Verified
  2026-11-09 through `scripts/wine-run.sh`: in a Tour of Duty mission the
  game played track 2, and a recording of Wine's output (a PipeWire null
  sink) correlates with track 2 read from `HEAVYG2.mdf` at 1.000, against
  0.000 when misaligned by 7 s.
- Wine's mount manager follows udisks. When the device behind `r::` has
  no mount point it deletes the `r:` link, so the virtual disc must stay
  mounted; without an `r::` link it gives a loaded virtual disc a drive
  letter of its own, where the game finds it too.
- With a CD-ROM device present the mission hangs on "Commencing Mission"
  right after `MCI_STATUS_NUMBER_OF_TRACKS` (three untraced runs; a
  `+relay` run, being slower, got through). Backtrace of the hung main
  thread: `HGSIM` calls `auxGetNumDevs`, winmm loads its legacy driver
  (`MMDRV_Install("mmdevapi")`), finds no devices of any type and closes
  it, and mmdevapi's `DriverProc(DRV_FREE)` posts quit to its ALSA MIDI
  notify thread and waits on it with `INFINITE`, never returning. Why that
  thread does not exit is **UNRESOLVED**. `WINEDLLOVERRIDES=winealsa.drv=d`
  keeps the MIDI driver from loading, so the legacy driver fails to open
  and nothing is unloaded; DirectSound still plays through winepulse.
  `scripts/wine-run.sh` and `scripts/wine-smoke.sh` set it whenever a
  `/dev/sr*` device exists.
- Wine caches the monitor layout of the display it last saw. A prefix
  built on the multi-monitor desktop and first launched on a single-screen
  Xvfb places the game on a stale `\\.\DISPLAY2` and the mode switch fails
  once; a warm-up launch on the new display (`wineboot -u` plus an empty
  virtual-desktop session) fixes the next run, which `scripts/wine-smoke.sh`
  does.
- Wine types a drive from `dosdevices/<x>::` when present; the registry
  `HKLM\Software\Wine\Drives` entry is used only when that device link is
  absent. Wine's mount manager also assigns low drive letters to volumes
  it detects on the host and will replace a hand-made `d:` link, so
  `scripts/wine-install.sh` exposes the disc as `R:`. The mount manager
  reads the `Drives` key only when a wineserver starts: a game launched
  against the server that performed the registry write still sees `R:`
  as a fixed disk and the shell never probes it. The install script
  therefore waits for its server to exit (`wineserver -w`).
- `HG.EXE` must run with the game directory as its working directory
  (§4). The scripts `cd` there before launching. Until 2026-10-24 they did
  not: an interactive launch from the repository root crashed the shell at
  startup, while the smoke test, which only counted front-buffer updates,
  passed on the same broken launch. The smoke test now also requires the
  `zlib.dll` load that happens only once the shell reads `HEAVYG.PRJ`.
- `HG.EXE` checks the display before loading anything. When the session
  reports no outputs (observed 2026-10-24: `wlr-randr` empty, X root
  window 0×0, Wine `err:system:lock_display_devices Failed to read display
  config`) it stops at a message box ending "You may change your Display
  Properties using the Windows Control Panel. Would you like to quit and
  change your settings now?" (`+msgbox` trace) and never loads
  `DKUPDDLL.dll`.
- This host exports `WINEDEBUG=-all` globally (`/etc/set-environment`), so
  a plain `wine` run prints no `err:` lines; `scripts/wine-run.sh` treats
  that value as unset and uses `err+all`.
