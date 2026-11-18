#!/usr/bin/env bash
# Launch the original Heavy Gear under Wine, inside a Wine virtual desktop.
#
# Why a virtual desktop: the shell asks DirectDraw for a fullscreen
# 640x480 8-bit mode. On Xwayland that mode switch is rejected
# (NtUserChangeDisplaySettings returns DISP_CHANGE_BADMODE) and the game
# falls back to a black window. Inside a virtual desktop Wine emulates the
# mode switch by resizing, the palette is honoured, and the shell draws.
#
# The prefix is built by wine-install.sh; this script only runs the game.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
: "${SUPERGEAR_HOME:=${XDG_DATA_HOME:-$HOME/.local/share}/supergear}"
export WINEPREFIX="$SUPERGEAR_HOME/wine"
export WINEARCH=win32
# Errors only by default. This host exports WINEDEBUG=-all system-wide
# (/etc/set-environment), which would silence the err: lines that explain
# a failure, so that value is treated like "unset". Wine's crash backtrace
# is printed by winedbg regardless of channels and lands in the log too.
case "${WINEDEBUG:-}" in ""|-all) export WINEDEBUG=err+all ;; esac
log="$SUPERGEAR_HOME/run.log"

if [ ! -f "$WINEPREFIX/drive_c/Games/HeavyGear/HG.EXE" ]; then
  echo "wine-run: no installed game at $WINEPREFIX; run scripts/wine-install.sh first" >&2
  exit 1
fi
# Absolute, because the script changes directory below.
SUPERGEAR_HOME=$(cd "$SUPERGEAR_HOME" && pwd)
log="$SUPERGEAR_HOME/run.log"

# Cutscene playback through Wine's DirectShow stalls the shell with no
# further activity (docs/spec/heavy-gear.md §7). DirectShow is disabled
# unless SUPERGEAR_MOVIES=1, so the shell treats each movie as unplayable
# and moves on to the menu.
if [ "${SUPERGEAR_MOVIES:-0}" != "1" ]; then
  export WINEDLLOVERRIDES="quartz=d${WINEDLLOVERRIDES:+;$WINEDLLOVERRIDES}"
fi

# R: is Disc 2: a CDEmu virtual drive with the soundtrack when the host
# has CDEmu, else the extracted directory (silent). See wine-cdaudio.sh.
SUPERGEAR_HOME="$SUPERGEAR_HOME" "$here/wine-cdaudio.sh"

# With a CD-ROM device on the host (Wine gives each one a drive letter,
# R: or another), HGSIM opens it for music and then calls auxGetNumDevs,
# which makes Wine load its legacy sound drivers. Finding no MIDI ports,
# Wine 11 unloads the ALSA MIDI driver and never returns (mmdevapi waits on
# its MIDI notify thread), so the mission hangs on "Commencing Mission".
# The game uses no MIDI and DirectSound goes through winepulse, so the
# ALSA driver is disabled whenever a CD-ROM device exists (spec §7).
if ls /dev/sr[0-9]* >/dev/null 2>&1; then
  export WINEDLLOVERRIDES="winealsa.drv=d${WINEDLLOVERRIDES:+;$WINEDLLOVERRIDES}"
fi

# The shell opens its configuration and index files by bare name
# (shelldll.ini, heavyg.map, cache.ini, lang.ini, ...), so they resolve
# against the process's working directory. Launched from anywhere else it
# never finds them and crashes in its resource cache (spec §4, §7).
cd "$WINEPREFIX/drive_c/Games/HeavyGear"

desktop="${SUPERGEAR_DESKTOP:-1024x768}"
echo "wine-run: Wine output goes to $log"
wine explorer "/desktop=HeavyGear,$desktop" 'C:\Games\HeavyGear\HG.EXE' "$@" 2>&1 | tee "$log"
