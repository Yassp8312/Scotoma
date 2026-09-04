import 'package:flutter/material.dart';
import 'package:scotoma/src/session/live_session_controller.dart';
import 'package:scotoma/src/session/session_backend.dart';
import 'package:scotoma/src/theme/scotoma_theme.dart';
import 'package:scotoma/src/widgets/scotoma_background.dart';
import 'package:scotoma/src/widgets/scotoma_mark.dart';

class SessionPreparationScreen extends StatefulWidget {
  const SessionPreparationScreen({required this.launch, super.key});

  final SessionLaunch launch;

  @override
  State<SessionPreparationScreen> createState() =>
      _SessionPreparationScreenState();
}

class _SessionPreparationScreenState extends State<SessionPreparationScreen>
    with WidgetsBindingObserver {
  late final LiveSessionController _controller;
  final _messageController = TextEditingController();
  bool _returnAfterBurn = false;
  bool _popScheduled = false;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    _controller = LiveSessionController(
      launch: widget.launch,
      onChanged: _onChanged,
    );
    _controller.start();
  }

  void _onChanged() {
    if (!mounted) return;
    if (_controller.isTerminal) _messageController.clear();
    setState(() {});
    if (_returnAfterBurn &&
        (_controller.phase == LiveSessionPhase.closed ||
            _controller.phase == LiveSessionPhase.failed)) {
      _schedulePop();
    }
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state != AppLifecycleState.resumed) _controller.close();
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    _controller.dispose();
    widget.launch.cancel();
    _messageController.clear();
    _messageController.dispose();
    super.dispose();
  }

  void _closeAndReturn() {
    if (_controller.phase == LiveSessionPhase.connected) {
      _returnAfterBurn = true;
      if (_controller.burn()) return;
    }
    _controller.close();
    _schedulePop();
  }

  void _schedulePop() {
    if (_popScheduled) return;
    _popScheduled = true;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted && Navigator.of(context).canPop()) {
        Navigator.of(context).pop();
      }
    });
  }

  void _sendMessage() {
    final text = _messageController.text;
    if (!_controller.sendText(text)) return;
    _messageController.clear();
  }

  @override
  Widget build(BuildContext context) {
    final presentation = _presentation(_controller.phase);
    return PopScope<void>(
      onPopInvokedWithResult: (didPop, _) {
        if (didPop) _controller.close();
      },
      child: Scaffold(
        appBar: AppBar(
          titleSpacing: 4,
          title: const Row(
            children: [
              ScotomaMark(size: 34, showGlow: false),
              SizedBox(width: 12),
              Text('SCOTOMA', style: TextStyle(letterSpacing: 3, fontSize: 16)),
            ],
          ),
        ),
        body: ScotomaBackground(
          child: SafeArea(
            top: false,
            child: _controller.phase == LiveSessionPhase.connected
                ? _ChatView(
                    messages: _controller.messages,
                    sendPending: _controller.sendPending,
                    verificationCode: _controller.verificationCode!,
                    messageController: _messageController,
                    onSend: _sendMessage,
                    onBurn: _closeAndReturn,
                  )
                : _StatusView(
                    presentation: presentation,
                    burning: _controller.phase == LiveSessionPhase.burning,
                    onCancel: _closeAndReturn,
                  ),
          ),
        ),
      ),
    );
  }
}

class _StatusView extends StatelessWidget {
  const _StatusView({
    required this.presentation,
    required this.burning,
    required this.onCancel,
  });

  final _Presentation presentation;
  final bool burning;
  final VoidCallback onCancel;

