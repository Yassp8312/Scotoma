import 'dart:convert';
import 'dart:js_interop';

const maxMessageBytes = 16000;

@JS('wasm_bindgen.scotomaSecureMessageStage')
external int _secureStage(JSUint8Array message);

void initialize({String? libraryPath}) {}

int stageText(String message) {
  if (message.isEmpty) return 0;
  final bytes = utf8.encode(message);
  if (bytes.isEmpty || bytes.length > maxMessageBytes) {
    bytes.fillRange(0, bytes.length, 0);
    return 0;
  }
  final jsBytes = bytes.toJS;
  try {
    return _secureStage(jsBytes);
  } finally {
    bytes.fillRange(0, bytes.length, 0);
    jsBytes.toDart.fillRange(0, jsBytes.toDart.length, 0);
  }
}
