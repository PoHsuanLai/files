# files

files is a file manager for Linux desktops, built on the [quire](https://github.com/PoHsuanLai/quire)
design system. It follows the Mac's Finder: a sidebar of places, list and icon views, tabs,
Quick Look on Space, tags, undo for every change, and the freedesktop.org standards that make it the
desktop's file manager (the `org.freedesktop.FileManager1` service, the Trash, `mimeapps.list`).

It is early. `ARCHITECTURE.md` is the map, `CONVENTIONS.md` the rules.

## Build

You need `rustup` (the toolchain in `rust-toolchain.toml` is installed on first use) and `git`.

```
cargo build --release
```

## Install

`cargo build --release` makes `target/release/files`. Put these where the desktop looks (the
prefix is `/usr` for a package, `~/.local` for one person; the `Exec=` lines of the two service
files name `/usr/bin/files`, so change them for another prefix):

| File | Goes to |
| --- | --- |
| `target/release/files` | `<prefix>/bin/files` |
| `crates/files/dist/org.quire.Files.desktop` | `<prefix>/share/applications/` |
| `crates/files/dist/org.freedesktop.FileManager1.service` | `<prefix>/share/dbus-1/services/` |
| `crates/files/dist/org.quire.Files.service` | `<prefix>/share/dbus-1/services/` |
| `dist/intents/org.quire.Files.toml` | `<prefix>/share/quire/intents/` |

Files is one running app. A second `files FOLDER` hands the folder to the first and exits. While it
runs, it answers `org.freedesktop.FileManager1` (`ShowFolders`, `ShowItems`, `ShowItemProperties`),
unless another file manager already owns that name; a program that calls it while Files is closed
starts it through the service file. To make it the default for folders:
`xdg-mime default org.quire.Files.desktop inode/directory`.

## Licence

MIT or Apache-2.0, at your option.
