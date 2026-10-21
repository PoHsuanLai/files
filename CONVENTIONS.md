# Conventions

files follows quire's `CONVENTIONS.md` and `CONSUMING.md`, and the layering rubric every repo of the
desktop shares. In short:

- Every crate is a library with crate docs and a doctest; the binary is thin.
- Small typed structs and enums; newtypes for every kind of value; no `bool` fields or parameters
  (an enum with two named variants instead); illegal states are not representable.
- Pure logic returns effects; I/O sits behind traits with fakes behind a `testing` feature.
- Errors are typed (`thiserror`), one `error.rs` per crate when a crate has more than one; no `anyhow`
  in libraries; no `unwrap`, `expect` or `panic` on library paths (tests may unwrap).
- No environment reads or global state in libraries; `#[non_exhaustive]` on public types that will grow;
  builders instead of long argument lists; files under about 400 lines.
- No `unsafe`.
- Words a person reads say what happened in plain words, with no protocol or developer names.
- Tests: one `tests/it/main.rs` binary per crate, table rows over new tests, never the real home folder
  or session bus (scratch XDG folders and a private bus).
- Dependencies: quire and anyview by git rev, never by path; licences per `deny.toml` (no GPL or LGPL).
- Commits carry no trailers.
