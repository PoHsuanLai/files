# files

files is a file manager for Linux desktops, built on the [quire](https://github.com/PoHsuanLai/quire)
design system. It follows the Mac's Finder: a sidebar of places, list and icon views (with thumbnails), tabs,
Quick Look on Space, a preview pane (Shift+Command+P), tags, undo for every change, and the freedesktop.org standards that make it the
desktop's file manager (the `org.freedesktop.FileManager1` service, the Trash, `mimeapps.list`).

It is early. `ARCHITECTURE.md` is the map, `CONVENTIONS.md` the rules.

## Build

You need `rustup` (the toolchain in `rust-toolchain.toml` is installed on first use) and `git`.

```
cargo build --release
```

## Install

`dist/install.sh` builds files in release and puts everything below under `/usr/local` (or
`FILES_PREFIX`), writing the installed path into the `Exec=` lines. It asks for root only when the
place is not yours; `--dry-run` prints every step and changes nothing, and `dist/uninstall.sh`
takes it all out again. `FILES_BIN` installs a binary you built instead; `FILES_DESTDIR` stages
into a scratch root.

To do it by hand, `cargo build --release` makes `target/release/files`. Put these where the desktop looks (the
prefix is `/usr` for a package, `~/.local` for one person; the `Exec=` lines of the two service
files name `/usr/bin/files`, so change them for another prefix):

| File | Goes to |
| --- | --- |
| `target/release/files` | `<prefix>/bin/files` |
| `crates/files/dist/org.quire.Files.desktop` | `<prefix>/share/applications/` |
| `crates/files/dist/org.quire.Files.FileManager1.service` | `<prefix>/share/dbus-1/services/` |
| `crates/files/dist/org.quire.Files.service` | `<prefix>/share/dbus-1/services/` |
| `dist/intents/org.quire.Files.toml` | `<prefix>/share/quire/intents/` |

Files is one running app. A second `files FOLDER` hands the folder to the first and exits. While it
runs, it answers `org.freedesktop.FileManager1` (`ShowFolders`, `ShowItems`, `ShowItemProperties`),
unless another file manager already owns that name; a program that calls it while Files is closed
starts it through the service file. To make it the default for folders:
`xdg-mime default org.quire.Files.desktop inode/directory`.

## Licence

MIT or Apache-2.0, at your option.
