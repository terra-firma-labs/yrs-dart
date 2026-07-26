import 'dart:convert';

import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart';

import '../rust/api/values.dart';
import '../rust/api/yrs_array.dart';
import '../rust/api/yrs_doc.dart';
import '../rust/api/yrs_map.dart';
import '../rust/api/yrs_text.dart';
import '../rust/api/yrs_undo.dart' as raw;

/// Idiomatic alias for [YrsMap]. The `Y` prefix matches Yjs JS conventions.
typedef YMap = YrsMap;

/// Idiomatic alias for [YrsArray].
typedef YArray = YrsArray;

/// Idiomatic alias for [YrsText].
typedef YText = YrsText;

/// Wraps yrs's [`UndoManager`](https://docs.rs/yrs/latest/yrs/undo/struct.UndoManager.html)
/// with two-origin tracking: only direct Dart mutations (`YMap.set`,
/// `YArray.push`, etc.) are tracked. Updates applied via [YrsDoc.applyUpdate]
/// (i.e. remote-peer updates) deliberately do NOT enter the undo stack.
///
/// Use one manager, and the document it tracks, from a single isolate. Sharing
/// either across isolates is unsupported: the underlying history manager
/// detaches its observers by taking exclusive access to the document as it is
/// dropped, and drops happen routinely during ordinary history operations, so
/// a concurrent transaction on another thread takes the process down rather
/// than raising. One isolate per document is the ordinary Flutter shape, so
/// this costs nothing in practice.
class YUndoManager {
  /// Construct an undo manager scoped to [scope] containers. `scope` accepts
  /// instances of [YMap], [YArray], or [YText] (mixed lists supported).
  ///
  /// `scope` must be non-empty — empty scope would silently no-op every
  /// undo() call. Use [addScope] for late-joining containers.
  YUndoManager(
    YrsDoc doc, {
    required List<Object> scope,
    Duration captureTimeout = const Duration(milliseconds: 500),
  })  : assert(
          scope.isNotEmpty,
          'YUndoManager scope must not be empty. Pass at least one '
          'YMap/YArray/YText, then use addScope() for late-joining '
          'containers.',
        ),
        _raw = raw.YrsUndoManager(
          doc: doc,
          scope: scope.map(_toScopeItem).toList(),
          captureTimeoutMillis: BigInt.from(captureTimeout.inMilliseconds),
        );

  final raw.YrsUndoManager _raw;

  /// Returns `true` if something was undone.
  ///
  /// While a cancelable capture is open this reaches only what that session
  /// wrote: history older than the capture is held aside for its duration and
  /// cannot be reached, undone or otherwise, until the capture closes. That is
  /// what makes a cancellation able to restore the document whatever the
  /// session did. [canUndo] reports the same reachability.
  bool undo() => _raw.undo();

  /// Returns `true` if something was redone. While a capture is open this
  /// covers the session's own work only, as [undo] does.
  bool redo() => _raw.redo();

  /// Whether [undo] would change anything right now.
  bool get canUndo => _raw.canUndo();

  /// Whether [redo] would change anything right now.
  bool get canRedo => _raw.canRedo();

  /// Total tracked undo depth, including history an open capture is holding
  /// aside. Use [canUndo] to ask what is reachable right now.
  ///
  /// A depth is **not a stable identifier for a step.** Depth falls when a
  /// capture finishes, because finishing collapses everything the session
  /// pushed into one item, and later writes then re-issue the depths the
  /// collapse freed. So the same number can name different history states at
  /// different times. It is usable to key per-step metadata only while no
  /// capture opens or closes in between; a caller that must survive a capture
  /// needs its own identifier.
  BigInt get undoStackLength => _raw.undoStackLen();

  /// Total redo depth, paired with [undoStackLength] and counting held history
  /// on the same terms. The same stability caveat applies.
  BigInt get redoStackLength => _raw.redoStackLen();

  /// Add a container to the tracked scope after construction. Mutations on
  /// `container` from this point forward will be undoable.
  void addScope(Object container) =>
      _raw.addScope(item: _toScopeItem(container));

  /// Equivalent of Yjs's `stopCapturing()`. Resets the merge-window timer so
  /// the next mutation starts a fresh stack item rather than merging into the
  /// previous one.
  void reset() => _raw.reset();

  /// Drop all tracked history.
  ///
  /// During an open capture the history held aside is dropped too, so closing
  /// the capture afterwards cannot hand back anything cleared here. Clearing
  /// also destroys the session's own stack items, which are the only record of
  /// what a cancellation would walk back: the capture still closes, but
  /// [cancelCancelableCapture] then leaves the document as the session left it
  /// rather than reverting part of it. Close the capture first if the
  /// session's writes were meant to be discarded.
  void clear() => _raw.clear();

