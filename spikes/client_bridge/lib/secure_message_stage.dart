import 'src/secure_message_stage_native.dart'
    if (dart.library.js_interop) 'src/secure_message_stage_web.dart'
    as platform;

void initializeSecureMessageStage({String? libraryPath}) =>
    platform.initialize(libraryPath: libraryPath);

/// Stages one UTF-8 message in bounded mutable memory and returns a one-use
/// token. The immutable Dart [String] cannot be deterministically erased.
int secureMessageStage(String message) => platform.stageText(message);
