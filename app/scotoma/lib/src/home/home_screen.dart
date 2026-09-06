import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:scotoma/src/home/session_preparation_screen.dart';
import 'package:scotoma/src/session/numeric_credential_formatter.dart';
import 'package:scotoma/src/session/numeric_credential_rules.dart';
import 'package:scotoma/src/session/session_backend.dart';
import 'package:scotoma/src/theme/scotoma_theme.dart';
import 'package:scotoma/src/widgets/scotoma_background.dart';
import 'package:scotoma/src/widgets/scotoma_mark.dart';

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
      body: ScotomaBackground(
        child: SafeArea(
          child: LayoutBuilder(
            builder: (context, constraints) {
              final wide = constraints.maxWidth >= 740;
              final horizontal = constraints.maxWidth < 520 ? 20.0 : 40.0;
              return SingleChildScrollView(
                padding: EdgeInsets.symmetric(
                  horizontal: horizontal,
                  vertical: 32,
                ),
                child: ConstrainedBox(
                  constraints: BoxConstraints(
                    minHeight: (constraints.maxHeight - 64).clamp(
                      0.0,
                      double.infinity,
                    ),
                  ),
                  child: Center(
                    child: ConstrainedBox(
                      constraints: const BoxConstraints(maxWidth: 1120),
                      child: wide
                          ? Row(
                              children: [
                                const Expanded(child: _BrandHeader(wide: true)),
                                const SizedBox(width: 32),
                                SizedBox(width: 420, child: _entryCard()),
                              ],
                            )
                          : ConstrainedBox(
                              constraints: const BoxConstraints(maxWidth: 480),
                              child: Column(
                                mainAxisAlignment: MainAxisAlignment.center,
                                children: [
                                  const _BrandHeader(),
                                  const SizedBox(height: 32),
                                  _entryCard(),
                                ],
                              ),
                            ),
                    ),
                  ),
                ),
              );
            },
          ),
        ),
      ),
    );
  }

  Widget _entryCard() {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Form(
          key: _formKey,
          child: AutofillGroup(
            onDisposeAction: AutofillContextAction.cancel,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                const _AlphaNotice(),
                const SizedBox(height: 20),
                Text(
                  'Entrar a un canal',
                  style: Theme.of(context).textTheme.headlineSmall,
                ),
                const SizedBox(height: 8),
                Text(
                  'Ambas personas deben escribir exactamente los mismos dos números.',
                  style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                    color: Theme.of(context).colorScheme.onSurfaceVariant,
                  ),
                ),
                const SizedBox(height: 24),
                TextFormField(
                  key: const Key('room-field'),
                  controller: _roomController,
                  keyboardType: TextInputType.number,
                  textInputAction: TextInputAction.next,
                  maxLength: NumericCredentialRules.maxDigits,
                  maxLengthEnforcement: MaxLengthEnforcement.none,
                  inputFormatters: const [NumericCredentialFormatter()],
                  autofillHints: null,
                  validator: NumericCredentialRules.validateRoom,
                  autocorrect: false,
                  enableSuggestions: false,
                  enableIMEPersonalizedLearning: false,
                  decoration: const InputDecoration(
                    labelText: 'Room',
                    hintText: 'Número acordado',
                    helperText: 'De 1 a 128 dígitos',
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
                  inputFormatters: const [NumericCredentialFormatter()],
                  autofillHints: null,
                  validator: NumericCredentialRules.validatePassword,
                  onFieldSubmitted: (_) => _submit(),
                  autocorrect: false,
                  enableSuggestions: false,
                  enableIMEPersonalizedLearning: false,
                  decoration: InputDecoration(
                    labelText: 'Contraseña',
                    hintText: 'Número compartido',
                    helperText: 'No se guarda en este dispositivo',
                    prefixIcon: const Icon(Icons.lock_outline_rounded),
                    suffixIcon: IconButton(
                      key: const Key('toggle-password-visibility'),
                      tooltip: _obscurePassword
                          ? 'Mostrar contraseña'
                          : 'Ocultar contraseña',
                      onPressed: () =>
                          setState(() => _obscurePassword = !_obscurePassword),
                      icon: Icon(
                        _obscurePassword
                            ? Icons.visibility_outlined
                            : Icons.visibility_off_outlined,
                      ),
                    ),
                  ),
                ),
                const SizedBox(height: 12),
                const _StrengthNotice(),
                const SizedBox(height: 20),
                FilledButton.icon(
                  key: const Key('continue-button'),
                  onPressed: _submit,
                  icon: const Icon(Icons.arrow_forward_rounded),
                  label: const Text('Continuar'),
                ),
                const SizedBox(height: 20),
                const _PrivacyNotice(),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _BrandHeader extends StatelessWidget {
  const _BrandHeader({this.wide = false});

  final bool wide;

  @override
  Widget build(BuildContext context) {
    final textTheme = Theme.of(context).textTheme;
    return Column(
      crossAxisAlignment: wide
          ? CrossAxisAlignment.start
          : CrossAxisAlignment.center,
      children: [
        ScotomaMark(size: wide ? 150 : 96),
        SizedBox(height: wide ? 32 : 20),
        Text(
          'SCOTOMA',
          style: textTheme.headlineMedium?.copyWith(
            fontWeight: FontWeight.w700,
            letterSpacing: 7,
          ),
        ),
        const SizedBox(height: 12),
        Text(
          wide
              ? 'Una conversación.\nSin identidad permanente.'
              : 'Conversaciones efímeras, sin cuentas.',
          textAlign: wide ? TextAlign.left : TextAlign.center,
          style: (wide ? textTheme.displaySmall : textTheme.bodyLarge)
              ?.copyWith(color: Theme.of(context).colorScheme.onSurfaceVariant),
        ),
        if (wide) ...[
          const SizedBox(height: 32),
          const Wrap(
            spacing: 10,
            runSpacing: 10,
            children: [
              _FeatureChip(
                icon: Icons.person_off_outlined,
                label: 'Sin cuenta',
              ),
              _FeatureChip(
                icon: Icons.history_toggle_off_rounded,
                label: 'Sin historial',
              ),
              _FeatureChip(
                icon: Icons.lock_outline_rounded,
                label: 'Cifrado E2E',
              ),
            ],
          ),
        ],
      ],
    );
  }
}

class _FeatureChip extends StatelessWidget {
  const _FeatureChip({required this.icon, required this.label});

  final IconData icon;
  final String label;

  @override
  Widget build(BuildContext context) => Container(
    padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
    decoration: BoxDecoration(
      color: ScotomaColors.surface.withValues(alpha: 0.78),
      borderRadius: BorderRadius.circular(999),
      border: Border.all(color: ScotomaColors.outline),
    ),
    child: Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Icon(icon, size: 17, color: ScotomaColors.violetLight),
        const SizedBox(width: 7),
        Text(label),
      ],
    ),
  );
}

class _AlphaNotice extends StatelessWidget {
  const _AlphaNotice();

  @override
  Widget build(BuildContext context) => Semantics(
    label: 'Alpha experimental',
    child: Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: ScotomaColors.danger.withValues(alpha: 0.12),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: ScotomaColors.danger.withValues(alpha: 0.55)),
      ),
      child: const Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Icon(Icons.science_outlined, size: 20, color: ScotomaColors.danger),
          SizedBox(width: 10),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  'ALPHA EXPERIMENTAL',
                  style: TextStyle(
                    color: ScotomaColors.danger,
                    fontWeight: FontWeight.w700,
                    letterSpacing: 0.8,
                  ),
                ),
                SizedBox(height: 4),
                Text(
                  'Solo para pruebas. No uses información sensible o de alto riesgo.',
                ),
              ],
            ),
          ),
        ],
      ),
    ),
  );
}

class _StrengthNotice extends StatelessWidget {
  const _StrengthNotice();

  @override
  Widget build(BuildContext context) => Semantics(
    label: 'Advertencia de seguridad',
    child: Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: ScotomaColors.violetDeep.withValues(alpha: 0.18),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: ScotomaColors.violet.withValues(alpha: 0.35)),
      ),
      child: const Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Icon(
            Icons.info_outline_rounded,
            size: 20,
            color: ScotomaColors.violetLight,
          ),
          SizedBox(width: 10),
          Expanded(
            child: Text(
              'La seguridad depende de que ambos números sean difíciles de adivinar. Los valores cortos son débiles.',
            ),
          ),
        ],
      ),
    ),
  );
}

class _PrivacyNotice extends StatelessWidget {
  const _PrivacyNotice();

  @override
  Widget build(BuildContext context) {
    return Semantics(
      label: 'Aviso de privacidad',
      child: const Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Icon(Icons.memory_rounded, size: 20, color: ScotomaColors.secure),
          SizedBox(width: 10),
          Expanded(
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
