import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:scotoma_bridge_probe/numeric_transfer.dart';
import 'package:scotoma_bridge_probe/credential_session_port.dart';

void main() {
  for (final length in [1, 39, 128]) {
    test('accepts $length digits and overwrites submitted live buffers', () {
      final input = NumericTransfer.fromText('1' * length, '2' * length)!;
      late Uint8List room, password;
      expect(
        input.submit((r, p) {
          room = r;
          password = p;
          expect(input.consumed, true);
          expect(r, List.filled(length, 49));
          expect(p, List.filled(length, 50));
          return StageOutcome.staged;
        }),
        StageOutcome.staged,
      );
      expect(room, List.filled(length, 0));
      expect(password, List.filled(length, 0));
      expect(
        input.submit((_, _) => throw StateError('must_not_repeat')),
        StageOutcome.failed,
      );
    });
  }

  test('validates both fields before handoff, including hostile paste', () {
    for (final bad in [
      '',
      '0' * 129,
      '1\n',
      ' 1',
      '+1',
      '1.0',
      '1e3',
      '١٢',
      '１２',
      '😀',
      '\u0000',
    ]) {
      expect(NumericTransfer.fromText(bad, '1370'), isNull);
      expect(NumericTransfer.fromText('13', bad), isNull);
    }
  });

  test('does not canonicalize or round numbers in Dart', () {
    final input = NumericTransfer.fromText('00013', '001370')!;
    expect(
      input.submit((r, p) {
        expect(r, [48, 48, 48, 49, 51]);
        expect(p, [48, 48, 49, 51, 55, 48]);
        return StageOutcome.staged;
      }),
      StageOutcome.staged,
    );
    expect(input.toString(), 'NumericTransfer([REDACTED])');
  });

  for (final throws in [false, true]) {
    test('wipes both lists on ${throws ? 'exception' : 'rejection'}', () {
      final input = NumericTransfer.fromText('13', '1370')!;
      late Uint8List room, password;
      expect(
        input.submit((r, p) {
          room = r;
          password = p;
          if (throws) throw StateError('SCOTOMA_PRIVATE_ERROR_CANARY');
          return StageOutcome.failed;
        }),
        StageOutcome.failed,
      );
      expect(room, [0, 0]);
      expect(password, [0, 0, 0, 0]);
      expect(input.consumed, true);
    });
  }

  test('abandoned or reentrant input is never handed off twice', () {
    final abandoned = NumericTransfer.fromText('13', '1370')!;
    abandoned.dispose();
    abandoned.dispose();
    expect(abandoned.consumed, true);
    expect(
      abandoned.submit((_, _) => throw StateError('no call')),
      StageOutcome.failed,
    );
    final input = NumericTransfer.fromText('13', '1370')!;
    expect(
      input.submit((_, _) {
        expect(
          input.submit((_, _) => throw StateError('no call')),
          StageOutcome.failed,
        );
        input.dispose();
        return StageOutcome.staged;
      }),
      StageOutcome.staged,
    );
  });

  test(
    'bridge creation failure still consumes input and uses a fixed error',
    () {
      // No bridge initialized in this unit-test process; this is not real FFI.
      final input = NumericTransfer.fromText('13', '1370')!;
      expect(
        () => CredentialSessionPort.open(input),
        throwsA(
          isA<StateError>().having(
            (e) => e.message,
            'message',
            'credential_stage_failed',
          ),
        ),
      );
      expect(input.consumed, true);
      expect(CredentialSessionPort.runtimeAvailable, false);
      expect(CredentialSessionPort.hasQuarantinedHandle, false);
      final retry = NumericTransfer.fromText('13', '1370')!;
      expect(() => CredentialSessionPort.open(retry), throwsStateError);
      expect(retry.consumed, true);
    },
  );
}
