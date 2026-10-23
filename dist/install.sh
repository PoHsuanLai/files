#!/usr/bin/env bash
# Install files into the desktop from this checkout: build it in release, then put the binary, the
# desktop entry, the two bus services that start it (by its own name and as the desktop's file
# manager), the intents manifest and the settings schema where the session reads them. Run as
# yourself; sudo asks once when the place is not yours. Safe to run again.
#
#   dist/install.sh --dry-run    print every step, change nothing (not even a build)
#   dist/install.sh              do it
#
# Installs to /usr/local (FILES_PREFIX redirects it). Not touched: your files, the Trash,
# ~/.config, any other session's files. dist/uninstall.sh undoes it. Next: log in again, or run
# `xdg-mime default org.quire.Files.desktop inode/directory` to open folders in Files.
#
# FILES_BIN names the binary to install instead of the release build. FILES_PREFIX is where the
# files will live (the service files name it); FILES_DESTDIR is a staging root put in front of
# every path written (a release bundle or a scratch directory; it never needs sudo).
set -euo pipefail
# shellcheck source=install-lib.sh
source "$(dirname "${BASH_SOURCE[0]}")/install-lib.sh"

usage() { sed -n '2,15p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; }
EXTRA_FLAGS="--no-build"
parse_flags "$@"
[ "$DRY" = 0 ] || echo "dry run: nothing below is executed"

step "1/4 checks"
refuse_root
command -v cargo >/dev/null || [ "${FLAG_no_build:-0}" = 1 ] || [ -n "${FILES_BIN:-}" ] || { echo "cargo not found" >&2; exit 1; }
if [ -e "$HOME/.cargo/bin/files" ]; then
  note "WARNING: $HOME/.cargo/bin/files would come first on a login shell's PATH and shadow $BIN_INSTALLED;"
  note "         remove it (rm $HOME/.cargo/bin/files) or a shell runs the old build"
fi
need_sudo

step "2/4 build files (release, from $ROOT)"
if [ "${FLAG_no_build:-0}" = 1 ] || [ -n "${FILES_BIN:-}" ]; then
  note "no build: using $BUILT"
else
  # With this system's own linker, never the tree's clang/mold: run from / and name the manifest.
  TOOLCHAIN="$(sed -n 's/^channel *= *"\(.*\)"/\1/p' "$ROOT/rust-toolchain.toml" 2>/dev/null | head -n 1)"
  note "cargo build runs from / with --manifest-path $ROOT/Cargo.toml${TOOLCHAIN:+ (toolchain $TOOLCHAIN)}"
  if [ "$DRY" = 1 ]; then
    run cargo build --release --locked --manifest-path "$ROOT/Cargo.toml" -p files --bin files
  else
    (
      cd /
      [ -z "$TOOLCHAIN" ] || ! command -v rustup >/dev/null || export RUSTUP_TOOLCHAIN="$TOOLCHAIN"
      export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
      run cargo build --release --locked --manifest-path "$ROOT/Cargo.toml" -p files --bin files
    )
  fi
fi
[ "$DRY" = 1 ] || [ -x "$BUILT" ] || { echo "no binary at $BUILT" >&2; exit 1; }

step "3/4 the binary, the desktop entry, the services, the intents and the settings schema"
put 755 "$BUILT" "$BIN_DEST"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
desktop_text >"$TMP/desktop"
put 644 "$TMP/desktop" "$DESKTOP_DEST"
service_text "$APPDIST/org.quire.Files.service" >"$TMP/files.service"
put 644 "$TMP/files.service" "$SERVICE_DEST"
service_text "$APPDIST/org.quire.Files.FileManager1.service" >"$TMP/fm.service"
put 644 "$TMP/fm.service" "$FM_SERVICE_DEST"
# What the desktop's intent router reads.
put 644 "$DIST/intents/org.quire.Files.toml" "$INTENTS_DEST"
# What the Settings app draws Files' page from: the binary writes it from its own settings.
if [ "$DRY" = 1 ]; then
  note "would run: $BUILT --write-schema and install files.settings.toml"
else
  "$BUILT" --write-schema "$TMP/schema" >/dev/null || { echo "files --write-schema failed" >&2; exit 1; }
  put 644 "$TMP/schema/files.settings.toml" "$SCHEMA_DEST"
fi

step "4/4 caches"
if [ -z "$DESTDIR" ] && command -v update-desktop-database >/dev/null; then
  run ${SUDO:+"$SUDO"} update-desktop-database "$(dirname "$DESKTOP_DEST")"
fi
if command -v restorecon >/dev/null && [ -n "$SUDO" ]; then
  asroot restorecon -F "$BIN_DEST" "$DESKTOP_DEST" "$SERVICE_DEST" "$FM_SERVICE_DEST" "$INTENTS_DEST" "$SCHEMA_DEST"
fi

step "done"
note "Open Files from the launcher. To open folders in it: xdg-mime default org.quire.Files.desktop inode/directory"
note "Another file manager that owns the file-manager service name (GNOME's, from /usr/share) keeps it until it is removed."
