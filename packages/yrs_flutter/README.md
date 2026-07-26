# yrs_flutter

> 🚧 **Experimental — not for production.** APIs unstable, breaking changes between `0.0.x` releases. Pin to specific git commits.

Flutter bindings for [`yrs`](https://github.com/y-crdt/y-crdt) — the official Rust port of [Yjs](https://yjs.dev/). Thin Flutter wrapper around the [`yrs`](../yrs) Dart package; adds automatic native library compilation and distribution via cargokit.

## When to use this package vs `yrs`

| You're building | Use |
|---|---|
| **Flutter app** (iOS, Android, macOS, Linux, Windows, Flutter Web `--wasm`) | **`yrs_flutter`** (this package) |
| Pure-Dart project (Dart VM, server, CLI, Serverpod backend) | [`yrs`](../yrs) |

`yrs_flutter` re-exports the entire `yrs` API. You depend on `yrs_flutter`; cargokit compiles the underlying Rust crate as part of the Flutter build. No separate Rust toolchain setup needed for native targets (iOS/Android/macOS/Linux/Windows). For Flutter Web `--wasm`, see [Web setup](#web-setup) below.

## Installation (pre-1.0, git dep)

```yaml
dependencies:
  yrs_flutter:
    git:
      url: https://github.com/terra-firma-labs/yrs-dart.git
      ref: <commit-sha>
      path: packages/yrs_flutter
```

## Usage

```dart
import 'package:yrs_flutter/yrs_flutter.dart';

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

For a runnable example, see [`example/`](./example).

## Web setup

Flutter Web requires a separate Rust → WASM build step from your application's root, before `flutter build web`:

```sh
flutter_rust_bridge_codegen build-web --release
```

This produces `web/pkg/*.wasm` artifacts that get bundled when you run `flutter build web` (or `flutter run -d chrome --wasm`). No `getrandom` workaround needed — yrs uses `fastrand` internally, which builds cleanly on `wasm32-unknown-unknown`.

Web hosting requires the following HTTP response headers because Skwasm needs `SharedArrayBuffer`:

```
Cross-Origin-Opener-Policy: same-origin
Cross-Origin-Embedder-Policy: require-corp
```

`flutter run -d chrome` sets these automatically in development. Production hosts need to be configured.

## License

[MIT](../../LICENSE). Copyright (c) 2026 Jakob Calvén.
