import 'package:flutter_test/flutter_test.dart';
import 'package:scotoma/src/session/numeric_credential_rules.dart';

void main() {
  group('NumericCredentialRules', () {
    test('accepts one digit, leading zeroes, and 128 digits', () {
      expect(NumericCredentialRules.validateRoom('0'), isNull);
      expect(NumericCredentialRules.validateRoom('000013'), isNull);
      expect(NumericCredentialRules.validateRoom('1' * 39), isNull);
      expect(NumericCredentialRules.validatePassword('7' * 128), isNull);
    });

    test('rejects empty, nonnumeric, and oversized values', () {
      expect(NumericCredentialRules.validateRoom(''), isNotNull);
      expect(NumericCredentialRules.validateRoom('12 34'), isNotNull);
      expect(NumericCredentialRules.validatePassword('12a34'), isNotNull);
      for (final value in ['123\n', '123\r\n', '١٢٣', '１２３', '+123', '1.2']) {
        expect(NumericCredentialRules.validateRoom(value), isNotNull);
        expect(NumericCredentialRules.validatePassword(value), isNotNull);
      }
      expect(NumericCredentialRules.validatePassword('7' * 129), isNotNull);
    });
  });
}
