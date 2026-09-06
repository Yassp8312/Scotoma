import 'dart:io';

Never _fail(String message) => throw StateError(message);

String _read(String relativePath) {
  final file = File(relativePath);
  if (!file.existsSync()) _fail('Missing security input: $relativePath');
  return file.readAsStringSync();
}

void _require(String body, String value, String source) {
  if (!body.contains(value)) _fail('$source is missing required value: $value');
}

void main() {
  final manifest = _read('android/app/src/main/AndroidManifest.xml');
  for (final value in <String>[
    'android:allowBackup="false"',
    'android:fullBackupContent="false"',
    'android:dataExtractionRules="@xml/data_extraction_rules"',
    'android:usesCleartextTraffic="false"',
    'android:taskAffinity=""',
  ]) {
    _require(manifest, value, 'AndroidManifest.xml');
  }
  final permissions = RegExp(r'<uses-permission\s+android:name="([^"]+)"\s*/>')
      .allMatches(manifest)
      .map((match) => match.group(1))
      .toList();
  if (permissions.length != 1 ||
      permissions.single != 'android.permission.INTERNET') {
    _fail('Source manifest must request only android.permission.INTERNET');
  }

  final extractionRules = _read(
    'android/app/src/main/res/xml/data_extraction_rules.xml',
  );
  for (final section in <String>['cloud-backup', 'device-transfer']) {
    _require(extractionRules, '<$section>', 'data_extraction_rules.xml');
  }
  for (final domain in <String>[
    'root',
    'file',
    'database',
    'sharedpref',
    'external',
    'device_root',
    'device_file',
    'device_database',
    'device_sharedpref',
  ]) {
    final count = RegExp('domain="$domain"').allMatches(extractionRules).length;
    if (count != 2) {
      _fail('Backup domain $domain must be excluded from both transfer modes');
    }
  }

  final activity = _read(
    'android/app/src/main/kotlin/app/scotoma/scotoma/MainActivity.kt',
  );
  for (final value in <String>[
    'WindowManager.LayoutParams.FLAG_SECURE',
    'filterTouchesWhenObscured = true',
    'setRecentsScreenshotEnabled(false)',
  ]) {
    _require(activity, value, 'MainActivity.kt');
  }

  final dependencyFiles = <String>[
    'pubspec.yaml',
    '../../packages/scotoma_bridge/pubspec.yaml',
  ];
  final forbiddenDependencies = <String>[
    'shared_preferences',
    'sqflite',
    'hive_flutter',
    'firebase_analytics',
    'firebase_crashlytics',
    'sentry_flutter',
    'path_provider',
  ];
  for (final path in dependencyFiles) {
    final body = _read(path);
    for (final dependency in forbiddenDependencies) {
      if (body.contains(dependency)) {
        _fail(
          '$path contains forbidden persistence/telemetry dependency $dependency',
        );
      }
    }
  }

  final sourceRoots = <Directory>[
    Directory('lib'),
    Directory('../../packages/scotoma_bridge/lib'),
  ];
  final forbiddenSourceTokens = <String>[
    'debugPrint(',
    'Clipboard.setData(',
    "import 'dart:developer'",
    'FirebaseAnalytics',
    'FirebaseCrashlytics',
    'Sentry.capture',
  ];
  for (final root in sourceRoots) {
    for (final entity in root.listSync(recursive: true)) {
      if (entity is! File || !entity.path.endsWith('.dart')) continue;
      final body = entity.readAsStringSync();
      for (final token in forbiddenSourceTokens) {
        if (body.contains(token)) {
          _fail('${entity.path} contains forbidden source token $token');
        }
      }
    }
  }

  final webIndex = _read('web/index.html');
  for (final match in RegExp(
    r'<script[^>]+src="([^"]+)"',
  ).allMatches(webIndex)) {
    final source = match.group(1)!;
    if (source.contains('://') || source.startsWith('//')) {
      _fail('web/index.html loads a remote script: $source');
    }
  }

  stdout.writeln('SOURCE_SECURITY_BASELINE_OK');
}
