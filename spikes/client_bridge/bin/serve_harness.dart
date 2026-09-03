import 'dart:io';

/// Read-only, loopback-only server for the built harness. Never serves the repo.
Future<void> main(List<String> arguments) async {
  final omitIsolation = arguments.contains('--without-isolation');
  final port = omitIsolation ? 8792 : 8791;
  final directory = Directory.fromUri(
    Platform.script.resolve('../harness/build/web/'),
  );
  if (!directory.existsSync()) throw StateError('Build the web harness first.');
  final root = await directory.resolveSymbolicLinks();
  final server = await HttpServer.bind(InternetAddress.loopbackIPv4, port);
  stdout.writeln(
    'Harness: http://127.0.0.1:$port (isolation: ${!omitIsolation})',
  );
  await for (final request in server) {
    try {
      final response = request.response;
      response.headers.set('Cache-Control', 'no-store');
      response.headers.set('X-Content-Type-Options', 'nosniff');
      if (!omitIsolation) {
        response.headers.set('Cross-Origin-Opener-Policy', 'same-origin');
        response.headers.set('Cross-Origin-Embedder-Policy', 'require-corp');
        response.headers.set('Cross-Origin-Resource-Policy', 'same-origin');
      }
      final segments = request.uri.pathSegments
          .where((s) => s.isNotEmpty)
          .toList();
      if (segments.any(
            (s) =>
                s == '..' ||
                s == '.' ||
                s.contains('\\') ||
                s.contains('/') ||
                s.contains('\u0000'),
          ) ||
          !['GET', 'HEAD'].contains(request.method)) {
        response.statusCode = HttpStatus.badRequest;
      } else {
        final relative = segments.isEmpty
            ? 'index.html'
            : segments.join(Platform.pathSeparator);
        final file = File('$root${Platform.pathSeparator}$relative');
        if (!file.existsSync() ||
            !(await file.resolveSymbolicLinks()).startsWith(
              '$root${Platform.pathSeparator}',
            )) {
          response.statusCode = HttpStatus.notFound;
        } else {
          final extension = file.path.split('.').last;
          response.headers.set('Content-Type', switch (extension) {
            'html' => 'text/html; charset=utf-8',
            'js' => 'text/javascript; charset=utf-8',
            'json' => 'application/json',
            'wasm' => 'application/wasm',
            'css' => 'text/css; charset=utf-8',
            'png' => 'image/png',
            'woff2' => 'font/woff2',
            'ttf' => 'font/ttf',
            _ => 'application/octet-stream',
          });
          if (request.method == 'GET')
            await response.addStream(file.openRead());
        }
      }
      await response.close();
    } catch (_) {
      // No user data or request paths are logged by this local diagnostic server.
      try {
        request.response.statusCode = HttpStatus.internalServerError;
        await request.response.close();
      } catch (_) {
        // A disconnected client must not terminate the diagnostic server.
      }
    }
  }
}
