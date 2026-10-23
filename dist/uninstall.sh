#!/usr/bin/env bash
# Remove what dist/install.sh put in place: the desktop entry, the two bus services, the intents
# manifest, the settings schema and the binary. Your files and the Trash are not touched. Run as
# yourself; sudo asks once when the place is not yours. Safe to run again.
#
#   dist/uninstall.sh --dry-run    print every step, change nothing
#   dist/uninstall.sh              remove the files
set -euo pipefail
# shellcheck source=install-lib.sh
source "$(dirname "${BASH_SOURCE[0]}")/install-lib.sh"

usage() { sed -n '2,8p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; }
parse_flags "$@"
[ "$DRY" = 0 ] || echo "dry run: nothing below is executed"

step "1/2 checks"
refuse_root
need_sudo

step "2/2 the entry, the services, the intents, the settings schema and the binary"
gone "$DESKTOP_DEST"
gone "$SERVICE_DEST"
gone "$FM_SERVICE_DEST"
gone "$INTENTS_DEST"
gone "$SCHEMA_DEST"
gone "$BIN_DEST"
if [ -z "$DESTDIR" ] && command -v update-desktop-database >/dev/null && [ -d "$(dirname "$DESKTOP_DEST")" ]; then
  run ${SUDO:+"$SUDO"} update-desktop-database "$(dirname "$DESKTOP_DEST")"
fi

step "done"
note "Kept: your files and the Trash."
