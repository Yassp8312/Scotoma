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
  const expectedPermissions = <String>{
    'android.permission.INTERNET',
    'android.permission.HIDE_OVERLAY_WINDOWS',
  };
  if (permissions.toSet().length != expectedPermissions.length ||
      !permissions.toSet().containsAll(expectedPermissions)) {
    _fail('Source manifest permission set changed: ${permissions.join(', ')}');
  }
  for (final forbidden in <String>[
    'android.intent.category.BROWSABLE',
    'android.intent.action.PROCESS_TEXT',
    '<queries>',
    '<service',
    '<receiver',
    '<provider',
  ]) {
    if (manifest.contains(forbidden)) {
      _fail('AndroidManifest.xml contains forbidden surface: $forbidden');
    }
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
    'View.IMPORTANT_FOR_AUTOFILL_NO_EXCLUDE_DESCENDANTS',
    'View.IMPORTANT_FOR_CONTENT_CAPTURE_NO_EXCLUDE_DESCENDANTS',
    'window.setHideOverlayWindows(true)',
    'setRecentsScreenshotEnabled(false)',
  ]) {
    _require(activity, value, 'MainActivity.kt');
  }
  final firstSecureFlag = activity.indexOf(
    'window.addFlags(WindowManager.LayoutParams.FLAG_SECURE)',
  );
  final superOnCreate = activity.indexOf('super.onCreate(savedInstanceState)');
  if (firstSecureFlag < 0 ||
      superOnCreate < 0 ||
      firstSecureFlag > superOnCreate) {
    _fail('FLAG_SECURE must be applied before Flutter draws its first frame');
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
    'print(',
    'Clipboard.',
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

  final webSourceRoot = Directory('web');
  for (final entity in webSourceRoot.listSync(recursive: true)) {
    if (entity is! File || !entity.path.endsWith('.js')) continue;
    final body = entity.readAsStringSync();
    for (final token in <String>[
      'console.log(',
      'console.debug(',
      'console.info(',
      'console.warn(',
      'console.error(',
    ]) {
      if (body.contains(token)) {
        _fail('${entity.path} contains forbidden web logging token $token');
      }
    }
  }

  final androidSourceRoot = Directory('android/app/src/main');
  for (final entity in androidSourceRoot.listSync(recursive: true)) {
    if (entity is! File ||
        (!entity.path.endsWith('.kt') && !entity.path.endsWith('.java'))) {
      continue;
    }
    final body = entity.readAsStringSync();
    for (final token in <String>[
      'android.util.Log',
      'ClipboardManager',
      'System.out.',
      'WebView.setWebContentsDebuggingEnabled(true)',
    ]) {
      if (body.contains(token)) {
        _fail('${entity.path} contains forbidden native token $token');
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

  for (final path in <String>['build_client.ps1', 'bin/serve_web.dart']) {
    final body = _read(path);
    if (body.contains("'unsafe-eval'")) {
      _fail('$path weakens script-src with unsafe-eval');
    }
  }

  final rustWebUtils = _read(
    '../../vendor/flutter_rust_bridge/src/misc/web_utils.rs',
  );
  if (rustWebUtils.contains('js_sys::eval')) {
    _fail('Vendored Rust bridge reintroduced js_sys::eval');
  }
  _require(rustWebUtils, '__SCOTOMA_FRB_SCRIPT_URL__', 'vendored Rust bridge');

  final dartWasmLoader = _read(
    '../../vendor/flutter_rust_bridge_dart/lib/src/wasm_module/_web.dart',
  );
  for (final forbidden in <String>["@JS('Function')", 'jsEval(']) {
    if (dartWasmLoader.contains(forbidden)) {
      _fail('Vendored Dart bridge reintroduced dynamic code: $forbidden');
    }
  }
  _require(dartWasmLoader, "@JS('wasm_bindgen')", 'vendored Dart bridge');

  final bridgeBuild = _read('../../packages/scotoma_bridge/verify.ps1');
  _require(
    bridgeBuild,
    'globalThis.wasm_bindgen =',
    'bridge build wrapper transform',
  );
  _require(
    _read('web/runtime_supervisor.js'),
    '__SCOTOMA_FRB_SCRIPT_URL__',
    'web runtime supervisor',
  );

  stdout.writeln('SOURCE_SECURITY_BASELINE_OK');
}
