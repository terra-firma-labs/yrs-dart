# Changelog — `yrs`

## 0.1.0 — atomic transfers and cancelable undo captures

Adds single-transaction document transfers and cancelable undo captures, and
changes how authored Dart integers are persisted.

### Changed

- **BREAKING (pre-1.0).** Authored Dart `int` values now persist as lib0
  `BigInt` instead of collapsing to `Any::Number`. This preserves the
  integer/double scalar distinction across persistence and update transfer,
  which previously did not survive a round trip. JavaScript peers observe these
  values as `bigint`, and passing them straight to `JSON.stringify` throws, so a
  JS peer must convert them first. Only newly authored integers change
  representation; documents written by 0.0.2 are unaffected by this change,
  which also means an integer authored under 0.0.2 remains an `Any::Number` and
  still reads back as a Dart `double`, exactly as it did before.
- Bindings are regenerated with the pinned Flutter Rust Bridge generator
  (2.12.0). `dart_fix` is disabled in `flutter_rust_bridge.yaml`: current Dart
  SDK fixes rewrite the generator's valid Web wire constructor into an
  incompatible zero-argument tear-off, so generated output is kept exactly as
  emitted.
- The generated binding directory carries a self-contained
  `analysis_options.yaml` ignoring one generator-specific inference diagnostic
  that arises from the generator's marker-only generic construction. Nothing
  else in that directory is exempted.
- CI now runs `cargo test`. It previously ran only fmt, clippy, and
  `cargo check`, so the test suite was not gated. A second job scans tracked
  files for machine-specific paths, since the bridge generator writes its own
  working directory into generated bindings that nobody reads in review.
- `freezed` / `freezed_annotation` are new dev dependencies, required by the
  generated union types the transfer API introduces.
- Documented that a `YUndoManager` and the document it tracks belong to a
  single isolate. This has always been the case and is unchanged here, but it
  was never written down: the underlying history manager takes exclusive access
  to the document as it is dropped, and ordinary history operations drop
  managers routinely, so a concurrent transaction on another thread aborts the
  process instead of raising.
- A `moved` outcome emits a yrs move block. yrs and Yjs are the Rust and
  JavaScript implementations of the same CRDT but are not at feature parity:
  moves exist in yrs and were never merged into released Yjs, whose decode
  table has no entry for them. A yrs peer — including another copy of this
  package — handles it normally; a released-Yjs peer would reject the update.
  Noted for completeness, since this package exists to bring **yrs** to Dart.
- Dropped the unused `ffigen` dev dependency from `yrs_flutter`. Bindings come
  exclusively from Flutter Rust Bridge.

### Added

- `YrsDoc.transferEntryAtomically` moves or deep-copies a single document entry
  inside one local transaction. Paths, container kinds, source existence, target
  availability, indexes, and ancestry are validated against that same
  transaction before its first write, so a rejected transfer leaves the document
  byte-identical. A same-list move to either adjacent insertion gap is a
  zero-write no-op.
- `YrsTransferOutcome` reports which mechanism a transfer used, because a move
  within one list and a move between lists are **not the same operation** and
  the caller does not choose between them:

  | Outcome | Meaning |
  |---|---|
  | `unchanged` | Already satisfied; nothing written. |
  | `moved` | A real CRDT move inside one list. The entry keeps its identity, so a peer's concurrent edits to it survive sync and live handles stay valid. |
  | `reparented` | The parent changed, so the entry was deep-copied and the original removed. |
  | `copied` | Deep copy; source left in place. |

  **`reparented` is lossy, by necessity.** Neither yrs nor Yjs can express a
  move between different parents — there is no cross-parent move in the CRDT to
  fall back on — so the entry is recreated under its new parent. A peer's edits
  to that subtree made concurrently with the reparent are discarded when the two
  sides synchronize, and container handles held on the source become invalid
  (reads return nothing; writes land on a deleted branch and produce no visible
  change while still growing the document). Re-resolve handles after a
  `reparented` outcome, and consider whether reparenting is safe to offer while
  remote peers may be editing the same subtree. `changedDocument()` is available
  when only "did anything change" matters.
- `YrsTransferMode`, `YrsTransferLocation`, and `YrsPathSegment` address the
  source and target of a transfer. `YrsTransferLocation.arrayAtMapKey`
  addresses a list slot owned by a map, creating the array when the key is
  absent and rejecting an existing non-array value before any write.
