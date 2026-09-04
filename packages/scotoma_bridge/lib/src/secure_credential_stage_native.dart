import 'dart:ffi';
import 'dart:io';
import 'dart:typed_data';

import 'package:ffi/ffi.dart';

const _capacity = 128;

typedef _NativeStage = Uint32 Function(
  Pointer<Uint8>,
  UintPtr,
  Pointer<Uint8>,
  UintPtr,
);
typedef _DartStage = int Function(Pointer<Uint8>, int, Pointer<Uint8>, int);

_DartStage? _stage;

void initialize({String? libraryPath}) {
  if (_stage != null) return;
  final library = libraryPath == null
      ? _openPackagedLibrary()
      : DynamicLibrary.open(libraryPath);
  _stage = library.lookupFunction<_NativeStage, _DartStage>(
    'scotoma_secure_credential_stage',
  );
}

int stage(Uint8List room, Uint8List password) {
  final call = _stage;
  if (call == null || !_bounded(room) || !_bounded(password)) return 0;
  final allocation = calloc<Uint8>(_capacity * 2);
  final roomPointer = allocation;
  final passwordPointer = allocation + _capacity;
  final fullAllocation = allocation.asTypedList(_capacity * 2);
  final roomAllocation = roomPointer.asTypedList(_capacity);
  final passwordAllocation = passwordPointer.asTypedList(_capacity);
  try {
    roomAllocation.setRange(0, room.length, room);
    passwordAllocation.setRange(0, password.length, password);
    return call(roomPointer, room.length, passwordPointer, password.length);
  } finally {
    fullAllocation.fillRange(0, fullAllocation.length, 0);
    calloc.free(allocation);
  }
}

bool _bounded(Uint8List value) => value.isNotEmpty && value.length <= _capacity;

DynamicLibrary _openPackagedLibrary() {
  if (Platform.isIOS) return DynamicLibrary.process();
  if (Platform.isAndroid || Platform.isLinux) {
    return DynamicLibrary.open('libscotoma_bridge.so');
  }
  if (Platform.isWindows) {
    return DynamicLibrary.open('scotoma_bridge.dll');
  }
  if (Platform.isMacOS) {
    return DynamicLibrary.open('libscotoma_bridge.dylib');
  }
  throw UnsupportedError('secure_credential_stage_unsupported');
}
