import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:scotoma/src/home/session_preparation_screen.dart';
import 'package:scotoma/src/session/numeric_credential_formatter.dart';
import 'package:scotoma/src/session/numeric_credential_rules.dart';
import 'package:scotoma/src/session/session_backend.dart';

class HomeScreen extends StatefulWidget {
  const HomeScreen({required this.backend, super.key});

  final SessionBackend backend;

  @override
  State<HomeScreen> createState() => _HomeScreenState();
}

class _HomeScreenState extends State<HomeScreen> with WidgetsBindingObserver {
  final _formKey = GlobalKey<FormState>();
  final _roomController = TextEditingController();
  final _passwordController = TextEditingController();
  bool _obscurePassword = true;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state != AppLifecycleState.resumed) {
      TextInput.finishAutofillContext(shouldSave: false);
      _clearCredentials();
      if (mounted && !_obscurePassword) {
        setState(() => _obscurePassword = true);
      }
    }
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    _clearCredentials();
    _roomController.dispose();
    _passwordController.dispose();
    super.dispose();
  }

  void _clearCredentials() {
    _roomController.clear();
    _passwordController.clear();
  }

  void _submit() {
    if (!(_formKey.currentState?.validate() ?? false)) {
      return;
    }

    final launch = widget.backend.prepare(
      _roomController.text,
      _passwordController.text,
    );
    FocusManager.instance.primaryFocus?.unfocus();
    TextInput.finishAutofillContext(shouldSave: false);
    _clearCredentials();
    setState(() => _obscurePassword = true);
    if (launch == null) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('No se pudo preparar la sesión.')),
      );
      return;
    }
    var routeOwnsLaunch = false;
    try {
      Navigator.of(context).push<void>(
        MaterialPageRoute<void>(
          builder: (_) => SessionPreparationScreen(launch: launch),
        ),
      );
      routeOwnsLaunch = true;
    } finally {
      if (!routeOwnsLaunch) launch.cancel();
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: SafeArea(
        child: LayoutBuilder(
          builder: (context, constraints) {
            return SingleChildScrollView(
              padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 32),
              child: ConstrainedBox(
                constraints: BoxConstraints(
                  minHeight: (constraints.maxHeight - 64).clamp(
                    0.0,
                    double.infinity,
                  ),
                ),
                child: Center(
                  child: SizedBox(
                    width: 460,
                    child: Form(
                      key: _formKey,
                      child: AutofillGroup(
                        onDisposeAction: AutofillContextAction.cancel,
                        child: Column(
                          mainAxisAlignment: MainAxisAlignment.center,
                          crossAxisAlignment: CrossAxisAlignment.stretch,
                          children: [
                            const _BrandHeader(),
                            const SizedBox(height: 40),
                            TextFormField(
                              key: const Key('room-field'),
                              controller: _roomController,
                              keyboardType: TextInputType.number,
                              textInputAction: TextInputAction.next,
                              maxLength: NumericCredentialRules.maxDigits,
                              maxLengthEnforcement: MaxLengthEnforcement.none,
                              inputFormatters: const [
                                NumericCredentialFormatter(),
                              ],
                              autofillHints: null,
                              validator: NumericCredentialRules.validateRoom,
                              autocorrect: false,
                              enableSuggestions: false,
                              enableIMEPersonalizedLearning: false,
                              decoration: const InputDecoration(
                                labelText: 'Room',
                                hintText: 'Número acordado',
                                prefixIcon: Icon(Icons.tag_rounded),
                              ),
                            ),
                            const SizedBox(height: 12),
                            TextFormField(
                              key: const Key('password-field'),
                              controller: _passwordController,
                              keyboardType: TextInputType.number,
                              textInputAction: TextInputAction.done,
                              maxLength: NumericCredentialRules.maxDigits,
                              maxLengthEnforcement: MaxLengthEnforcement.none,
                              obscureText: _obscurePassword,
                              inputFormatters: const [
                                NumericCredentialFormatter(),
                              ],
                              autofillHints: null,
                              validator:
                                  NumericCredentialRules.validatePassword,
                              onFieldSubmitted: (_) => _submit(),
                              autocorrect: false,
                              enableSuggestions: false,
                              enableIMEPersonalizedLearning: false,
                              decoration: InputDecoration(
                                labelText: 'Contraseña',
                                hintText: 'Número compartido',
                                prefixIcon: const Icon(
                                  Icons.lock_outline_rounded,
                                ),
                                suffixIcon: IconButton(
                                  key: const Key('toggle-password-visibility'),
                                  tooltip: _obscurePassword
                                      ? 'Mostrar contraseña'
                                      : 'Ocultar contraseña',
                                  onPressed: () {
                                    setState(() {
                                      _obscurePassword = !_obscurePassword;
                                    });
                                  },
                                  icon: Icon(
                                    _obscurePassword
                                        ? Icons.visibility_outlined
                                        : Icons.visibility_off_outlined,
                                  ),
                                ),
                              ),
                            ),
                            const SizedBox(height: 12),
                            FilledButton.icon(
                              key: const Key('continue-button'),
                              onPressed: _submit,
                              icon: const Icon(Icons.arrow_forward_rounded),
                              label: const Padding(
                                padding: EdgeInsets.symmetric(vertical: 15),
                                child: Text('Continuar'),
                              ),
                            ),
                            const SizedBox(height: 24),
                            const _PrivacyNotice(),
                          ],
                        ),
                      ),
                    ),
                  ),
                ),
              ),
            );
          },
        ),
      ),
    );
  }
}

class _BrandHeader extends StatelessWidget {
  const _BrandHeader();

  @override
  Widget build(BuildContext context) {
    final textTheme = Theme.of(context).textTheme;
    return Column(
      children: [
        Container(
          width: 72,
          height: 72,
          decoration: BoxDecoration(
            shape: BoxShape.circle,
            border: Border.all(
              color: Theme.of(context).colorScheme.primary,
              width: 2,
            ),
          ),
          child: const Icon(Icons.blur_circular_rounded, size: 42),
        ),
        const SizedBox(height: 20),
        Text(
          'SCOTOMA',
          style: textTheme.headlineMedium?.copyWith(
            fontWeight: FontWeight.w700,
            letterSpacing: 6,
          ),
        ),
        const SizedBox(height: 8),
        Text(
          'Conversaciones efímeras, sin cuentas.',
          textAlign: TextAlign.center,
          style: textTheme.bodyLarge?.copyWith(
            color: Theme.of(context).colorScheme.onSurfaceVariant,
          ),
        ),
      ],
    );
  }
}

class _PrivacyNotice extends StatelessWidget {
  const _PrivacyNotice();

  @override
  Widget build(BuildContext context) {
    return Semantics(
      label: 'Aviso de privacidad',
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Icon(
            Icons.memory_rounded,
            size: 20,
            color: Theme.of(context).colorScheme.primary,
          ),
          const SizedBox(width: 10),
          const Expanded(
            child: Text(
              'La app no guarda estas credenciales en disco. Se vacían los campos al '
              'continuar y tendrás que escribirlas de nuevo en cada sesión.',
            ),
          ),
        ],
      ),
    );
  }
}
