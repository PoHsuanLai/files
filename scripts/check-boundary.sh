#!/usr/bin/env bash
# The layer rules of ARCHITECTURE.md, checked from `cargo metadata --no-deps` (direct edges, normal
# and build dependencies; dev-dependencies may reach lower layers' fakes):
#   - every files* crate is in the table below (a new crate must be placed in a layer);
#   - a crate names only workspace crates of its own layer or below, and files-core names none;
#   - the pure crates (files-core, files-model) name no I/O crate;
#   - only files-desktop names zbus, docket, porter (and its `prov`) or peephole;
#   - only files-ui and the binary name dioxus or quire's
#     UI crates (ds, ds-blitz, ds-settings, ...); ds-helpers and ds-peer are UI-free and allowed lower.
# Prints each violation as `crate -> dep (rule)` and exits 1 when there is any.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"

# The layer table. Keep it in step with ARCHITECTURE.md section 1.
LAYERS='
files-core 0
files-mime 0
files-fs 0
files-trash 0
files-ops 0
files-model 1
files-desktop 2
files-ui 3
files 4
'

metadata=$(cd "$root" && cargo metadata --format-version 1 --no-deps)

printf '%s\n' "$LAYERS" | python3 -I -c '
import json, re, sys

layers = {}
for line in sys.stdin.read().split("\n"):
    parts = line.split()
    if len(parts) == 2:
        layers[parts[0]] = int(parts[1])

PURE = {"files-core", "files-model"}
io = re.compile(r"^(tokio.*|async-.*|futures.*|notify.*|trash|rustix|nix|libc|xattr|zbus.*|docket.*|dioxus.*|ds-.*|ds)$")
bus = re.compile(r"^(zbus.*|docket.*|porter.*|peephole.*|prov)$")
ui = re.compile(r"^(dioxus.*|blitz.*|ds|ds-(?!helpers$|peer$).*)$")
meta = json.loads(sys.argv[1])
bad = []

for pkg in meta["packages"]:
    name = pkg["name"]
    if not name.startswith("files"):
        continue
    if name not in layers:
        bad.append(f"{name} -> (not in the layer table) (table)")
        continue
    layer = layers[name]
    for dep in pkg["dependencies"]:
        d = dep["name"]
        if dep.get("kind") == "dev":
            continue
        if d in layers:
            if name == "files-core":
                bad.append(f"{name} -> {d} (files-core names no workspace crate)")
            elif layers[d] > layer:
                bad.append(f"{name} -> {d} (layer {layer} names layer {layers[d]})")
            continue
        if name in PURE and io.match(d):
            bad.append(f"{name} -> {d} (a pure crate does no I/O)")
        if bus.match(d) and name != "files-desktop":
            bad.append(f"{name} -> {d} (only files-desktop names the bus)")
        if ui.match(d) and layer < layers["files-ui"]:
            bad.append(f"{name} -> {d} (only files-ui and the binary name the UI)")

for b in bad:
    print(b)
sys.exit(1 if bad else 0)
' "$metadata"
