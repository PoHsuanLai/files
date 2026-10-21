# Architecture

## 1. Layers

Every crate is a library except `files`, the binary, which parses its arguments, builds the one
runtime, adapts the desktop crate to the window's seams and launches the window. Dependencies point down; `scripts/check-boundary.sh` holds the table
below and fails on any edge it does not allow, or on a crate it does not list.

| Layer | Crate | Holds |
|---|---|---|
| 0, data | `files-core` | The vocabulary: names, entries, locations, views, sort, conflicts, progress and plans of work (`OpPlan`), trashed items, as plain data. Names no other workspace crate and does no I/O. |
| 0, library | `files-mime` | shared-mime-info, `mimeapps.list` and `mimeinfo.cache`: a file's type, the apps that open it, the default one, its icon name. Parsers take text; a thin loader reads the XDG folders. |
| 0, library | `files-fs` | The disk behind traits: listing a folder and the facts of one item (Get Info), watching it, the remembered view of each folder (`PrefsStore`, kept as TOML in the app's state folder by quire's `ds-helpers` file helpers), tags (`user.xdg.tags`) and the walk that finds a colour's files. Fakes behind the `testing` feature. |
| 0, library | `files-trash` | The freedesktop.org Trash behind a trait: move to it, list it, put back, delete now, empty, the per-volume `.Trash-$uid` folders. A fake keeps a Trash in a scratch folder. |
| 0, library | `files-ops` | Copy, move, trash and rename as a queue: progress, cancel, conflict questions (keep both, replace, skip, stop), an undo journal. Each job is submitted with a caller's tag (`Worker<T>`) that comes back with every event of it. Plain threads and channels; links are copied as links. |
| 1, logic | `files-model` | Pure state machines, `step(state, input, now) -> (state, effects)`: the browser (history, selection, rename, Go to Folder, type-to-select, sort and view per folder, tags, disks), tabs (`tabs/`), the queue as shown (work and questions), Quick Look and Get Info. No I/O. |
| 2, desktop | `files-desktop` | The only crate that names a bus, docket or porter: `org.freedesktop.FileManager1`, the app's intents (`IntentProvider1`), UDisks2 on the system bus, launching apps with an activation token, one instance. |
| 3, UI | `files-ui` | The window, drawn only with quire's components, and the seams it is given (`Opener`, `Apps`, `Volumes`, `Requests`). `edge/` carries out the model's effects against the layer 0 traits and those seams, one module per family (`read`, `ops`, `prefs`, `tagging`, `search`, `volumes`, `look`), and runs anyview's peek worker for Quick Look (its `Card` is drawn inside quire's `Peek`). The sidebar's sections are `places`. |
| 4, binary | `files` | `main`: arguments, the runtime, `ds_blitz::launch`, and the glue between layers 2 and 3: `files-desktop`'s launcher, disks and intents adapted to `files-ui`'s seams, the XDG folders, the session bus. Each file is one adapter. |

## 2. Data and logic

State is data (`files-core` types); logic is `files-model`'s step functions, which take the time as an
argument and return effects as values instead of doing them. The UI renders the state and feeds inputs;
`files-ui/src/edge` turns effects into calls on `files-fs`, `files-trash`, `files-ops` and the seams
(opener, apps, disks) the binary fills from `files-desktop`, and feeds their answers back as inputs. A change to the files is an `OpPlan` in
`Effect::Run`; the edge's one queue does it off the UI thread and reports back as `Input::Op`. A test of behaviour is a table of inputs and
the state and effects they give, with no disk.

## 3. Look

The window is quire's, as mailo's and temor's are: `Ds` with a `SplitView`, the `Sidebar` source list,
the `Toolbar`, `VirtualTable` for the list view and `CollectionView` for the icon view. Both views
read the same browser and answer the same inputs: a pick is an `Input::Click`, and a double click or
Command+Down (their `onopen`) is `Input::Open`. The view is the folder's remembered `ViewMode`, set by the
toolbar's switcher or Command+1 (icons) and Command+2 (list). Nothing missing is drawn locally; it is asked
of quire first. Keys the views leave alone (Return renames, Space is Quick Look, Command+Up and Command+Down
with a modifier, Shift+Command+G for Go to Folder) reach the app's keymap in `files-ui/src/keys.rs`. App CSS, if any, is layout only and passes `ds_lint` Strict with no exceptions.

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

