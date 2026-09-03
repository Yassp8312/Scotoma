import 'package:flutter/material.dart';
import 'package:scotoma/src/session/live_session_controller.dart';
import 'package:scotoma/src/session/session_backend.dart';

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
        appBar: AppBar(title: const Text('Sesión SCOTOMA')),
        body: SafeArea(
          child: _controller.phase == LiveSessionPhase.connected
              ? _ChatView(
                  messages: _controller.messages,
                  sendPending: _controller.sendPending,
                  messageController: _messageController,
                  onSend: _sendMessage,
                  onBurn: _closeAndReturn,
                )
              : Center(
                  child: ConstrainedBox(
                    constraints: const BoxConstraints(maxWidth: 520),
                    child: Padding(
                      padding: const EdgeInsets.all(24),
                      child: Column(
                        mainAxisAlignment: MainAxisAlignment.center,
                        children: [
                          if (presentation.busy)
                            const CircularProgressIndicator(
                              key: Key('session-progress'),
                            )
                          else
                            Icon(
                              presentation.icon,
                              key: const Key('session-status-icon'),
                              size: 64,
                              color: Theme.of(context).colorScheme.primary,
                            ),
                          const SizedBox(height: 24),
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
                            textAlign: TextAlign.center,
                          ),
                          const SizedBox(height: 28),
                          OutlinedButton.icon(
                            key: const Key('cancel-session-button'),
                            onPressed: _closeAndReturn,
                            icon: const Icon(Icons.close_rounded),
                            label: Text(
                              _controller.phase == LiveSessionPhase.burning
                                  ? 'Destruyendo…'
                                  : 'Cancelar y volver',
                            ),
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
    required this.messageController,
    required this.onSend,
    required this.onBurn,
  });

  final List<ChatMessage> messages;
  final bool sendPending;
  final TextEditingController messageController;
  final VoidCallback onSend;
  final VoidCallback onBurn;

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        MaterialBanner(
          content: const Text(
            'Canal Noise confirmado. Los mensajes solo permanecen en esta pantalla durante la sesión.',
          ),
          leading: const Icon(Icons.verified_user_outlined),
          actions: [
            TextButton.icon(
              key: const Key('burn-session-button'),
              onPressed: onBurn,
              icon: const Icon(Icons.local_fire_department_outlined),
              label: const Text('Destruir sesión'),
            ),
          ],
        ),
        Expanded(
          child: messages.isEmpty
              ? const Center(
                  child: Text(
                    'Canal conectado. Envía el primer mensaje.',
                    key: Key('empty-chat-message'),
                  ),
                )
              : ListView.builder(
                  key: const Key('message-list'),
                  padding: const EdgeInsets.all(16),
                  itemCount: messages.length,
                  itemBuilder: (context, index) =>
                      _MessageBubble(message: messages[index], index: index),
                ),
        ),
        Padding(
          padding: const EdgeInsets.fromLTRB(12, 8, 12, 12),
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
                    hintText: 'Mensaje',
                    counterText: '',
                  ),
                ),
              ),
              const SizedBox(width: 8),
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
        ),
      ],
    );
  }
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