  /// Start one explicitly bounded undo capture that can be committed or
  /// cancelled without disturbing older history.
  ///
  /// Returns `null` when another cancelable capture is already active.
  YCancelableUndoCapture? beginCancelableCapture() {
    final id = _raw.beginCancelableCapture();
    return id == null ? null : YCancelableUndoCapture._(this, id);
  }

  /// Commit [capture] as one undo item. A stale token or a token from another
  /// manager fails closed and returns `false`.
  bool finishCancelableCapture(YCancelableUndoCapture capture) {
    if (!identical(capture._owner, this) || capture._closed) return false;
    final finished = _raw.finishCancelableCapture(captureId: capture._id);
    if (finished) capture._closed = true;
    return finished;
  }

  /// Revert [capture] entirely, returning the document to its pre-session
  /// state however many stack items the session spanned and whatever it did —
  /// including writing, undoing its own writes, or leaving no net change.
  ///
  /// Undo history older than the capture remains available, redo history that
  /// existed before it began is restored, and the cancellation itself adds no
  /// undo or redo unit. A stale or mismatched token fails closed. The one case
  /// that does not revert is a capture whose history [clear] destroyed; see
  /// there.
  bool cancelCancelableCapture(YCancelableUndoCapture capture) {
    if (!identical(capture._owner, this) || capture._closed) return false;
    final cancelled = _raw.cancelCancelableCapture(captureId: capture._id);
    if (cancelled) capture._closed = true;
    return cancelled;
  }

  /// Release the underlying yrs UndoManager. After dispose the manager is
  /// inert (undo/redo are no-ops returning false).
  void dispose() => _raw.dispose();
}

/// Opaque capability for one active [YUndoManager] capture.
///
/// Tokens are manager-bound and single-use, so one caller cannot cancel
/// an unrelated undo item accidentally.
final class YCancelableUndoCapture {
  YCancelableUndoCapture._(this._owner, this._id);

  final YUndoManager _owner;
  final BigInt _id;
  bool _closed = false;
}

raw.YrsScopeItem _toScopeItem(Object container) {
  if (container is YrsMap) {
    return raw.YrsScopeItem.fromMap(map: container);
  }
  if (container is YrsArray) {
    return raw.YrsScopeItem.fromArray(array: container);
  }
  if (container is YrsText) {
    return raw.YrsScopeItem.fromText(text: container);
  }
  throw ArgumentError(
    'YUndoManager scope must be YMap, YArray, or YText. '
    'Got ${container.runtimeType}.',
  );
}

/// Caches the broadcast stream per `YrsDoc`. Without this, each `doc.updates`
/// access would register a fresh observer — a 1:N callback fan-out only
/// [YrsDoc.dispose] can clean up.
final _docUpdates = Expando<Stream<Uint8List>>();

extension YrsDocSugar on YrsDoc {
  /// Broadcast stream of v1-encoded update blobs, one per committed
  /// transaction. Each blob is consumable by [applyUpdate] on a peer doc.
  /// Inert after [dispose].
  Stream<Uint8List> get updates =>
      _docUpdates[this] ??= observeUpdates().asBroadcastStream();

  /// Move or deep-copy one document entry in a single local yrs transaction.
  ///
  /// Returns [YrsTransferOutcome.unchanged] when the request is already
  /// satisfied: an array drop on either adjacent insertion gap, or the same map
  /// slot. Otherwise the outcome names which mechanism ran —
  /// [YrsTransferOutcome.moved] for a real CRDT move within one list, or
  /// [YrsTransferOutcome.reparented] when the parent changed and the entry had
  /// to be copied and removed instead, which discards a peer's concurrent edits
  /// to that subtree and invalidates live handles to it.
  ///
  /// Throws if the transfer is rejected — an unknown path, a container kind the
  /// document contradicts, a missing source, an occupied target, an
  /// out-of-range index, or a target inside the source. A rejected transfer
  /// leaves the document untouched, so a `try` around this call does not need
  /// to undo anything.
  YrsTransferOutcome transferEntry({
    required YrsTransferMode mode,
    required YrsTransferLocation source,
    required YrsTransferLocation target,
  }) =>
      transferEntryAtomically(mode: mode, source: source, target: target);
}

/// Addresses one array entry (or insertion gap when used as a target).
YrsTransferLocation yrsArrayEntry({
  required List<Object> path,
  required int index,
}) {
  if (index < 0) {
    throw RangeError.value(index, 'index', 'must be non-negative');
  }
  return YrsTransferLocation.array(
    path: _toYrsPath(path),
    index: index,
  );
}

/// Addresses one map child slot.
YrsTransferLocation yrsMapEntry({
  required List<Object> path,
  required String key,
}) =>
    YrsTransferLocation.map(path: _toYrsPath(path), key: key);