  @override
  Widget build(BuildContext context) => Center(
    child: SingleChildScrollView(
      padding: const EdgeInsets.all(24),
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 520),
        child: Card(
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 28, vertical: 36),
            child: Semantics(
              liveRegion: true,
              label: '${presentation.title}. ${presentation.message}',
              child: Column(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  if (presentation.busy)
                    const SizedBox.square(
                      dimension: 56,
                      child: CircularProgressIndicator(
                        key: Key('session-progress'),
                        strokeWidth: 3,
                      ),
                    )
                  else
                    Container(
                      width: 72,
                      height: 72,
                      decoration: const BoxDecoration(
                        color: ScotomaColors.surfaceHigh,
                        shape: BoxShape.circle,
                      ),
                      child: Icon(
                        presentation.icon,
                        key: const Key('session-status-icon'),
                        size: 34,
                        color: ScotomaColors.violetLight,
                      ),
                    ),
                  const SizedBox(height: 28),
                  Text(
                    presentation.title,
                    key: const Key('session-status-title'),
                    style: Theme.of(context).textTheme.headlineSmall,
                    textAlign: TextAlign.center,
                  ),
                  const SizedBox(height: 12),
                  Text(
                    presentation.message,
                    key: const Key('session-status-message'),
                    style: TextStyle(
                      color: Theme.of(context).colorScheme.onSurfaceVariant,
                    ),
                    textAlign: TextAlign.center,
                  ),
                  const SizedBox(height: 28),
                  OutlinedButton.icon(
                    key: const Key('cancel-session-button'),
                    onPressed: onCancel,
                    icon: const Icon(Icons.close_rounded),
                    label: Text(burning ? 'Destruyendo…' : 'Cancelar y volver'),
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    ),
  );
}

_Presentation _presentation(LiveSessionPhase phase) => switch (phase) {
  LiveSessionPhase.idle || LiveSessionPhase.starting => const _Presentation(
    title: 'Entregando credenciales',
    message: 'Los campos ya están vacíos. La copia mutable se está transfiriendo a Rust.',
    icon: Icons.memory_rounded,
    busy: true,
  ),
  LiveSessionPhase.deriving => const _Presentation(
    title: 'Derivando clave local',
    message: 'El cálculo ocurre fuera de la interfaz y la contraseña se destruye al terminar.',
    icon: Icons.memory_rounded,
    busy: true,
  ),
  LiveSessionPhase.connecting => const _Presentation(
    title: 'Conectando con el relay',
    message: 'Solo se envía una etiqueta derivada; la room, la contraseña y la clave no salen del dispositivo.',
    icon: Icons.cloud_sync_outlined,
    busy: true,
  ),
  LiveSessionPhase.waitingForPeer => const _Presentation(
    title: 'Esperando al segundo participante',
    message: 'La sesión permanece efímera. La otra persona debe introducir exactamente la misma room y contraseña.',
    icon: Icons.hourglass_top_rounded,
    busy: true,
  ),
  LiveSessionPhase.handshaking => const _Presentation(
    title: 'Verificando el canal',
    message: 'Ambos dispositivos están confirmando mutuamente las claves mediante Noise.',
    icon: Icons.handshake_outlined,
    busy: true,
  ),
  LiveSessionPhase.connected => const _Presentation(
    title: 'Canal seguro conectado',
    message: 'El handshake Noise terminó. Los mensajes viajan cifrados y requieren confirmación autenticada.',
    icon: Icons.verified_user_outlined,
    busy: false,
  ),
  LiveSessionPhase.burning => const _Presentation(
    title: 'Destruyendo la sesión',
    message:
        'Se está enviando un BURN autenticado y eliminando las claves locales.',
    icon: Icons.local_fire_department_outlined,
    busy: true,
  ),
  LiveSessionPhase.peerBurned => const _Presentation(
    title: 'La otra persona destruyó la sesión',
    message: 'El BURN fue autenticado. Las claves locales y el socket quedaron cerrados.',
    icon: Icons.local_fire_department_outlined,
    busy: false,
  ),
  LiveSessionPhase.connectionLost => const _Presentation(
    title: 'Conexión interrumpida',
    message: 'El socket se cerró y la sesión no se reintentará. Introduce nuevamente ambos números para comenzar otra.',
    icon: Icons.cloud_off_outlined,
    busy: false,
  ),
  LiveSessionPhase.deliveryTimedOut => const _Presentation(
    title: 'Entrega no confirmada',
    message: 'No llegó un ACK autenticado dentro del límite. El mensaje no se reenvió automáticamente y la sesión fue cerrada.',
    icon: Icons.mark_chat_unread_outlined,
    busy: false,
  ),
  LiveSessionPhase.closed || LiveSessionPhase.disposed => const _Presentation(
    title: 'Sesión cerrada',
    message: 'El socket y el material local fueron cerrados. Introduce ambos números para comenzar otra.',
    icon: Icons.cancel_outlined,
    busy: false,
  ),
  LiveSessionPhase.timedOut => const _Presentation(
    title: 'Tiempo agotado',
    message: 'La conexión se cerró sin reintentar. No se abrió ninguna conversación.',
    icon: Icons.timer_off_outlined,
    busy: false,
  ),
  LiveSessionPhase.failed => const _Presentation(
    title: 'No se pudo abrir la sesión',
    message: 'La operación terminó sin abrir una conversación. No se realizará un reintento automático.',
    icon: Icons.error_outline_rounded,
    busy: false,
  ),
};

final class _ChatView extends StatelessWidget {
  const _ChatView({
    required this.messages,
    required this.sendPending,
    required this.verificationCode,
    required this.messageController,
    required this.onSend,
    required this.onBurn,
  });

  final List<ChatMessage> messages;
  final bool sendPending;
  final String verificationCode;
  final TextEditingController messageController;
  final VoidCallback onSend;
  final VoidCallback onBurn;

  @override
  Widget build(BuildContext context) {
    return Center(
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 980),
        child: Column(
          children: [
            _ChannelHeader(verificationCode: verificationCode, onBurn: onBurn),
            Expanded(
              child: messages.isEmpty
                  ? const _EmptyChat()
                  : ListView.builder(
                      key: const Key('message-list'),
                      padding: const EdgeInsets.fromLTRB(20, 20, 20, 8),
                      itemCount: messages.length,
                      itemBuilder: (context, index) => _MessageBubble(
                        message: messages[index],
                        index: index,
                      ),
                    ),
            ),
            _MessageComposer(
              sendPending: sendPending,
              messageController: messageController,
              onSend: onSend,
            ),
          ],
        ),
      ),
    );
  }
}

