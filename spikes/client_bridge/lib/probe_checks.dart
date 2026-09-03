import 'src/rust/api/probe.dart';

/// Public metadata checks shared by the CLI and the actual Flutter harness.
Future<void> verifyBridgeCalls() async {
  validateProtocol(await protocolProbe().timeout(const Duration(seconds: 10)));
  final replies = await Future.wait(List.generate(16, (_) => protocolProbe()))
      .timeout(const Duration(seconds: 10));
  for (final reply in replies) {
    validateProtocol(reply);
  }
  if (!await randomnessProbe().timeout(const Duration(seconds: 10))) {
    throw StateError('The platform random generator is unavailable.');
  }
}

void validateProtocol(ProtocolProbe value) {
  if (value.protocolVersion != 2 ||
      value.minCredentialDigits != 1 ||
      value.maxCredentialDigits != 128 ||
      value.noiseProtocol != 'Noise_NNpsk0_25519_ChaChaPoly_SHA256') {
    throw StateError(
      'Rust protocol metadata did not match the expected contract.',
    );
  }
}
