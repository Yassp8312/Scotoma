import 'dart:js_interop';
import 'dart:typed_data';

const _capacity = 128;

@JS('wasm_bindgen.scotomaSecureCredentialStage')
external int _secureStage(JSUint8Array room, JSUint8Array password);

void initialize({String? libraryPath}) {}

int stage(Uint8List room, Uint8List password) {
  if (!_bounded(room) || !_bounded(password)) return 0;
  // toJS may cast, unwrap or clone. Explicitly wipe the JS views here; the
  // NumericTransfer owner separately wipes the original Dart lists.
  final jsRoom = room.toJS;
  final jsPassword = password.toJS;
  try {
    return _secureStage(jsRoom, jsPassword);
  } finally {
    jsRoom.toDart.fillRange(0, jsRoom.toDart.length, 0);
    jsPassword.toDart.fillRange(0, jsPassword.toDart.length, 0);
  }
}

bool _bounded(Uint8List value) => value.isNotEmpty && value.length <= _capacity;
