#!/usr/bin/env bash
# Point the prefix's R: drive at Disc 2: a CDEmu virtual drive holding the
# disc image when the host has CDEmu, so the game gets its soundtrack, or
# else the extracted directory from wine-install.sh, which runs silent.
#
# The soundtrack is eleven Redbook audio tracks (docs/spec/heavy-gear.md
# §1); HGSIM.DLL plays them through MCI cdaudio. Wine's mcicda reads the
# tracks digitally (IOCTL_CDROM_RAW_READ) and plays them through
# DirectSound, but only from a CD-ROM device: a directory has no audio
# tracks. The virtual disc is mounted through udisks and serves R:'s files
# too. It must stay mounted: when the device behind `r::` has no mount
# point, Wine's mount manager follows udisks and deletes the `r:` link.
#
# CDEmu needs the host module (NixOS: programs.cdemu.enable = true),
# membership of group cdrom in the current login session, and udisksctl.
# Idempotent; wine-run.sh and wine-smoke.sh call it before every launch.
#
# Inputs (environment):
#   SUPERGEAR_CD2      Disc 2 as an Alcohol .mdf or .mds image
#                      (default: ~/Downloads/Games/hg/HEAVYG2.mdf); the .mds
#                      descriptor must sit beside the .mdf
#   SUPERGEAR_HOME     As in wine-install.sh
#   SUPERGEAR_CDAUDIO  0 to use the extracted directory even with CDEmu (a
#                      loaded virtual disc still appears under another
#                      letter, and the game plays music from any CD drive)
#   SUPERGEAR_CDEMU    CDEmu device number (default 0)
set -euo pipefail

: "${SUPERGEAR_HOME:=${XDG_DATA_HOME:-$HOME/.local/share}/supergear}"
: "${SUPERGEAR_CD2:=$HOME/Downloads/Games/hg/HEAVYG2.mdf}"
: "${SUPERGEAR_CDEMU:=0}"
dosdevices="$SUPERGEAR_HOME/wine/dosdevices"
[ -d "$dosdevices" ] || { echo "wine-cdaudio: no prefix under $SUPERGEAR_HOME; run scripts/wine-install.sh first" >&2; exit 1; }

# The directory wine-install.sh mapped: Disc 2 if it was given, else Disc 1.
use_directory() {
  local dir="$SUPERGEAR_HOME/cd1"
  if [ -d "$SUPERGEAR_CD2" ]; then dir="$SUPERGEAR_CD2"
  elif [ -d "$SUPERGEAR_HOME/cd2/VIDEO" ]; then dir="$SUPERGEAR_HOME/cd2"
  fi
  rm -f "$dosdevices/r::"
  ln -sfn "$dir" "$dosdevices/r:"
  echo "wine-cdaudio: R: is $dir, no music${1:+ ($1)}"
  exit 0
}

[ "${SUPERGEAR_CDAUDIO:-1}" = "1" ] || use_directory "SUPERGEAR_CDAUDIO=0"
command -v cdemu >/dev/null || use_directory "cdemu not found; NixOS: programs.cdemu.enable = true"
command -v udisksctl >/dev/null || use_directory "udisksctl not found"

# CDEmu's image library opens an Alcohol image through its .mds descriptor.
case "$SUPERGEAR_CD2" in
  *.mds|*.MDS) image="$SUPERGEAR_CD2" ;;
  *.mdf|*.MDF)
    image="${SUPERGEAR_CD2%.*}.mds"
    [ -f "$image" ] || image="${SUPERGEAR_CD2%.*}.MDS" ;;
  *) use_directory "SUPERGEAR_CD2 is not an .mdf or .mds image" ;;
esac
[ -f "$image" ] || use_directory "no .mds descriptor beside $SUPERGEAR_CD2"
image=$(realpath "$image")

# Load unless this device already holds this image. The first cdemu call
# starts the daemon through D-Bus activation.
loaded=$(cdemu status 2>/dev/null | awk -v d="$SUPERGEAR_CDEMU" '$1 == d { print $2, $3 }') ||
  use_directory "the CDEmu daemon did not answer"
if [ "$loaded" != "True $image" ]; then
  echo "wine-cdaudio: loading $image into CDEmu device $SUPERGEAR_CDEMU"
  cdemu load --replace "$SUPERGEAR_CDEMU" "$image" >/dev/null ||
    use_directory "cdemu could not load $image"
fi

# The kernel registers the SCSI CD-ROM node shortly after the load.
dev=""
for _ in 1 2 3 4 5 6 7 8 9 10; do
  dev=$(cdemu device-mapping | awk -v d="$SUPERGEAR_CDEMU" '$1 == d && $2 ~ /^\/dev\// { print $2 }') || true
  [ -n "$dev" ] && [ -b "$dev" ] && break
  sleep 0.5
done
[ -n "$dev" ] && [ -b "$dev" ] || use_directory "CDEmu device $SUPERGEAR_CDEMU has no /dev/sr node"
[ -r "$dev" ] || use_directory "$dev is not readable; join group cdrom and log in again"

mnt=$(findmnt -rn -o TARGET "$dev" || true)
if [ -z "$mnt" ]; then
  udisksctl mount -b "$dev" --no-user-interaction >/dev/null || use_directory "udisksctl could not mount $dev"
  mnt=$(findmnt -rn -o TARGET "$dev")
fi

ln -sfn "$mnt" "$dosdevices/r:"
ln -sfn "$dev" "$dosdevices/r::"
echo "wine-cdaudio: R: is Disc 2 in $dev, mounted at $mnt"
