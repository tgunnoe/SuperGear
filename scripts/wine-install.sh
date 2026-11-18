#!/usr/bin/env bash
# Build a Wine prefix with Heavy Gear installed from the Disc 1 image.
#
# The disc's InstallShield 3 setup only copies INSTALL/DATA to the target
# directory (DATA1.CAB holds 262 KB of shortcuts and strings), and it cannot
# be driven without a GUI, so this script performs the same copy itself.
# Nothing from the disc is written inside the repository.
#
# Inputs (environment):
#   SUPERGEAR_MDF   Disc 1 as an Alcohol .mdf image (default: ~/Downloads/Games/hg/Heavy Gear.mdf)
#   SUPERGEAR_ISO   Disc 1 as a plain ISO; overrides SUPERGEAR_MDF when set
#   SUPERGEAR_CD2   Disc 2 ("Gameplay CD") as .mdf, .iso, or an extracted directory
#                   (default: ~/Downloads/Games/hg/HEAVYG2.mdf when that file exists).
#                   Without it the game runs up to the shell's insert-CD-2 prompt.
#   SUPERGEAR_HOME  Where the prefix and extracted discs live
#                   (default: ${XDG_DATA_HOME:-~/.local/share}/supergear)
#   SUPERGEAR_LANG  Control-set language letter: E (English, default), F, I, S
#
# Requires: wine (32-bit capable), python3, 7z. All are in the flake dev shell.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
: "${SUPERGEAR_HOME:=${XDG_DATA_HOME:-$HOME/.local/share}/supergear}"
: "${SUPERGEAR_MDF:=$HOME/Downloads/Games/hg/Heavy Gear.mdf}"
: "${SUPERGEAR_ISO:=}"
: "${SUPERGEAR_CD2:=}"
: "${SUPERGEAR_LANG:=E}"
if [ -z "$SUPERGEAR_CD2" ] && [ -f "$HOME/Downloads/Games/hg/HEAVYG2.mdf" ]; then
  SUPERGEAR_CD2="$HOME/Downloads/Games/hg/HEAVYG2.mdf"
fi

export WINEPREFIX="$SUPERGEAR_HOME/wine"
export WINEARCH=win32
export WINEDEBUG="${WINEDEBUG:--all}"

cd1="$SUPERGEAR_HOME/cd1"
cd2="$SUPERGEAR_HOME/cd2"
game="$WINEPREFIX/drive_c/Games/HeavyGear"

mkdir -p "$SUPERGEAR_HOME"

# 1. Disc 1 as ISO.
if [ -z "$SUPERGEAR_ISO" ]; then
  SUPERGEAR_ISO="$SUPERGEAR_HOME/HeavyGear-cd1.iso"
  if [ ! -f "$SUPERGEAR_ISO" ]; then
    echo "wine-install: converting $SUPERGEAR_MDF -> $SUPERGEAR_ISO"
    python3 "$here/mdf2iso.py" "$SUPERGEAR_MDF" "$SUPERGEAR_ISO"
  fi
fi

# 2. Extract Disc 1. The game's CD check inspects the drive root, so the
#    whole tree is kept and exposed as drive D:.
if [ ! -f "$cd1/INSTALL/DATA/HG.EXE" ]; then
  echo "wine-install: extracting disc 1 to $cd1"
  rm -rf "$cd1"; mkdir -p "$cd1"
  7z x -y -o"$cd1" "$SUPERGEAR_ISO" >/dev/null
fi

# 3. Optional Disc 2, the mixed-mode "Gameplay CD": a data track with the
#    cutscene videos, then CD audio. An .mdf is converted like Disc 1 (the
#    audio tracks are skipped); an .iso or a directory is used as is.
label_iso="$SUPERGEAR_ISO"
if [ -n "$SUPERGEAR_CD2" ]; then
  if [ -d "$SUPERGEAR_CD2" ]; then
    cd2="$SUPERGEAR_CD2"
  else
    cd2_iso="$SUPERGEAR_CD2"
    case "$SUPERGEAR_CD2" in
      *.mdf|*.MDF)
        cd2_iso="$SUPERGEAR_HOME/HeavyGear-cd2.iso"
        if [ ! -f "$cd2_iso" ]; then
          echo "wine-install: converting $SUPERGEAR_CD2 -> $cd2_iso"
          python3 "$here/mdf2iso.py" "$SUPERGEAR_CD2" "$cd2_iso"
        fi ;;
    esac
    label_iso="$cd2_iso"
    if [ ! -d "$cd2/VIDEO" ]; then
      echo "wine-install: extracting disc 2 to $cd2"
      mkdir -p "$cd2"
      7z x -y -o"$cd2" "$cd2_iso" >/dev/null
    fi
  fi
fi

# 4. Fresh 32-bit prefix. HG.EXE is a 1997 i386 PE binary; WoW64 is not needed.
if [ ! -f "$WINEPREFIX/system.reg" ]; then
  echo "wine-install: creating prefix $WINEPREFIX"
  wineboot -i >/dev/null 2>&1
fi

# 5. Copy the installer payload; promote the chosen language's control sets
#    to the game root, the layout a real install produces.
echo "wine-install: installing payload to $game"
mkdir -p "$game"
cp -r "$cd1/INSTALL/DATA/." "$game/"
cp "$game/ACTION$SUPERGEAR_LANG/"* "$game/"
cp "$game/SIM$SUPERGEAR_LANG/CURRENT.CCT" "$game/CURRENT.CCT"
cp -r "$cd1/HELP" "$game/HELP"
cp -r "$cd1/SPLASH" "$game/SPLASH"

# 6. Drive letters. The game scans every drive for one of type CD-ROM, so
#    the letter is free; R: is used because Wine's mount manager hands out
#    low letters to volumes it detects on the host and would overwrite a
#    manual D:. With Disc 2 present R: is the gameplay disc; otherwise
#    Disc 1 so that a CD-ROM drive exists at all. Wine reads the volume
#    label from a .windows-label file at the drive root.
cdrom_dir="$cd1"
if [ -n "$SUPERGEAR_CD2" ]; then
  cdrom_dir="$cd2"
fi
if [ ! -f "$cdrom_dir/.windows-label" ] && [ -f "$label_iso" ]; then
  python3 - "$label_iso" "$cdrom_dir" <<'EOF'
import sys
iso, out = sys.argv[1], sys.argv[2]
# Volume identifier: ISO 9660 primary volume descriptor, sector 16, bytes 40..72.
with open(iso, "rb") as f:
    f.seek(16 * 2048 + 40)
    label = f.read(32).decode("ascii", "replace").strip()
open(f"{out}/.windows-label", "w").write(label + "\n")
print(f"wine-install: volume label {label!r}")
EOF
fi
ln -sfn "$cdrom_dir" "$WINEPREFIX/dosdevices/r:"
wine reg add 'HKLM\Software\Wine\Drives' /v 'R:' /t REG_SZ /d cdrom /f >/dev/null

# 7. DirectDraw through GDI. The shell renders in software into an 8-bit
#    palettised surface and blits to the primary; Wine's GDI renderer handles
#    that without OpenGL, which also makes headless runs possible.
wine reg add 'HKCU\Software\Wine\Direct3D' /v renderer /t REG_SZ /d gdi /f >/dev/null

# 8. Let this wineserver exit. The mount manager reads the Drives key only
#    when a server starts, so a game launched against the server that
#    performed the registry writes would still see R: as a fixed disk.
wineserver -w

echo "wine-install: done. Run scripts/wine-run.sh"
