# yrs (pure Dart)

> 🚧 **Experimental — not for production.** APIs unstable, breaking changes between `0.0.x` releases. Pin to specific git commits.

Pure-Dart bindings for [`yrs`](https://github.com/y-crdt/y-crdt) — the official Rust port of [Yjs](https://yjs.dev/), a CRDT library.

## When to use this package vs `yrs_flutter`

| You're building | Use |
|---|---|
| Flutter app (any platform) | [`yrs_flutter`](../yrs_flutter) — cargokit handles native libs |
| Pure-Dart project (Dart VM, server, CLI, Serverpod backend) | **`yrs`** (this package) |
| Both, in a monorepo | Both — `yrs_flutter` re-exports `yrs`'s API |

## Installation (pre-1.0, git dep)

```yaml
dependencies:
  yrs:
    git:
      url: https://github.com/terra-firma-labs/yrs-dart.git
      ref: <commit-sha>
      path: packages/yrs
```

## Compiling the native library (pure-Dart consumers)

This package wraps a Rust crate. Pure-Dart consumers need to build the native library themselves — there's no automatic distribution outside Flutter (yet — see [Roadmap](#roadmap)).

```sh
cd path/to/your/project
git clone https://github.com/terra-firma-labs/yrs-dart.git
cd yrs-dart/packages/yrs/rust
cargo build --release
# → produces target/release/libyrs_bindings.{so,dylib,dll}
```

Then point Dart's `DynamicLibrary` at the produced library — see the generated `lib/src/rust/frb_generated.io.dart` for the loading hook signature.

For Flutter projects, use [`yrs_flutter`](../yrs_flutter) instead — it auto-compiles via cargokit.

## Usage

```dart
import 'package:yrs/yrs.dart';

Future<void> main() async {
  await RustLib.init();

  final doc = YrsDoc.newEmpty();
  doc.getMap(name: 'root')
    ..set('title', 'Hello')
    ..set('count', 42);

  final blob = doc.save();
  final reloaded = YrsDoc.fromBytes(blob: blob);
  print(reloaded.getMap(name: 'root').json()); // {"title":"Hello","count":42}
}
```

## Roadmap

- [ ] Native lib distribution for pure-Dart contexts via `dart:hook` build hooks (`native_toolchain_rust`)
- [ ] Container-level / deep observation, YText formatting, XML, sub-docs, awareness
- [ ] Pub.dev publish

See [`../../README.md`](../../README.md) for the monorepo overview and [`../../docs/`](../../docs/) for design notes.

## License

[MIT](../../LICENSE). Copyright (c) 2026 Jakob Calvén.
