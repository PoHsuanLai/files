# Architecture

## 1. Layers

Every crate is a library except `files`, the binary, which only parses its arguments, builds the one
runtime and launches the window. Dependencies point down; `scripts/check-boundary.sh` holds the table
below and fails on any edge it does not allow, or on a crate it does not list.

| Layer | Crate | Holds |
|---|---|---|
| 0, data | `files-core` | The vocabulary: names, entries, locations, views, sort, conflicts, progress and plans of work (`OpPlan`), trashed items, as plain data. Names no other workspace crate and does no I/O. |
| 0, library | `files-mime` | shared-mime-info, `mimeapps.list` and `mimeinfo.cache`: a file's type, the apps that open it, the default one, its icon name. Parsers take text; a thin loader reads the XDG folders. |
| 0, library | `files-fs` | The disk behind traits: listing a folder and the facts of one item (Get Info), watching it, the remembered view of each folder (`PrefsStore`, kept as TOML in the app's state folder by quire's `ds-helpers` file helpers), tags (`user.xdg.tags`), volumes. Fakes behind the `testing` feature. |
| 0, library | `files-trash` | The freedesktop.org Trash behind a trait: move to it, list it, put back, delete now, empty, the per-volume `.Trash-$uid` folders. A fake keeps a Trash in a scratch folder. |
| 0, library | `files-ops` | Copy, move, trash and rename as a queue: progress, cancel, conflict questions (keep both, replace, skip, stop), an undo journal. Plain threads and channels; links are copied as links. |
| 1, logic | `files-model` | Pure state machines, `step(state, input, now) -> (state, effects)`: the browser (history, selection, rename, views per folder), tabs, the sidebar, the queue as shown, Quick Look and Get Info. No I/O. |
| 2, desktop | `files-desktop` | The only crate that names the session bus or docket: `org.freedesktop.FileManager1`, the app's intents, UDisks2, launching apps with an activation token, one instance. |
| 3, UI | `files-ui` | The window, drawn only with quire's components. `edge/` carries out the model's effects against the layer 0 traits, and runs anyview's peek worker for Quick Look (its `Card` is drawn inside quire's `Peek`). |
| 4, binary | `files` | `main`: arguments, the runtime, `ds_blitz::launch`. Nothing else. |

Crates are added in the phase that needs them; the table already names them all.

## 2. Data and logic

State is data (`files-core` types); logic is `files-model`'s step functions, which take the time as an
argument and return effects as values instead of doing them. The UI renders the state and feeds inputs;
`files-ui/src/edge` turns effects into calls on `files-fs`, `files-trash`, `files-ops` and
`files-desktop`, and feeds their answers back as inputs. A change to the files is an `OpPlan` in
`Effect::Run`; the edge's one queue does it off the UI thread and reports back as `Input::Op`. A test of behaviour is a table of inputs and
the state and effects they give, with no disk.

## 3. Look

The window is quire's, as mailo's and temor's are: `Ds` with a `SplitView`, the `Sidebar` source list,
the `Toolbar`, `VirtualTable` for the list view. Nothing missing is drawn locally; it is asked of quire
first. App CSS, if any, is layout only and passes `ds_lint` Strict with no exceptions.

## 4. Quick Look, Get Info and remembered views

Quick Look is state in the browser (`Looking`), shown over the row the keyboard is on, so it follows the
arrow keys. Each time the shown item changes, `step` adds `Effect::Look(item)`; the edge asks anyview's
`PeekWorker` (the latest request wins) and the card comes back as `Heard::Looked`, not as an input: a
picture is not something the pure model holds. Get Info is `InfoView` (closed, loading, shown, failed);
`Effect::ReadInfo` is answered by `Lister::info` as `Input::InfoRead`. A folder's view and sort are saved
with `Effect::SavePrefs` and loaded with `Effect::LoadPrefs`; the edge serves both from one thread so a
save and the load after it keep their order. The store is a seam (`PrefsStore`); the binary is the only
place that says where its file is.