List<YrsPathSegment> _toYrsPath(List<Object> path) {
  if (path.isEmpty) {
    throw ArgumentError.value(path, 'path', 'must not be empty');
  }
  return path.indexed.map((entry) {
    final (index, segment) = entry;
    if (segment is String) {
      return YrsPathSegment.key(segment);
    }
    if (segment is int && segment >= 0) {
      return YrsPathSegment.index(segment);
    }
    throw ArgumentError.value(
      segment,
      'path[$index]',
      'must be a string key or non-negative integer index',
    );
  }).toList(growable: false);
}

/// Destination-only list slot beneath a map. The list is created atomically
/// when [key] is absent; an existing non-list value is rejected before write.
YrsTransferLocation yrsArrayAtMapKey({
  required List<Object> parentPath,
  required String key,
  required int index,
}) {
  if (index < 0) {
    throw RangeError.value(index, 'index', 'must be non-negative');
  }
  return YrsTransferLocation.arrayAtMapKey(
    parentPath: _toYrsPath(parentPath),
    key: key,
    index: index,
  );
}

extension YrsMapSugar on YrsMap {
  /// Set a scalar value. `value` may be `String`, `int`, `double`, `bool`,
  /// `null`, or `Uint8List`. Throws [ArgumentError] for other types.
  ///
  /// To insert nested CRDT containers, use [setMap], [setArray], [setText].
  void set(String key, Object? value) =>
      // ignore: invalid_use_of_visible_for_overriding_member, library_private_types_in_public_api
      set_(key: key, value: _toYInValue(value));

  /// Returns the scalar value (Dart-native type) or container handle
  /// (`YMap`/`YArray`/`YText`). `null` if the key is absent.
  Object? get(String key) {
    final out = get_(key: key);
    return out == null ? null : _fromYOutValue(out);
  }
}

extension YrsArraySugar on YrsArray {
  /// Insert a scalar value at [index]. `value` may be `String`, `int`,
  /// `double`, `bool`, `null`, or `Uint8List`.
  void insertValue(int index, Object? value) =>
      insert(index: index, value: _toYInValue(value));

  /// Append a scalar value. `value` may be `String`, `int`, `double`, `bool`,
  /// `null`, or `Uint8List`.
  void pushValue(Object? value) => push(value: _toYInValue(value));

  /// Returns the scalar value (Dart-native type) or container handle
  /// (`YMap`/`YArray`/`YText`). `null` if `index` is out of bounds.
  Object? at(int index) {
    // ignore: invalid_use_of_visible_for_overriding_member, library_private_types_in_public_api
    final out = get_(index: index);
    return out == null ? null : _fromYOutValue(out);
  }
}

YInValue _toYInValue(Object? value) {
  if (value == null) return const YInValue.null_();
  if (value is String) return YInValue.string(value);
  if (value is bool) return YInValue.bool(value);
  if (value is int) return YInValue.int(PlatformInt64Util.from(value));
  if (value is double) return YInValue.double(value);
  if (value is Uint8List) return YInValue.bytes(value);
  throw ArgumentError(
    'Unsupported value type: ${value.runtimeType}. '
    'Use String, int, double, bool, null, or Uint8List for scalars; '
    'use setMap()/setArray()/setText() for nested containers.',
  );
}

Object? _fromYOutValue(YOutValue out) {
  // If kind() and the matching accessor ever disagree, throw a diagnostic
  // instead of a generic null-check failure.
  T require<T>(YOutKind kind, T? value) =>
      value ??
      (throw StateError(
        'YOutValue.kind() returned $kind but the '
        'matching accessor returned null — kind/accessor invariant '
        'violated; check the YOutValue/YOutKind definitions in values.rs.',
      ));
  switch (out.kind()) {
    case YOutKind.string:
      return require(YOutKind.string, out.asString());
    case YOutKind.int:
      return _platformIntToInt(require(YOutKind.int, out.asInt()));
    case YOutKind.double:
      return require(YOutKind.double, out.asDouble());
    case YOutKind.bool:
      return require(YOutKind.bool, out.asBool());
    case YOutKind.null_:
      return null;
    case YOutKind.bytes:
      return require(YOutKind.bytes, out.asBytes());
    case YOutKind.jsonArray:
      return jsonDecode(require(YOutKind.jsonArray, out.asJsonArray()))
          as List<dynamic>;
    case YOutKind.jsonMap:
      return jsonDecode(require(YOutKind.jsonMap, out.asJsonMap()))
          as Map<String, dynamic>;
    case YOutKind.map:
      return require(YOutKind.map, out.asMap());
    case YOutKind.array:
      return require(YOutKind.array, out.asArray());
    case YOutKind.text:
      return require(YOutKind.text, out.asText());
  }
}

/// `PlatformInt64` is `int` on native, `BigInt` on web.
int _platformIntToInt(Object i) {
  // ignore: unnecessary_type_check
  if (i is int) return i;
  return (i as BigInt).toInt();
}
