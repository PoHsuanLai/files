# files

files is a file manager for Linux desktops, built on the [quire](https://github.com/PoHsuanLai/quire)
design system. It follows the Mac's Finder: a sidebar of places, list, icon, column and gallery views,
Quick Look on Space, tags, undo for every change, and the freedesktop.org standards that make it the
desktop's file manager (the `org.freedesktop.FileManager1` service, the Trash, shared thumbnails,
`mimeapps.list`).

It is early: this is the scaffold. `ARCHITECTURE.md` is the map, `CONVENTIONS.md` the rules.

## Build

You need `rustup` (the toolchain in `rust-toolchain.toml` is installed on first use) and `git`.

```
cargo build --release
```

## Licence

MIT or Apache-2.0, at your option.