class _ChannelHeader extends StatelessWidget {
  const _ChannelHeader({required this.verificationCode, required this.onBurn});

  final String verificationCode;
  final VoidCallback onBurn;

  @override
  Widget build(BuildContext context) => Container(
    margin: const EdgeInsets.fromLTRB(16, 8, 16, 0),
    padding: const EdgeInsets.all(16),
    decoration: BoxDecoration(
      color: ScotomaColors.surface.withValues(alpha: 0.96),
      borderRadius: BorderRadius.circular(18),
      border: Border.all(color: ScotomaColors.outline),
    ),
    child: LayoutBuilder(
      builder: (context, constraints) {
        final compact = constraints.maxWidth < 620;
        final details = Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const _ConnectionStatus(),
            const SizedBox(height: 12),
            Text(
              'CÓDIGO DE VERIFICACIÓN',
              style: Theme.of(context).textTheme.labelSmall
                  ?.copyWith(color: ScotomaColors.muted, letterSpacing: 1.4),
            ),
            const SizedBox(height: 4),
            Semantics(
              label: 'Código de verificación $verificationCode',
              child: SelectableText(
                verificationCode,
                key: const Key('verification-code'),
                style: Theme.of(context).textTheme.titleLarge?.copyWith(
                  color: ScotomaColors.violetLight,
                  fontFeatures: const [FontFeature.tabularFigures()],
                  letterSpacing: 2.2,
                ),
              ),
            ),
            const SizedBox(height: 6),
            Text(
              'Compárenlo por otro medio. Si no coincide, destruyan la sesión.',
              style: Theme.of(context).textTheme.bodySmall
                  ?.copyWith(color: ScotomaColors.muted),
            ),
          ],
        );
        final burn = OutlinedButton.icon(
          key: const Key('burn-session-button'),
          onPressed: onBurn,
          style: OutlinedButton.styleFrom(
            foregroundColor: ScotomaColors.danger,
            side: BorderSide(
              color: ScotomaColors.danger.withValues(alpha: 0.55),
            ),
          ),
          icon: const Icon(Icons.local_fire_department_outlined),
          label: const Text('Destruir'),
        );
        if (compact) {
          return Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [details, const SizedBox(height: 14), burn],
          );
        }
        return Row(
          crossAxisAlignment: CrossAxisAlignment.center,
          children: [
            Expanded(child: details),
            const SizedBox(width: 20),
            burn,
          ],
        );
      },
    ),
  );
}

class _ConnectionStatus extends StatelessWidget {
  const _ConnectionStatus();

