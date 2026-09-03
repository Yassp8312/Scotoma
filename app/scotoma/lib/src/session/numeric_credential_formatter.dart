import 'package:flutter/services.dart';
import 'package:scotoma/src/session/numeric_credential_rules.dart';

/// Rejects an invalid edit in full; never strips or truncates a credential.
class NumericCredentialFormatter extends TextInputFormatter {
  const NumericCredentialFormatter();

  @override
  TextEditingValue formatEditUpdate(
    TextEditingValue oldValue,
    TextEditingValue newValue,
  ) {
    return NumericCredentialRules.isEditable(newValue.text)
        ? newValue
        : oldValue;
  }
}
