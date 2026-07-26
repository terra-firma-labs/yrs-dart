/// Pure-Dart bindings for [yrs](https://github.com/y-crdt/y-crdt) — the
/// official Rust port of [Yjs](https://yjs.dev/), a CRDT library for
/// building local-first and collaborative apps.
///
/// This package is pure Dart — it works on the Dart VM (server contexts,
/// CLI tools, Serverpod backends) and inside Flutter apps. For Flutter-only
/// use, the sibling [`yrs_flutter`](https://pub.dev/packages/yrs_flutter)
/// package layers on cargokit-driven native library distribution so you
/// don't need to compile the Rust crate yourself.
///
/// **Status: experimental; not recommended for production.** APIs are
/// unstable and breaking changes will happen without notice. Pin to specific
/// git commits or specific versions.
///
/// Example:
///
/// ```dart
/// import 'package:yrs/yrs.dart';
///
/// Future<void> main() async {
///   await RustLib.init();
///
///   final doc = YrsDoc.newEmpty();
///   final root = doc.getMap(name: 'root');
///   root.set('title', 'Hello');
///   final blob = doc.save();
///   final reloaded = YrsDoc.fromBytes(blob: blob);
/// }
/// ```
///
/// See https://github.com/terra-firma-labs/yrs-dart for the full README,
/// including how to compile and load the native library in pure-Dart
/// contexts.
library;

export 'src/dart/sugar.dart'
    show
        YMap,
        YArray,
        YText,
        YUndoManager,
        YCancelableUndoCapture,
        YrsDocSugar,
        YrsMapSugar,
        YrsArraySugar,
        yrsArrayEntry,
        yrsMapEntry,
        yrsArrayAtMapKey;
export 'src/rust/api/yrs_array.dart' show YrsArray;
export 'src/rust/api/yrs_doc.dart'
    show
        YrsDoc,
        YrsPathSegment,
        YrsTransferLocation,
        YrsTransferMode,
        YrsTransferOutcome;
export 'src/rust/api/yrs_map.dart' show YrsMap;
export 'src/rust/api/yrs_text.dart' show YrsText;
export 'src/rust/frb_generated.dart' show RustLib;
