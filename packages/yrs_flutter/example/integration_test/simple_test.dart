import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:yrs_flutter/yrs_flutter.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  setUpAll(() async => RustLib.init());

  test('multi-type round-trip across save/load', () {
    final doc = YrsDoc.newEmpty();
    doc.getMap(name: 'root')
      ..set('title', 'Hello')
      ..set('count', 42)
      ..set('ratio', 1.5)
      ..set('enabled', true)
      ..set('absent', null)
      ..set('blob', Uint8List.fromList([1, 2, 3]));

    final blob = doc.save();
    final reloaded = YrsDoc.fromBytes(blob: blob);
    final reloadedRoot = reloaded.getMap(name: 'root');

    expect(reloadedRoot.get('title'), equals('Hello'));
    expect(reloadedRoot.get('count'), equals(42));
    expect(reloadedRoot.get('ratio'), equals(1.5));
    expect(reloadedRoot.get('enabled'), equals(true));
    expect(reloadedRoot.get('absent'), isNull);
    expect(reloadedRoot.get('blob'), equals([1, 2, 3]));
  });

  test('nested containers serialize and reload', () {
    final doc = YrsDoc.newEmpty();
    final root = doc.getMap(name: 'root');
    final child = root.setMap(key: 'child')..set('inner', 'value');
    final list = root.setArray(key: 'list')
      ..pushValue('a')
      ..pushValue('b')
      ..pushValue('c');
    final text = root.setText(key: 'text')..insert(index: 0, chunk: 'hello');

    expect(child.get('inner'), equals('value'));
    expect(list.length(), equals(3));
    expect(text.value(), equals('hello'));

    final blob = doc.save();
    final reloaded = YrsDoc.fromBytes(blob: blob);
    final reloadedRoot = reloaded.getMap(name: 'root');

    final reloadedChild = reloadedRoot.get('child') as YMap;
    expect(reloadedChild.get('inner'), equals('value'));

    final reloadedList = reloadedRoot.get('list') as YArray;
    expect(reloadedList.length(), equals(3));
    expect(reloadedList.at(1), equals('b'));

    final reloadedText = reloadedRoot.get('text') as YText;
    expect(reloadedText.value(), equals('hello'));
  });

  test('two docs converge via update + state vector', () {
    final a = YrsDoc.newEmpty();
    final b = YrsDoc.newEmpty();

    a.getMap(name: 'root').set('owner', 'Alice');
    b.getMap(name: 'root').set('count', 7);

    // Compute the diff each peer is missing.
    final diffForB = a.encodeStateAsUpdate(stateVector: b.getStateVector());
    final diffForA = b.encodeStateAsUpdate(stateVector: a.getStateVector());

    a.applyUpdate(update: diffForA);
    b.applyUpdate(update: diffForB);

    final aRoot = a.getMap(name: 'root');
    final bRoot = b.getMap(name: 'root');

    expect(aRoot.get('owner'), equals('Alice'));
    expect(aRoot.get('count'), equals(7));
    expect(bRoot.get('owner'), equals('Alice'));
    expect(bRoot.get('count'), equals(7));
  });

  test('undo manager reverts a sequence of mutations', () {
    final doc = YrsDoc.newEmpty();
    final root = doc.getMap(name: 'root');
    final undo = YUndoManager(
      doc,
      scope: [root],
      captureTimeout: Duration.zero,
    );

    root.set('a', 1);
    root.set('b', 2);

    expect(undo.canUndo, isTrue);
    expect(undo.undo(), isTrue); // reverts b
    expect(root.get('b'), isNull);
    expect(root.get('a'), equals(1));

    expect(undo.redo(), isTrue);
    expect(root.get('b'), equals(2));

    undo.dispose();
  });

  test('undo manager ignores remote-applied updates', () {
    final local = YrsDoc.newEmpty();
    final localRoot = local.getMap(name: 'root');
    final undo = YUndoManager(
      local,
      scope: [localRoot],
      captureTimeout: Duration.zero,
    );

    // Local mutation — should be undoable.
    localRoot.set('local', 'mine');

    // Remote-applied update — should NOT be undoable.
    final peer = YrsDoc.newEmpty();
    peer.getMap(name: 'root').set('remote', 'peer');
    local.applyUpdate(update: peer.save());

    expect(localRoot.get('local'), equals('mine'));
    expect(localRoot.get('remote'), equals('peer'));

    expect(undo.undo(), isTrue);
    expect(localRoot.get('local'), isNull);
    // Remote value must remain — undo only reverted the local mutation.
    expect(localRoot.get('remote'), equals('peer'));

    undo.dispose();
  });

  test('addScope makes a late-created container undoable', () {
    final doc = YrsDoc.newEmpty();
    final root = doc.getMap(name: 'root');
    final undo = YUndoManager(
      doc,
      scope: [root],
      captureTimeout: Duration.zero,
    );

    // late-joining root container, not initially in scope
    final ops = doc.getArray(name: 'ops');
    undo.addScope(ops);

    ops.pushValue('first');
    expect(undo.undo(), isTrue);
    expect(ops.length(), equals(0));

    undo.dispose();
  });

  test('captureTimeout: 0 makes each transaction its own undo step', () {
    final doc = YrsDoc.newEmpty();
    final root = doc.getMap(name: 'root');
    final undo = YUndoManager(doc, scope: [root], captureTimeout: Duration.zero);

    root.set('a', 1);
    root.set('b', 2);
    root.set('c', 3);

    expect(undo.undo(), isTrue);
    expect(root.get('c'), isNull);
    expect(root.get('b'), equals(2));

    expect(undo.undo(), isTrue);
    expect(root.get('b'), isNull);
    expect(root.get('a'), equals(1));

    undo.dispose();
  });

  test('YText insert/remove and UTF-16 length', () {
    final doc = YrsDoc.newEmpty();
    final text = doc.getText(name: 'body')..insert(index: 0, chunk: 'café');

    // UTF-16 default: 4 code units (matches Dart String.length).
    expect(text.length(), equals(4));
    expect(text.value(), equals('café'));
    expect('café'.length, equals(4));

    text.remove(index: 3, length: 1);
    expect(text.value(), equals('caf'));
  });

  test('updates stream emits per-transaction blobs that converge a peer',
      () async {
    final source = YrsDoc.newEmpty();
    final peer = YrsDoc.newEmpty();

    final received = <Uint8List>[];
    final twoEvents = Completer<void>();
    final sub = source.updates.listen((bytes) {
      received.add(bytes);
      if (received.length >= 2 && !twoEvents.isCompleted) {
        twoEvents.complete();
      }
    });

    source.getMap(name: 'root')
      ..set('one', 1)
      ..set('two', 2);

    await twoEvents.future.timeout(
      const Duration(seconds: 3),
      onTimeout: () => throw TimeoutException(
        'updates stream did not deliver 2 events; got ${received.length}',
      ),
    );

    expect(received.length, greaterThanOrEqualTo(2));

    for (final blob in received) {
      peer.applyUpdate(update: blob);
    }

    final peerRoot = peer.getMap(name: 'root');
    expect(peerRoot.get('one'), equals(1));
    expect(peerRoot.get('two'), equals(2));

    // Don't await — controller.onCancel cancels the inner async-generator
    // subscription which can deadlock on certain frb stream paths. Letting it
    // finalize lazily is fine for test cleanup.
    unawaited(sub.cancel());
    source.dispose();
  });

  test('dispose is a no-op for state and does not crash', () {
    final doc = YrsDoc.newEmpty();
    doc.getMap(name: 'root').set('persists', 'after dispose');

    doc.dispose();

    // doc handle is still valid; subscriptions cleared but state intact
    expect(doc.getMap(name: 'root').get('persists'), equals('after dispose'));
    final saved = doc.save();
    expect(saved, isA<Uint8List>());
    expect(saved.length, greaterThan(0));
  });

  test('fromBytes errors cleanly on garbage input', () {
    expect(
      () => YrsDoc.fromBytes(blob: Uint8List(0)),
      throwsA(anything),
    );
    expect(
      () => YrsDoc.fromBytes(blob: Uint8List.fromList([0xff, 0xff, 0xff, 0xff])),
      throwsA(anything),
    );
  });

  test('applyUpdate errors on garbage input without poisoning the doc', () {
    final doc = YrsDoc.newEmpty();
    doc.getMap(name: 'root').set('before', 'value');

    expect(
      () => doc.applyUpdate(update: Uint8List.fromList([0xff, 0xff, 0xff])),
      throwsA(anything),
    );

    // doc remains usable after the failed apply
    expect(doc.getMap(name: 'root').get('before'), equals('value'));
    doc.getMap(name: 'root').set('after', 'still works');
    expect(doc.getMap(name: 'root').get('after'), equals('still works'));
  });

  test('YMap delete + contains + clear + keys', () {
    final doc = YrsDoc.newEmpty();
    final root = doc.getMap(name: 'root')
      ..set('a', 1)
      ..set('b', 2)
      ..set('c', 3);

    expect(root.length(), equals(3));
    expect(root.contains(key: 'b'), isTrue);
    expect(root.contains(key: 'missing'), isFalse);
    expect(root.keys()..sort(), equals(['a', 'b', 'c']));

    root.delete(key: 'b');
    expect(root.contains(key: 'b'), isFalse);
    expect(root.length(), equals(2));

    root.clear();
    expect(root.length(), equals(0));
    expect(root.keys(), isEmpty);
  });

  test('YText handles UTF-16 surrogate pairs (emoji)', () {
    final doc = YrsDoc.newEmpty();
    final text = doc.getText(name: 'body')..insert(index: 0, chunk: 'a😀b');

    // 😀 is a surrogate pair: 2 UTF-16 code units. So length is 4.
    expect(text.length(), equals(4));
    expect('a😀b'.length, equals(4));
    expect(text.value(), equals('a😀b'));

    // Removing the emoji's two code units leaves "ab".
    text.remove(index: 1, length: 2);
    expect(text.value(), equals('ab'));
    expect(text.length(), equals(2));
  });

  test('updates stream goes inert after dispose', () async {
    final doc = YrsDoc.newEmpty();
    final received = <Uint8List>[];
    final firstEvent = Completer<void>();
    final sub = doc.updates.listen((bytes) {
      received.add(bytes);
      if (!firstEvent.isCompleted) firstEvent.complete();
    });

    doc.getMap(name: 'root').set('one', 1);
    await firstEvent.future.timeout(const Duration(seconds: 2));
    final beforeDispose = received.length;

    doc.dispose();

    // Mutations after dispose should not deliver any further events to the
    // existing subscription.
    doc.getMap(name: 'root').set('two', 2);
    await Future<void>.delayed(const Duration(milliseconds: 100));

    expect(received.length, equals(beforeDispose));
    unawaited(sub.cancel());
  });

  test('two peers concurrent-mutation on same key converges deterministically',
      () async {
    final a = YrsDoc.newEmpty();
    final b = YrsDoc.newEmpty();

    a.getMap(name: 'root').set('shared', 'a-wrote-this');
    b.getMap(name: 'root').set('shared', 'b-wrote-this');

    // Cross-apply each peer's full state.
    a.applyUpdate(update: b.save());
    b.applyUpdate(update: a.save());

    final aValue = a.getMap(name: 'root').get('shared');
    final bValue = b.getMap(name: 'root').get('shared');

    // Both peers must agree on the same winner — either is fine, the CRDT
    // tie-break is deterministic but document-clock-dependent.
    expect(aValue, equals(bValue));
    expect(aValue, anyOf(equals('a-wrote-this'), equals('b-wrote-this')));
  });

  test('YUndoManager rejects empty scope at construction', () {
    final doc = YrsDoc.newEmpty();
    expect(
      () => YUndoManager(doc, scope: const []),
      throwsA(isA<AssertionError>()),
    );
  });

  test('legacy json round-trip via getMap', () {
    final doc = YrsDoc.newEmpty();
    final root = doc.getMap(name: 'root');
    root
      ..set('title', 'Hello')
      ..set('subtitle', 'via FRB');

    final firstJson = root.json();
    final blob = doc.save();
    final reloaded = YrsDoc.fromBytes(blob: blob);
    final secondJson = reloaded.getMap(name: 'root').json();

    final firstMap = jsonDecode(firstJson) as Map<String, dynamic>;
    final secondMap = jsonDecode(secondJson) as Map<String, dynamic>;

    expect(firstMap, equals(secondMap));
    expect(firstMap['title'], equals('Hello'));
    expect(firstMap['subtitle'], equals('via FRB'));
  });
}
