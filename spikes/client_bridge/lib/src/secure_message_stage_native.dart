import 'dart:convert';
import 'dart:ffi';
import 'dart:io';

import 'package:ffi/ffi.dart';

const maxMessageBytes = 16000;

typedef _NativeStage = Uint32 Function(Pointer<Uint8>, UintPtr);
typedef _DartStage = int Function(Pointer<Uint8>, int);

_DartStage? _stage;

void initialize({String? libraryPath}) {
  if (_stage != null) return;
  final library = libraryPath == null
      ? _openPackagedLibrary()
      : DynamicLibrary.open(libraryPath);
  _stage = library.lookupFunction<_NativeStage, _DartStage>(
    'scotoma_secure_message_stage',
  );
}

int stageText(String message) {
  final call = _stage;
  if (call == null || message.isEmpty) return 0;
  final bytes = utf8.encode(message);
  if (bytes.isEmpty || bytes.length > maxMessageBytes) {
    bytes.fillRange(0, bytes.length, 0);
    return 0;
  }
  final allocation = calloc<Uint8>(bytes.length);
  final nativeBytes = allocation.asTypedList(bytes.length);
  try {
    nativeBytes.setAll(0, bytes);
    return call(allocation, bytes.length);
  } finally {
    bytes.fillRange(0, bytes.length, 0);
    nativeBytes.fillRange(0, nativeBytes.length, 0);
    calloc.free(allocation);
  }
}

DynamicLibrary _openPackagedLibrary() {
  if (Platform.isIOS) return DynamicLibrary.process();
  if (Platform.isAndroid || Platform.isLinux) {
    return DynamicLibrary.open('libscotoma_bridge_probe.so');
  }
  if (Platform.isWindows)
    return DynamicLibrary.open('scotoma_bridge_probe.dll');
  if (Platform.isMacOS) {
    return DynamicLibrary.open('libscotoma_bridge_probe.dylib');
  }
  throw UnsupportedError('secure_message_stage_unsupported');
}
