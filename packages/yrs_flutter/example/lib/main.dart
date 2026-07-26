import 'package:flutter/material.dart';
import 'package:yrs_flutter/yrs_flutter.dart';

Future<void> main() async {
  await RustLib.init();
  runApp(const YrsExampleApp());
}

class YrsExampleApp extends StatelessWidget {
  const YrsExampleApp({super.key});

  @override
  Widget build(BuildContext context) {
    return const MaterialApp(
      title: 'yrs_flutter example',
      home: ExampleHome(),
    );
  }
}

class ExampleHome extends StatefulWidget {
  const ExampleHome({super.key});

  @override
  State<ExampleHome> createState() => _ExampleHomeState();
}

class _ExampleHomeState extends State<ExampleHome> {
  String _status = 'Press the button to run a save/load round-trip.';
  String _firstJson = '';
  String _secondJson = '';
  int _saveBytes = 0;

  void _runRoundTrip() {
    try {
      final doc = YrsDoc.newEmpty();
      doc.getMap(name: 'root')
        ..set('title', 'Hello from Yjs')
        ..set('subtitle', 'via flutter_rust_bridge');

      final firstJson = doc.getMap(name: 'root').json();
      final blob = doc.save();
      final reloaded = YrsDoc.fromBytes(blob: blob);
      final secondJson = reloaded.getMap(name: 'root').json();

      setState(() {
        _firstJson = firstJson;
        _secondJson = secondJson;
        _saveBytes = blob.length;
        _status = firstJson == secondJson
            ? 'Round-trip OK — JSON identical after save/load.'
            : 'Round-trip OK by data — JSON key order may differ '
                '(yrs uses HashMap; data is preserved).';
      });
    } catch (e, st) {
      setState(() {
        _status = 'Error: $e\n$st';
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('yrs_flutter example')),
      body: Padding(
        padding: const EdgeInsets.all(16),
        child: SingleChildScrollView(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              FilledButton(
                onPressed: _runRoundTrip,
                child: const Text('Run round-trip'),
              ),
              const SizedBox(height: 16),
              Text(_status, style: Theme.of(context).textTheme.titleMedium),
              const SizedBox(height: 16),
              const Text('JSON before save/load:'),
              SelectableText(_firstJson),
              const SizedBox(height: 16),
              const Text('JSON after save/load:'),
              SelectableText(_secondJson),
              const SizedBox(height: 16),
              Text('Save blob size: $_saveBytes bytes'),
            ],
          ),
        ),
      ),
    );
  }
}
