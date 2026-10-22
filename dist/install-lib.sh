# shellcheck shell=bash disable=SC2034
# Shared by dist/install.sh and dist/uninstall.sh (sourced, never run). Where each piece of files
# goes, and the helpers that print every step and honour --dry-run. The same shape as mailo's
# dist/install-lib.sh, so one habit covers both.
#
# Nothing here touches the person's own files: no ~/.config, no ~/.local/share, no tags or Trash.

DIST="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$DIST/.." && pwd)"
APPDIST="$ROOT/crates/files/dist"

# FILES_PREFIX is where the files will live on the computer (the paths the service files name);
# FILES_DESTDIR is a staging root put in front of every path written (a release bundle, a scratch
# directory), empty for a real install.
PREFIX="${FILES_PREFIX:-/usr/local}"
DESTDIR="${FILES_DESTDIR:-}"
ROOTED="$DESTDIR$PREFIX"
BIN_INSTALLED="$PREFIX/bin/files"
BIN_DEST="$ROOTED/bin/files"
DESKTOP_DEST="$ROOTED/share/applications/org.quire.Files.desktop"
# The two bus services: Files by its own name, and the file-manager name other programs call to
# "show this folder". Both under $PREFIX/share, which is on XDG_DATA_DIRS for /usr and /usr/local.
SERVICE_DEST="$ROOTED/share/dbus-1/services/org.quire.Files.service"
FM_SERVICE_DEST="$ROOTED/share/dbus-1/services/org.quire.Files.FileManager1.service"
# The intents manifest the desktop's router reads, beside the other apps'.
INTENTS_DEST="$ROOTED/share/quire/intents/org.quire.Files.toml"
# The binary to install: a release build unless FILES_BIN says otherwise.
BUILT="${FILES_BIN:-${CARGO_TARGET_DIR:-$ROOT/target}/release/files}"
# What the person asked to escalate with, if anything (SUDO=doas ./install.sh), read before SUDO is
# reset below.
ASKED_SUDO="${SUDO:-}"
# Set by need_sudo to the program that gains root when the destination is not ours to write;
# empty for a staging root or a place we own.
SUDO=

DRY=0

# service_text SRC: a bus service file, its Exec= naming the binary where it will be installed.
service_text() { sed "s|^Exec=/usr/bin/files |Exec=$BIN_INSTALLED |" "$1"; }
# desktop_text: the desktop entry, launching that same binary.
desktop_text() { sed "s|^Exec=files |Exec=$BIN_INSTALLED |" "$APPDIST/org.quire.Files.desktop"; }

# parse_flags "$@": --dry-run, and the extra flags a script allows ($EXTRA_FLAGS, space-separated).
parse_flags() {
  local arg name
  for arg in "$@"; do
    case "$arg" in
      --dry-run) DRY=1 ;;
      -h|--help) usage; exit 0 ;;
      *) if [[ " ${EXTRA_FLAGS:-} " == *" $arg "* ]]; then name="${arg#--}"; declare -g "FLAG_${name//-/_}=1"; else
           echo "unknown argument: $arg" >&2; usage >&2; exit 2; fi ;;
    esac
  done
}

step() { printf '\n==> %s\n' "$*"; }
note() { printf '    %s\n' "$*"; }

# run CMD...: print the command; run it unless --dry-run.
run() {
  printf '    $ %s\n' "$*"
  [ "$DRY" = 1 ] || "$@"
}

# asroot CMD...: run CMD as root through the program need_sudo chose, or as ourselves when the
# destination is a place we own.
asroot() { run ${SUDO:+"$SUDO"} "$@"; }

# put MODE SRC DEST: install SRC at DEST, unless DEST already is a copy of it.
put() {
  local mode="$1" src="$2" dest="$3"
  if [ -f "$dest" ] && cmp -s "$src" "$dest"; then
    note "unchanged: $dest"
    return 0
  fi
  asroot install -D -m "$mode" "$src" "$dest"
}

# gone PATH: remove PATH when it exists.
gone() {
  if [ -e "$1" ]; then asroot rm -f "$1"; else note "already gone: $1"; fi
}

refuse_root() {
  [ "$(id -u)" -ne 0 ] || {
    echo "run this as yourself, not root: it builds as you and asks for root only to install" >&2
    exit 1
  }
}

# escalator: print the program that gains root here: the one asked for in $SUDO, else sudo, doas
# or pkexec, whichever is installed first. Prints nothing when there is none.
escalator() {
  local want
  for want in "$ASKED_SUDO" sudo doas pkexec; do
    [ -n "$want" ] && command -v "$want" >/dev/null && { echo "$want"; return 0; }
  done
  return 1
}

# writable_place: whether the nearest folder that exists above $ROOTED is ours to write.
writable_place() {
  local d="$ROOTED"
  while [ ! -e "$d" ] && [ "$d" != / ]; do d="$(dirname "$d")"; done
  [ -w "$d" ]
}

# need_sudo: pick the program that gains root, and ask for the password once, unless the destination
# is a staging root or a place we own (~/.local) or this is a dry run. With none to be had, say which
# commands to run as root instead.
need_sudo() {
  if [ -n "$DESTDIR" ] || writable_place; then SUDO=; return 0; fi
  if ! SUDO="$(escalator)"; then
    SUDO=sudo
    [ "$DRY" = 1 ] && return 0
    {
      echo "this needs root to install, and this computer has no sudo, doas or pkexec."
      echo "Either install one of them and run this again, or list the steps and run each"
      echo "line that starts with \$ yourself, as root:"
      echo "    $0 --dry-run"
    } >&2
    exit 1
  fi
  [ "$DRY" = 1 ] && return 0
  case "$SUDO" in
    sudo) note "sudo will ask for your password once"; sudo -v ;;
    *) note "$SUDO will ask for your password" ;;
  esac
}