  @override
  Widget build(BuildContext context) => Semantics(
    label: 'Estado: canal cifrado conectado',
    child: const Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Icon(
          Icons.verified_user_outlined,
          size: 19,
          color: ScotomaColors.secure,
        ),
        SizedBox(width: 8),
        Flexible(
          child: Text(
            'Canal cifrado conectado',
            style: TextStyle(
              color: ScotomaColors.secure,
              fontWeight: FontWeight.w600,
            ),
          ),
        ),
      ],
    ),
  );
}

class _EmptyChat extends StatelessWidget {
  const _EmptyChat();

  @override
  Widget build(BuildContext context) => Center(
    child: Padding(
      padding: const EdgeInsets.all(32),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(
            Icons.chat_bubble_outline_rounded,
            size: 38,
            color: ScotomaColors.muted.withValues(alpha: 0.65),
          ),
          const SizedBox(height: 14),
          const Text(
            'Canal conectado. Envía el primer mensaje.',
            key: Key('empty-chat-message'),
            textAlign: TextAlign.center,
          ),
          const SizedBox(height: 6),
          Text(
            'Los mensajes solo viven en esta sesión.',
            style: Theme.of(context).textTheme.bodySmall
                ?.copyWith(color: ScotomaColors.muted),
            textAlign: TextAlign.center,
          ),
        ],
      ),
    ),
  );
}

class _MessageComposer extends StatelessWidget {
  const _MessageComposer({
    required this.sendPending,
    required this.messageController,
    required this.onSend,
  });

  final bool sendPending;
  final TextEditingController messageController;
  final VoidCallback onSend;

  @override
  Widget build(BuildContext context) => Container(
    padding: const EdgeInsets.fromLTRB(16, 10, 16, 16),
    decoration: BoxDecoration(
      color: ScotomaColors.background.withValues(alpha: 0.92),
      border: const Border(
        top: BorderSide(color: ScotomaColors.outlineVariant),
      ),
    ),
    child: Row(
      crossAxisAlignment: CrossAxisAlignment.end,
      children: [
        Expanded(
          child: TextField(
            key: const Key('message-field'),
            controller: messageController,
            enabled: !sendPending,
            minLines: 1,
            maxLines: 5,
            maxLength: 4000,
            autocorrect: false,
            enableSuggestions: false,
            enableIMEPersonalizedLearning: false,
            autofillHints: null,
            textInputAction: TextInputAction.newline,
            decoration: const InputDecoration(
              hintText: 'Escribe un mensaje…',
              counterText: '',
            ),
          ),
        ),
        const SizedBox(width: 10),
        IconButton.filled(
          key: const Key('send-message-button'),
          onPressed: sendPending ? null : onSend,
          icon: sendPending
              ? const SizedBox.square(
                  dimension: 18,
                  child: CircularProgressIndicator(strokeWidth: 2),
                )
              : const Icon(Icons.send_rounded),
          tooltip: 'Enviar',
        ),
      ],
    ),
  );
}

final class _MessageBubble extends StatelessWidget {
  const _MessageBubble({required this.message, required this.index});

  final ChatMessage message;
  final int index;

  @override
  Widget build(BuildContext context) {
    final outgoing = message.direction == ChatDirection.outgoing;
    return Align(
      alignment: outgoing ? Alignment.centerRight : Alignment.centerLeft,
      child: Semantics(
        label: outgoing ? 'Mensaje enviado' : 'Mensaje recibido',
        child: Container(
          key: Key(
            outgoing ? 'outgoing-message-$index' : 'incoming-message-$index',
          ),
          constraints: const BoxConstraints(maxWidth: 360),
          margin: const EdgeInsets.only(bottom: 8),
          padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
          decoration: BoxDecoration(
            color: outgoing
                ? Theme.of(context).colorScheme.primaryContainer
                : Theme.of(context).colorScheme.surfaceContainerHighest,
            borderRadius: BorderRadius.circular(16),
          ),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.end,
            children: [
              Text(message.text),
              if (outgoing)
                Icon(
                  message.delivered
                      ? Icons.done_all_rounded
                      : Icons.done_rounded,
                  key: Key('delivery-$index'),
                  size: 16,
                ),
            ],
          ),
        ),
      ),
    );
  }
}

final class _Presentation {
  const _Presentation({
    required this.title,
    required this.message,
    required this.icon,
    required this.busy,
  });

  final String title;
  final String message;
  final IconData icon;
  final bool busy;
}
