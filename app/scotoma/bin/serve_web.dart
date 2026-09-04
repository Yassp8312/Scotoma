import 'dart:io';

/// Read-only loopback server for the built web client with production headers.
Future<void> main(List<String> arguments) async {
  final omitIsolation = arguments.contains('--without-isolation');
  final port = omitIsolation ? 8794 : 8793;
  final directory = Directory.fromUri(Platform.script.resolve('../build/web/'));
  if (!directory.existsSync()) throw StateError('Build the web client first.');
  final root = await directory.resolveSymbolicLinks();
  final server = await HttpServer.bind(InternetAddress.loopbackIPv4, port);
  stdout.writeln('SCOTOMA web: http://127.0.0.1:$port');
  await for (final request in server) {
    await _serve(request, root, omitIsolation: omitIsolation);
  }
}

Future<void> _serve(
  HttpRequest request,
  String root, {
  required bool omitIsolation,
}) async {
  try {
    final response = request.response;
    response.headers.set('Cache-Control', 'no-store');
    response.headers.set('Pragma', 'no-cache');
    response.headers.set('X-Content-Type-Options', 'nosniff');
    response.headers.set('Referrer-Policy', 'no-referrer');
    response.headers.set(
      'Permissions-Policy',
      'camera=(), microphone=(), geolocation=()',
    );
    response.headers.set(
      'Content-Security-Policy',
      "default-src 'self'; base-uri 'self'; object-src 'none'; "
          "frame-ancestors 'none'; connect-src 'self' ws: wss:; "
          "img-src 'self' data: blob:; font-src 'self' data:; "
          "style-src 'self' 'unsafe-inline'; script-src 'self' "
          "'unsafe-inline' 'unsafe-eval' 'wasm-unsafe-eval'; "
          "worker-src 'self' blob:; "
          "manifest-src 'self'",
    );
    if (!omitIsolation) {
      response.headers.set('Cross-Origin-Opener-Policy', 'same-origin');
      response.headers.set('Cross-Origin-Embedder-Policy', 'require-corp');
      response.headers.set('Cross-Origin-Resource-Policy', 'same-origin');
    }

    final segments = request.uri.pathSegments
        .where((part) => part.isNotEmpty)
        .toList();
    if (!['GET', 'HEAD'].contains(request.method) ||
        segments.any(
          (part) =>
              part == '.' ||
              part == '..' ||
              part.contains('/') ||
              part.contains('\\') ||
              part.contains('\u0000'),
        )) {
      response.statusCode = HttpStatus.badRequest;
    } else {
      final relative = segments.isEmpty
          ? 'index.html'
          : segments.join(Platform.pathSeparator);
      final file = File('$root${Platform.pathSeparator}$relative');
      final insideRoot =
          file.existsSync() &&
          (await file.resolveSymbolicLinks()).startsWith(
            '$root${Platform.pathSeparator}',
          );
      if (!insideRoot) {
        response.statusCode = HttpStatus.notFound;
      } else {
        response.headers.contentType = _contentType(file.path);
        if (request.method == 'GET') await response.addStream(file.openRead());
      }
    }
    await response.close();
  } catch (_) {
    try {
      request.response.statusCode = HttpStatus.internalServerError;
      await request.response.close();
    } catch (_) {
      // Request values and exception details are never logged.
    }
  }
}

ContentType _contentType(String path) => switch (path.split('.').last) {
  'html' => ContentType.html,
  'js' => ContentType('text', 'javascript', charset: 'utf-8'),
  'json' => ContentType.json,
  'wasm' => ContentType('application', 'wasm'),
  'css' => ContentType('text', 'css', charset: 'utf-8'),
  'png' => ContentType('image', 'png'),
  'woff2' => ContentType('font', 'woff2'),
  'ttf' => ContentType('font', 'ttf'),
  _ => ContentType.binary,
};