Each offer carries its app's icon as an image file. `DesktopLauncher` is built with
`DesktopLauncher::builder(Catalog, activate)` and takes an `IconLookup` (the theme, behind a
trait; `NoIcons` by default, `FakeIcons` in tests). An `Icon=` that is an absolute path is used as
it is; a name goes to the lookup, and each answer, found or not, is kept for the launcher's life.
The binary's `ThemeIcons` is the real lookup: the theme named in GTK's `settings.ini` or KDE's
`kdeglobals` (hicolor otherwise) through `freedesktop-icons`, at 32 px, PNG or SVG only. The menu
draws the file with quire's image `MenuImage::Source`; the "Other…" sheet leads its rows with it
(`RowLeading::Source`), and an app with none gets quire's plain app glyph there and an empty image
slot in the menu.

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
disk is not just told: `EjectFailure::InUse` (with the programs found in `/proc` by `DiskUsers`, when
there are any) becomes `Question::ForceEject(BusyDisk)`, drawn as quire's `Alert` in Finder's words
with Force Eject (destructive) and Cancel. Confirming is `Effect::ForceEjectVolume`, which the seam
does as `Disks::force_eject`: `Filesystem.Unmount` with `{"force": true}`, then the drive is powered
off as a normal eject does. If that fails it is told as a toast, never asked again; every other
eject failure is a toast.
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

## 8. Tabs

`files-model::Tabs` holds the window's browsers as a non-empty row with one in front. Each tab has a
stable `TabId` (never reused). `Tabs::step` gives an input to the front tab, or opens (`NewTab`,
`OpenInNewTab`), closes (`CloseTab`) and switches (`NextTab`, `PreviousTab`, `ShowTab`) tabs; every
effect it returns is a `Tabbed`, the effect and the tab that asked. The edge carries the tag
through: its listing, watch, prefs, Get Info, folder check and mount threads answer as
`Heard::Tab(tab, input)`, and the window feeds that to `Tabs::heard`, so a listing asked for by a
tab in the background lands in that tab, and an answer for a closed tab is dropped. A watch belongs
to a tab and a place, so two tabs on one folder each follow it. What is no tab's goes to every tab
(`Heard::All`, `Tabs::broadcast`): the disks changing, and the rescans after a change. The queue of
changes tags every job with the tab that asked for it (`Worker<TabId>`), so its started, progress,
question and finished reports all go to that tab whatever other tabs queue in the meantime, and the
rescans of the places it touched go to every tab. If the asking tab was closed since, `Tabs::heard`
drops the progress, answers a question Stop (the queue would wait for ever), tells a failure as a
toast in the front tab, and still updates what Undo would take back. The clipboard and what Undo
would take back are the window's, copied to every tab when one changes them. Closing the last tab is
`Effect::CloseWindow`, which the edge says as `Heard::CloseWindow` and the window turns into the
window handle's `close`. The tab bar is quire's `SegmentedControl` (the strip `TabView` is made of),
drawn only from two tabs on; ⌘T, ⇧⌘] and ⇧⌘[ are files' actions, ⌘W is chordkit's standard Close.
Requests from outside (FileManager1, the intents) act on the front tab. Folder views and Quick Look
are per tab.

## 9. Tags

A file's tags are the comma-separated list in its `user.xdg.tags` extended attribute, as GNOME and KDE
keep them. `files-core` has `Tag`, `TagColour` (Finder's seven) and `Tags`; an `Entry` carries its
`tags`. `files-fs::Tagger` reads and writes them (`XattrTagger`, `FakeTagger`, `NoTags`), and `Tagged`
wraps any `Lister` so every entry it gives has its tags. `Input::Tag(colour)` puts the colour on the
selection, or takes it off when every selected item has it (`Effect::Tag`); the edge does the
read-modify-write on a thread, rescans the folders it touched, and tells "Tags can't be added on this
disk." when the disk keeps no attributes. `Location::Tag(colour)` is a search: `Effect::Find` walks the
home folder (`files_fs::TagWalk`, at most 20,000 entries, links not followed, dot folders not entered)
and streams batches as `Input::Found` and `Input::SearchDone`; `Listing::Searching` shows them while it
walks, with a progress bar. A result row is keyed by its whole path and carries the item it stands for
(`Entry::found`), so Open, Copy and Move to Trash act on the real file.
