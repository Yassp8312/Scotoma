final class NumericCredentialRules {
  const NumericCredentialRules._();

  static const int maxDigits = 128;

  static bool isEditable(String value) =>
      value.length <= maxDigits &&
      value.codeUnits.every((digit) => digit >= 0x30 && digit <= 0x39);

  static String? validateRoom(String? value) => _validate(value, 'room');

  static String? validatePassword(String? value) =>
      _validate(value, 'contraseña');

  static String? _validate(String? value, String fieldName) {
    if (value == null || value.isEmpty) {
      return 'Introduce la $fieldName.';
    }
    if (value.length > maxDigits) {
      return 'Usa como máximo $maxDigits dígitos.';
    }
    if (!isEditable(value)) {
      return 'Usa solamente números del 0 al 9.';
    }
    return null;
  }
}
