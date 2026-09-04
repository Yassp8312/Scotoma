enum DeriveOutcome { ready, closed, failed }

/// Minimal lifecycle contract consumed by the Flutter application.
abstract interface class SessionPort {
  Future<DeriveOutcome> derive();
  bool close();
  void dispose();
}
