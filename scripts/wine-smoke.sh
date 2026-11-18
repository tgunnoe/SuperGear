#!/usr/bin/env bash
# Headless smoke test for the Wine-hosted original game.
#
# Passes as soon as Wine's DirectDraw trace shows a front-buffer update while
# the HG.EXE process is alive: the shell reached its render loop. Fails if
# that has not happened within SMOKE_SECONDS (a first launch in a fresh
# prefix takes longer than later ones). Needs Xvfb and the prefix built by
# wine-install.sh.
#
# Known limitation: frames captured from Xvfb come out uniformly black even
# when the shell is drawing (the same configuration renders correctly on a
# real display), so the screenshot written to SMOKE_OUT is for a human eye
# only and is not part of the pass/fail decision.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
: "${SUPERGEAR_HOME:=${XDG_DATA_HOME:-$HOME/.local/share}/supergear}"
: "${SMOKE_SECONDS:=90}"
: "${SMOKE_OUT:=$SUPERGEAR_HOME/smoke}"
export WINEPREFIX="$SUPERGEAR_HOME/wine"
export WINEARCH=win32
export DISPLAY=:99
export WAYLAND_DISPLAY=
# +loaddll: the shell loads zlib.dll only once it is reading HEAVYG.PRJ, so
# that load proves the data was found (a shell started outside its
# directory still draws frames but never gets there).
export WINEDEBUG="+ddraw,+loaddll,err+all"
# Same DirectShow override as wine-run.sh: with Disc 2 mapped the intro
# movie would otherwise stall the shell before its first frame.
if [ "${SUPERGEAR_MOVIES:-0}" != "1" ]; then
  export WINEDLLOVERRIDES="quartz=d${WINEDLLOVERRIDES:+;$WINEDLLOVERRIDES}"
fi

mkdir -p "$SMOKE_OUT"
SMOKE_OUT=$(cd "$SMOKE_OUT" && pwd)  # absolute: the script changes directory below
log="$SMOKE_OUT/wine.log"

Xvfb :99 -screen 0 1280x1024x24 +extension GLX +render -noreset -nolisten tcp >/dev/null 2>&1 &
xvfb_pid=$!
cleanup() {
  wineserver -k >/dev/null 2>&1 || true
  kill "$xvfb_pid" >/dev/null 2>&1 || true
}
trap cleanup EXIT
sleep 2

# Warm the prefix on this display first. Wine caches the monitor layout
# of the display it last saw; a prefix built on a multi-monitor desktop
# and first run on Xvfb otherwise places the game on a stale monitor and
# the 640x480 mode switch fails once.
WINEDEBUG=-all wineboot -u >/dev/null 2>&1 || true
WINEDEBUG=-all wine explorer /desktop=HeavyGear,1024x768 cmd /c exit >/dev/null 2>&1 || true
wineserver -w

# R: as wine-run.sh sets it (CDEmu virtual disc or extracted directory),
# after the warm-up: the mount manager reads the drive links at start.
SUPERGEAR_HOME="$SUPERGEAR_HOME" "$here/wine-cdaudio.sh"
# Same ALSA override as wine-run.sh when the host has a CD-ROM device.
if ls /dev/sr[0-9]* >/dev/null 2>&1; then
  export WINEDLLOVERRIDES="winealsa.drv=d${WINEDLLOVERRIDES:+;$WINEDLLOVERRIDES}"
fi

# Bare-name opens resolve against the working directory (see wine-run.sh).
cd "$WINEPREFIX/drive_c/Games/HeavyGear"
wine explorer /desktop=HeavyGear,1024x768 'C:\Games\HeavyGear\HG.EXE' >"$log" 2>&1 &

alive=0
frames=0
data=0
elapsed=0
while [ "$elapsed" -lt "$SMOKE_SECONDS" ]; do
  sleep 5
  elapsed=$((elapsed + 5))
  frames=$(grep -c 'ddraw_surface_update_frontbuffer' "$log" || true)
  data=$(grep -ci 'Loaded L"C:\\\\Games\\\\HeavyGear\\\\zlib.dll"' "$log" || true)
  alive=0
  if winedbg --command "info proc" 2>/dev/null | grep -q "HG.EXE"; then
    alive=1
  fi
  if [ "$alive" -eq 1 ] && [ "$frames" -ge 1 ] && [ "$data" -ge 1 ]; then
    break
  fi
  if [ "$alive" -eq 0 ] && [ "$elapsed" -ge 20 ]; then
    break
  fi
done

if command -v import >/dev/null 2>&1; then
  import -window root "$SMOKE_OUT/frame.png" 2>/dev/null || true
fi

echo "wine-smoke: after ${elapsed}s HG.EXE alive=$alive frontbuffer_updates=$frames data_loaded=$data log=$log"
if [ "$alive" -eq 1 ] && [ "$frames" -ge 1 ] && [ "$data" -ge 1 ]; then
  echo "wine-smoke: PASS"
  exit 0
fi
echo "wine-smoke: FAIL" >&2
exit 1
