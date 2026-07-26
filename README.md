# yrs for Dart

> 🚧 **Status: Experimental — not recommended for production.**
> Active early development. APIs will break without notice between `0.0.x`
> releases. No support guarantees. If you do use these packages, **pin to
> specific git commits** rather than version ranges.

Dart bindings for [`yrs`][yrs] — the official Rust port of [Yjs][yjs], a CRDT
library for building local-first and collaborative apps. Powered by
[`flutter_rust_bridge`][frb] 2.12.

This monorepo ships **two complementary packages** so consumers can pick the
right shape for their runtime:

| Package | When to use | What you get |
|---|---|---|
| [`yrs`](./packages/yrs) | Pure-Dart contexts (Dart VM, Serverpod backend, CLI tools). Also works in Flutter. | Core Yjs/CRDT semantics. You compile the Rust crate yourself (or via a setup script). |
| [`yrs_flutter`](./packages/yrs_flutter) | Flutter apps (iOS, Android, macOS, Linux, Windows, Flutter Web `--wasm`). | Same API as `yrs`, plus automatic native library distribution via cargokit. Just add the dep, the build system handles the rest. |

Both packages share the same Rust crate (`packages/yrs/rust/`, named
`yrs_bindings`) and the same frb-generated Dart bindings. `yrs_flutter`
re-exports `yrs`'s API and layers on the Flutter-specific build glue.

[yrs]: https://github.com/y-crdt/y-crdt
[yjs]: https://yjs.dev/
[frb]: https://github.com/fzyzcjy/flutter_rust_bridge

## Quick start

### In a Flutter app

```yaml
dependencies:
  yrs_flutter:
    git:
      url: https://github.com/terra-firma-labs/yrs-dart.git
      ref: <commit-sha>
      path: packages/yrs_flutter
```

```dart
import 'package:yrs_flutter/yrs_flutter.dart';

Future<void> main() async {
  await RustLib.init();

  final doc = YrsDoc.newEmpty();
  final root = doc.getMap(name: 'root')
    ..set('title', 'Hello')
    ..set('count', 42)
    ..set('enabled', true);

  final blob = doc.save();
  // ...

  final undo = YUndoManager(doc, scope: [root]);
  undo.undo();
}
```

### In a pure-Dart project (Dart VM, server, etc.)

```yaml
dependencies:
  yrs:
    git:
      url: https://github.com/terra-firma-labs/yrs-dart.git
      ref: <commit-sha>
      path: packages/yrs
```

You'll also need the Rust toolchain to compile the native library — see
[`packages/yrs/README.md`](./packages/yrs/README.md) for the setup.

## Development

This repo uses [Melos](https://melos.invertase.dev/) to coordinate the two
packages.

```sh
# bootstrap (resolves all package deps)
melos bootstrap

# analyze across the workspace
melos run analyze

# format check
melos run format

# tests (where present)
melos run test
```

## License

[MIT](./LICENSE). Copyright (c) 2026 Jakob Calvén.

## Acknowledgments

- [Yjs](https://yjs.dev/) — the CRDT library this project binds to.
- [`yrs`](https://github.com/y-crdt/y-crdt) — the Rust port of Yjs that does
  the actual work.
- [`flutter_rust_bridge`](https://github.com/fzyzcjy/flutter_rust_bridge) —
  the bridge generator that does the heavy lifting of mapping Rust types to
  Dart.
