/// Flutter bindings for [yrs](https://github.com/y-crdt/y-crdt) — the
/// official Rust port of [Yjs](https://yjs.dev/).
///
/// This package re-exports the entire [`yrs`](https://pub.dev/packages/yrs)
/// API and adds Flutter-specific native library distribution via cargokit.
/// On Flutter (iOS, Android, macOS, Linux, Windows, Flutter Web `--wasm`),
/// the Rust crate is compiled and bundled automatically — consumers don't
/// need to set up a Rust toolchain.
///
/// For pure-Dart contexts (Dart VM, Serverpod backend, CLI tools), use the
/// [`yrs`](https://pub.dev/packages/yrs) package directly.
///
/// **Status: experimental; not recommended for production.** APIs are
/// unstable and breaking changes will happen without notice.
///
/// Example:
///
/// ```dart
/// import 'package:yrs_flutter/yrs_flutter.dart';
///
/// Future<void> main() async {
///   await RustLib.init();
///
///   final doc = YrsDoc.newEmpty();
///   doc.getMap(name: 'root')
///     ..set('title', 'Hello')
///     ..set('count', 42);
///
///   final blob = doc.save();
///   final reloaded = YrsDoc.fromBytes(blob: blob);
/// }
/// ```
///
/// See https://github.com/terra-firma-labs/yrs-dart for the full README.
library;

// Re-export the entire yrs API. Consumers depend on yrs_flutter and get
// everything yrs offers, transparently.
export 'package:yrs/yrs.dart';
