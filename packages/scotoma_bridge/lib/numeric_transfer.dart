import 'dart:typed_data';

enum StageOutcome { staged, failed }

/// One-use mutable handoff for bounded numeric session credentials.
/// Does not own or erase the original immutable Dart strings / bridge copies.
final class NumericTransfer {
  NumericTransfer._(this._room, this._password);

  static const maxDigits = 128;
  Uint8List? _room, _password;
  bool get consumed => _room == null;

  static NumericTransfer? fromText(String room, String password) {
    // Check BOTH before allocating. No trim, integer conversion, normalization,
    // UTF-8 encoder, growable list, password generation or logging.
    if (!_valid(room) || !_valid(password)) return null;
    return NumericTransfer._(_copy(room), _copy(password));
  }

  static bool _valid(String value) {
    if (value.isEmpty || value.length > maxDigits) return false;
    for (var i = 0; i < value.length; i++) {
      final code = value.codeUnitAt(i);
      if (code < 48 || code > 57) return false;
    }
    return true;
  }

  static Uint8List _copy(String value) {
    final bytes = Uint8List(value.length);
    for (var i = 0; i < value.length; i++) {
      bytes[i] = value.codeUnitAt(i);
    }
    return bytes;
  }

  /// Synchronous by contract. Callback must not retain/forward buffers or log
  /// them. References are detached before the call; finally overwrites both
  /// live lists even on failure. This is not physical-erasure certification.
  StageOutcome submit(
    StageOutcome Function(Uint8List room, Uint8List password) stage,
  ) {
    final room = _room, password = _password;
    if (room == null || password == null) return StageOutcome.failed;
    _room = null;
    _password = null;
    try {
      return stage(room, password);
    } catch (_) {
      return StageOutcome.failed;
    } finally {
      room.fillRange(0, room.length, 0);
      password.fillRange(0, password.length, 0);
    }
  }

  /// Caller must dispose an input abandoned before submit (cancel/background).
  void dispose() {
    final room = _room, password = _password;
    _room = null;
    _password = null;
    room?.fillRange(0, room.length, 0);
    password?.fillRange(0, password.length, 0);
  }

  @override
  String toString() => 'NumericTransfer([REDACTED])';
}
