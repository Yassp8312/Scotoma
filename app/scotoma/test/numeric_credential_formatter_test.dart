import 'package:flutter_test/flutter_test.dart';
import 'package:scotoma/src/session/numeric_credential_formatter.dart';

void main() {
  const formatter = NumericCredentialFormatter();
  const oldValue = TextEditingValue(text: '00013');

  test('preserves valid edits including leading zeroes and deletion', () {
    for (final text in ['', '000013', '7' * 39, '9' * 128]) {
      final next = TextEditingValue(text: text);
      expect(formatter.formatEditUpdate(oldValue, next), same(next));
    }
  });

  test('rejects a whole invalid paste instead of changing its meaning', () {
    for (final text in ['12a3', '123\n', '1 23', '１２３', '9' * 129]) {
      expect(
        formatter.formatEditUpdate(oldValue, TextEditingValue(text: text)),
        same(oldValue),
      );
    }
  });
}