- Typed Dart sugar for the above: `YrsDocSugar.transferEntry`, plus the
  `yrsArrayEntry`, `yrsMapEntry`, and `yrsArrayAtMapKey` location builders.
- Cancelable undo captures on `YUndoManager`: `beginCancelableCapture`,
  `finishCancelableCapture`, and `cancelCancelableCapture`, returning a
  `YCancelableUndoCapture` handle. Finishing collapses everything the session
  pushed into the single undoable step a capture means, however many stack items
  it spanned. Cancelling returns the document to its pre-session state and
  restores the redo history that existed before the capture. History older than
  an open capture is held outside the manager for its duration, so a session
  cannot reach it and cancellation cannot be defeated by what the session did —
  including undoing its own writes or leaving no net change. The one exception
  is a capture whose history `clear()` destroyed; see *Fixed* below. Either
  close path always frees the capture slot and restores the configured merge
  window. Note that `finish` collapses a session to one **undo** step but
  leaves whatever redo entries the session produced as separate steps.
- `YUndoManager.undoStackLength` and `.redoStackLength` report total stack
  depth, counting history an open capture is holding aside; `canUndo` /
  `canRedo` answer what is reachable right now. A depth is not a stable
  identifier for a step — finishing a capture collapses the session into one
  item, so the depth falls and later writes re-issue the freed depths. Key
  per-step state by it only between capture boundaries.

### Fixed

- **A transfer could corrupt a document across container kinds.** Root
  containers are looked up by name and the underlying accessors do not check
  the recorded type, so a root map or text addressed as an array was silently
  cast and written through — a root text could be destructively edited by an
  array-addressed transfer while still reporting its original string, and a
  root could end up holding both map entries and array items, which a peer
  reading it as one kind never sees. The requested kind is now checked against
  the kind the document actually records, before any write.

  A document loaded from bytes carries no root type tags at all — only nested
  container kinds are encoded — so on the path a collaborative editor always
  takes, every root is undeclared and there is no recorded kind to check. Such
  a root is now **rejected** rather than guessed at, with an error naming the
  remedy: read it once through the matching accessor first, which declares its
  kind. Inferring the kind from the root's content was tried and abandoned —
  the type a shared branch really has is decided from raw block state that
  still counts deleted entries, while every accessor reachable from outside
  `yrs` skips them, so a map root whose entries had all been deleted read as
  holding nothing and would have been accepted as any kind at all.
- **A deep enough subtree could crash the process instead of failing.** Copying
  an entry recurses once per nesting level, so a sufficiently deep source can
  overflow the stack and abort — uncatchable, and nesting depth is
  remote-influenced, since a peer can send an arbitrarily deep subtree. Depth
  is now measured iteratively before the copy and before any write, and a
  source nesting deeper than 128 levels is rejected like any other invalid
  transfer. The measurement walks every shape the copy itself recurses through
  — nested maps and arrays, xml elements and fragments, and the values embedded
  in text and xml-text deltas — and is written so that a future `yrs` value
  variant fails to compile rather than passing through unmeasured. A source
  subtree containing a container of undeclared kind is rejected for the same
  reason a root of undeclared kind is: its shape cannot be established from
  here, so it cannot be measured.
- **Cancelling a capture could destroy history instead of restoring it.** A
  session that consumed pre-session history and then wrote left cancellation
  with nothing to revert: the session's write survived and the older step it
  had consumed was gone for good, because a tracked write clears the redo
  stack. Bounding `undo` by a recorded depth does not fix this — `UndoManager`
  pops stack items until one produces a visible change and discards the rest,
  so a session with no net effect falls through its own items and reaches older
  history inside a single call, whatever was checked beforehand. Instead, a
  capture now holds pre-session undo history outside the manager for its whole
  duration, exactly as it already did for redo. The session can only ever undo
  its own work, because nothing else is there to undo, and cancelling lands on
  the pre-session document whatever the session did.
- **`clear()` during a capture handed back history the caller had destroyed.**
  A capture holds pre-session history outside the manager, where
  `UndoManager::clear` could not reach it, and both close paths then restored
  it — so history the caller had explicitly dropped came back. `clear()` now
  drops the held history too. Because it also destroys the session's own stack
  items, which are the only record of what a cancellation would walk back, a
  capture cleared mid-session is no longer revertible: it still closes, but
  cancelling leaves the document as the session left it rather than reverting
  part of it. Both close paths then behave identically, collapsing whatever the
  session wrote after the clear into one undoable step, so cancelling is never
  worse than finishing. Documented on both `clear` and
  `cancelCancelableCapture`.
