import 'dart:async';
import 'dart:convert';

import 'src/rust/api/benchmark.dart';
import 'lifecycle_checks.dart';

/// End-to-end async call latency, NOT isolated CPU time or measured peak RAM.
/// Fixtures exist only in Rust. No credentials, tags or derived bytes returned.
Future<String> runKdfBenchmark() async {
  final warmup = await benchmarkKdf().timeout(const Duration(seconds: 30));
  if (!warmup.releaseBuild ||
      warmup.memoryKib != 8192 ||
      warmup.iterations != 2 ||
      warmup.lanes != 1) {
    throw StateError('benchmark_requires_expected_release_core');
  }
  final samples = <int>[];
  var ticks = 0;
  var lastTickUs = 0;
  var maxGapUs = 0;
  final wall = Stopwatch()..start();
  final heartbeat = Timer.periodic(const Duration(milliseconds: 10), (_) {
    final now = wall.elapsedMicroseconds;
    final gap = now - lastTickUs;
    if (gap > maxGapUs) maxGapUs = gap;
    lastTickUs = now;
    ticks++;
  });
  try {
    for (var i = 0; i < 5; i++) {
      final call = Stopwatch()..start();
      await benchmarkKdf().timeout(const Duration(seconds: 30));
      samples.add(call.elapsedMicroseconds);
    }
  } finally {
    heartbeat.cancel();
    wall.stop();
  }
  final finalGap = wall.elapsedMicroseconds - lastTickUs;
  if (finalGap > maxGapUs) maxGapUs = finalGap;
  final sorted = [...samples]..sort();
  // Run outside the timed sample window, with the bridge still initialized.
  await verifyLifecycleCalls();
  return jsonEncode({
    'lifecycle_checks': true,
    'algorithm': 'Argon2id v1.3',
    'memory_kib_configured': warmup.memoryKib,
    'iterations': warmup.iterations,
    'lanes': warmup.lanes,
    'rust_release': warmup.releaseBuild,
    'warmups': 1,
    'samples_us': samples,
    'min_us': sorted.first,
    'median_us': sorted[2],
    'max_us': sorted.last,
    'event_loop_ticks_10ms': ticks,
    'max_event_loop_gap_us': maxGapUs,
  });
}
