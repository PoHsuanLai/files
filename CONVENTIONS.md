# Conventions

files follows quire's `CONVENTIONS.md` and `CONSUMING.md`, and the layering rubric every repo of the
desktop shares. In short:

- Every crate is a library with crate docs and a doctest; the binary is thin.
- Small typed structs and enums; illegal states are not representable (see Types below).
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

## Types

Plain Rust first (owner, 2026-10-28; replaces "no bools, typed enums everywhere"):

- **bool is fine** for fields, return values, locals and private functions. Use an enum instead only
  when a public function would read as `f(true, false)`, a third state is realistic, or the value is
  written to disk or the bus where a named word reads better.
- **Newtypes only when they hold a rule:** parsed or validated (an app name, a URL), a unit (`Tokens`,
  `Seconds`), or an id that must not be mixed with another. Otherwise plain `String`, `u32`, etc.
- **Enums for closed sets the code branches on.** A set that grows (apps, providers, programs,
  phrases, window classes) is data: a string id or a shipped file, not a variant list.
- **Behaviour that differs by case is a trait with default methods**, overridden where a case
  differs. Generics by default; `Box<dyn Trait>` is fine when the implementation is picked from
  configuration at run time.
- **One fact, one type.** No mirror enums across crates (`X`/`XKind`/`XWire`/`XState`); the lower
  crate owns it. A field-less view of an enum is a method, not a second declaration.
- **Two-way outcomes are `Result` or `Option`**, not a two-arm enum. "Absent or one real thing" is
  `Option<T>`.

And, for every repo of the shell:

1. A two-way enum that is not persisted or on the bus is a `bool`, an `Option` or a `Result`.
2. One fact, one type, owned by the lowest crate that needs it; an upper crate never declares a copy
   to map into.
3. A per-variant property used in more than one place is a row (a table or a method on the type),
   never a second `match` in another file.
4. Persisted words keep their spelling: change the Rust type, not the word in the file.
