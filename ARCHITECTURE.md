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
| 2, desktop | `files-desktop` | The only crate that names a bus, docket or porter: `org.freedesktop.FileManager1`, the app's intents (`IntentProvider1`), UDisks2 on the system bus, launching apps with an activation token, one instance. |
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

## 5. Open With, Go to Folder and Copy as Pathname

Open With is a submenu of the row's context menu. The window asks the `Apps` seam (`files-ui`) which
apps open the selected paths; the binary answers it from `DesktopLauncher::apps_for` (the default
first, marked, then the rest by name), and "Other…" puts up a sheet of `all_apps`. A pick is
`Input::OpenWith(AppPick)`, which becomes `Effect::OpenFilesWith`, and the edge hands the paths and
an `OpenAs` to the `Opener` seam. Go to Folder is browser state (`GoTo`): the typed text is expanded
against the home folder the binary passed in (`~`), `Effect::CheckFolder` asks the lister whether it
is a folder, and `Input::FolderChecked` either visits it or leaves the sheet up with its words.
Copy as Pathname is `Effect::CopyText`, written to quire's clipboard by the edge.

## 6. Disks

Mounted and attached disks are the sidebar's Locations after the computer. `files-core` holds the
vocabulary (`Volume`: id, name, kind, mount, eject; `EjectFailure`, `MountFailure`). The model keeps
the list the caller last told (`Input::Volumes`), and `OpenVolume`, `Eject` and their answers
(`Mounted`, `Ejected`) are inputs whose effects (`MountVolume`, `EjectVolume`, `TellVolume`) the edge
carries out. `files-ui` owns the seam (`Volumes`, with `FakeVolumes` behind `testing`); one thread of
the edge lists the disks, lists them again whenever the seam says they changed, and mounts and
ejects in order. `files-desktop::Disks` is the real one: UDisks2 on the system bus (one
`GetManagedObjects` read, a signal stream for hot-plug, `Filesystem.Mount` and `Unmount`, then
`Drive.PowerOff` or `Eject`), with polkit asking the person when the system's rules say so. A busy
disk is told in Finder's words, with the programs found in `/proc` (`DiskUsers`) when there are any.
The binary adapts `Disks` to `Volumes` and runs it on the bus runtime.

## 7. Asks from outside: intents

`dist/intents/org.quire.Files.toml` declares the app's actions (`files.open`, `files.trash`,
`files.new_folder`, `files.copy`, `files.move`, `files.empty_trash`, `files.selection`,
`files.current_folder`); docket generates the command line from it, and `files-desktop::intents`
serves `IntentProvider1` on the name the app already owns. The window is behind `Window`: the binary
turns each `Ask` into a model input sent through `files-ui`'s request channel
(`RequestSender::send`, `send_quietly`), so a change is queued, shown and undone like the person's
own. The two reads and the undo are questions (`RequestSender::ask`) the window answers with a
`Reading`. An undo token names the change (`undo:trash:3`); the window takes back the newest change
only if it is that one. Emptying the Trash only puts up the window's own question.
