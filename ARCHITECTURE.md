# Architecture

## 1. Layers

Every crate is a library except `files`, the binary, which only parses its arguments, builds the one
runtime and launches the window. Dependencies point down; `scripts/check-boundary.sh` holds the table
below and fails on any edge it does not allow, or on a crate it does not list.

| Layer | Crate | Holds |
|---|---|---|
| 0, data | `files-core` | The vocabulary: names, entries, locations, views, sort, conflicts and plans of work, as plain data. Names no other workspace crate and does no I/O. |
| 0, library | `files-mime` | shared-mime-info, `mimeapps.list` and `mimeinfo.cache`: a file's type, the apps that open it, the default one, its icon name. Parsers take text; a thin loader reads the XDG folders. |
| 0, library | `files-fs` | The disk behind traits: listing a folder, watching it, tags (`user.xdg.tags`), volumes. Fakes behind the `testing` feature. |
| 0, library | `files-trash` | The freedesktop.org Trash: list, put back, delete now, empty, the per-volume `.Trash-$uid` folders. |
| 0, library | `files-ops` | Copy, move, trash and rename as a queue: progress, pause and cancel, conflict questions, an undo journal. |
| 1, logic | `files-model` | Pure state machines, `step(state, input, now) -> (state, effects)`: the browser (history, selection, rename, views per folder), tabs, the sidebar, the queue as shown, Quick Look. No I/O. |
| 2, desktop | `files-desktop` | The only crate that names the session bus or docket: `org.freedesktop.FileManager1`, the app's intents, UDisks2, launching apps with an activation token, one instance. |
| 3, UI | `files-ui` | The window, drawn only with quire's components. `edge/` carries out the model's effects against the layer 0 traits. |
| 4, binary | `files` | `main`: arguments, the runtime, `ds_blitz::launch`. Nothing else. |

Crates are added in the phase that needs them; the table already names them all.

## 2. Data and logic

State is data (`files-core` types); logic is `files-model`'s step functions, which take the time as an
argument and return effects as values instead of doing them. The UI renders the state and feeds inputs;
`files-ui/src/edge` turns effects into calls on `files-fs`, `files-trash`, `files-ops` and
`files-desktop`, and feeds their answers back as inputs. A test of behaviour is a table of inputs and
the state and effects they give, with no disk.

## 3. Look

The window is quire's, as mailo's and temor's are: `Ds` with a `SplitView`, the `Sidebar` source list,
the `Toolbar`, `VirtualTable` for the list view. Nothing missing is drawn locally; it is asked of quire
first. App CSS, if any, is layout only and passes `ds_lint` Strict with no exceptions.
