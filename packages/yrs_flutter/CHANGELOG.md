# Changelog — `yrs_flutter`

## 0.1.0 — atomic transfers and cancelable undo captures

Re-exports everything new from [`yrs` 0.1.0](../yrs/CHANGELOG.md). Same Flutter
build glue (cargokit) — no consumer-side migration beyond the API changes.

### Changed (BREAKING — pre-1.0)

- Authored Dart `int` values now persist as lib0 `BigInt`. JavaScript peers
  observe them as `bigint` and must convert before `JSON.stringify`. See the
  [`yrs` changelog](../yrs/CHANGELOG.md) for the full note.

### Highlights

- `YrsDoc.transferEntryAtomically` — single-transaction move or deep copy of one
  document entry, fully validated before its first write.
- Location builders `yrsArrayEntry`, `yrsMapEntry`, `yrsArrayAtMapKey` and the
  `YrsDocSugar.transferEntry` sugar.
- Cancelable undo captures on `YUndoManager`: `beginCancelableCapture`,
  `finishCancelableCapture`, `cancelCancelableCapture`.

### Fixed

- The package `LICENSE` carried a placeholder instead of the repository's MIT
  license text.

### Removed

- The unused `ffigen` dev dependency. This repository has no FFIgen
  configuration or generated output; bindings come exclusively from Flutter
  Rust Bridge.

## 0.0.2 — full CRDT surface

Re-exports everything new from [`yrs` 0.0.2](../yrs/CHANGELOG.md). Same Flutter
build glue (cargokit) — no consumer-side migration beyond the API changes.

### Highlights

- Container handles: `YMap`, `YArray`, `YText`
- Multi-type values: `String`, `int`, `double`, `bool`, `null`, `Uint8List`
- Sync primitives: `applyUpdate`, `getStateVector`, `encodeStateAsUpdate`
- Document-level observation via `Stream<Uint8List> get updates`
- `YUndoManager` with two-origin tracking (remote-applied updates do NOT
  enter the undo stack)
- `YrsDoc.dispose()` for explicit lifecycle

### Removed (BREAKING — pre-1.0)

- `YrsDoc.putString` and `YrsDoc.json` are gone. Use `doc.getMap(name: 'root').set(...)` / `.json()`.

See [`packages/yrs/CHANGELOG.md`](../yrs/CHANGELOG.md) for the full details.

## 0.0.1 — initial experimental release

First public release of Flutter bindings for [yrs](https://github.com/y-crdt/y-crdt). Validated on iOS, Android, and Flutter Web (`--wasm`).

This package re-exports [`yrs`](../yrs) and adds Flutter-specific native library distribution via cargokit. Consumers don't need a Rust toolchain — cargokit compiles the Rust crate automatically as part of the Flutter build.

API surface (re-exported from `yrs`):

- `YrsDoc.newEmpty()`
- `YrsDoc.fromBytes(blob:)`
- `YrsDoc.putString(key:, value:)`
- `YrsDoc.json()`
- `YrsDoc.save()`

Configuration:

- `default_dart_async: false` in `flutter_rust_bridge.yaml`. Required for Flutter Web `--wasm`.
- Web hosting needs `Cross-Origin-Opener-Policy: same-origin` + `Cross-Origin-Embedder-Policy: require-corp` (Skwasm requires `SharedArrayBuffer`).

Known limitations:

- Web build still requires a separate `flutter_rust_bridge_codegen build-web --release` step from the consuming app's root before `flutter build web`.
- No `YArray`, `YText`, sub-documents, marks, awareness, sync messages — additive to add later.