- A manager dropped without an explicit `dispose()` left every document struct
  its history retained alive for the document's lifetime. `dispose` now also
  clears the live manager's own stacks, and an equivalent release runs from
  `Drop` — written not to raise, since a panic escaping a destructor aborts.
- **A cancelable capture could become permanently unusable.** Any session that
  produced more than one undo stack item — for example typing, undoing,
  redoing, then typing again, since undo and redo reset the merge timer — left
  both close paths refusing, the capture slot occupied forever, and an
  effectively unbounded merge window installed, which silently collapsed all
  later edits into one undo step and disabled redo. Both close paths now handle
  a session of any shape, and every exit restores the merge window, releases the
  held redo history, and frees the slot. Disposing the manager mid-capture also
  releases that history rather than dropping it.
- `packages/yrs_flutter/LICENSE` carried a placeholder instead of the MIT
  license text. The license is now also present in the package and Rust crate
  directories, where wasm-pack discovers it.

## 0.0.2 — first real CRDT surface

Expanded from the v0.0.1 5-method spike to a full CRDT binding sufficient for production editor use cases.

### Added

- `YMap`, `YArray`, `YText` container handles with idiomatic Dart APIs
  (`set`, `get`, `at`, `pushValue`, `setMap` / `setArray` / `setText` factories
  for nested containers, etc.)
- Multi-type values (`String`, `int`, `double`, `bool`, `null`, `Uint8List`)
  via the `YInValue`/`YOutValue` enums
- Sync-protocol primitives on `YrsDoc`: `applyUpdate`, `getStateVector`,
  `encodeStateAsUpdate`
- Document-level observation: `Stream<Uint8List> get updates` (broadcast
  stream of v1-encoded delta blobs, one per committed transaction)
- `YUndoManager` wrapper with explicit scope, `addScope` for late-joining
  containers, and configurable `captureTimeout`
- Hidden two-origin split: local mutations use a sentinel `local` origin;
  `applyUpdate` uses a `remote` origin. The undo manager only tracks the
  local origin, so a user's Cmd-Z never undoes a remote peer's edit
- `YrsDoc.dispose()` (releases held observer subscriptions; the underlying
  yrs `Doc` is `Arc`-backed and freed via standard Dart GC)

### Changed

- `Doc` is now constructed with `OffsetKind::Utf16` so YText indices match
  Dart string indices (Dart strings are UTF-16 internally). Multi-byte
  characters in YText now produce lengths consistent with `String.length`
- Added `yrs/sync` cargo feature (required for `Send + Sync` bounds on
  observer callbacks)

### Removed (BREAKING — pre-1.0)

- `YrsDoc.putString(key:, value:)` — use `doc.getMap(name: 'root').set(key, value)` instead
- `YrsDoc.json()` — use `doc.getMap(name: 'root').json()` instead
- The implicit `"root"` map that v0.0.1 created on `newEmpty()` / `fromBytes()`
  is no longer auto-created. Call `doc.getMap(name: 'root')` explicitly
  (it's get-or-create, so this is a one-line change for migration)

### Deferred to v0.0.3

- Container-level / deep observation (`MapRef::observe`, `observe_deep`,
  `observe_transaction_cleanup`)
- YText formatting / attributes / embeds
- XML types
- Sub-documents
- Snapshots and awareness protocol

## 0.0.1 — initial experimental release

First public release of pure-Dart bindings for [yrs](https://github.com/y-crdt/y-crdt). Toolchain validated on iOS, Android, and Flutter Web (`--wasm`) via the sibling [`yrs_flutter`](../yrs_flutter) package.

API surface intentionally narrow:

- `YrsDoc.newEmpty()`
- `YrsDoc.fromBytes(blob:)`
- `YrsDoc.putString(key:, value:)`
- `YrsDoc.json()`
- `YrsDoc.save()`

Configuration: `default_dart_async: false` in `flutter_rust_bridge.yaml`.

Pure-Dart consumers must compile the Rust crate (`packages/yrs/rust/`) themselves — see the README. Flutter consumers should use `yrs_flutter` instead, which handles compilation automatically via cargokit.
