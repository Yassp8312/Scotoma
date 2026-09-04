import 'package:flutter/material.dart';
import 'package:scotoma/src/home/home_screen.dart';
import 'package:scotoma/src/session/session_backend.dart';
import 'package:scotoma/src/theme/scotoma_theme.dart';
import 'package:scotoma/src/widgets/scotoma_background.dart';
import 'package:scotoma/src/widgets/scotoma_mark.dart';

class ScotomaApp extends StatelessWidget {
  const ScotomaApp({required this.backend, super.key});

  final SessionBackend backend;

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'SCOTOMA',
      debugShowCheckedModeBanner: false,
      theme: buildScotomaTheme(),
      home: HomeScreen(backend: backend),
    );
  }
}

class ScotomaUnavailableApp extends StatelessWidget {
  const ScotomaUnavailableApp({super.key});

  @override
  Widget build(BuildContext context) => MaterialApp(
    title: 'SCOTOMA',
    debugShowCheckedModeBanner: false,
    theme: buildScotomaTheme(),
    home: const Scaffold(
      body: ScotomaBackground(
        child: SafeArea(
          child: Center(
            child: Padding(
              padding: EdgeInsets.all(24),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  ScotomaMark(size: 84),
                  SizedBox(height: 24),
                  Text(
                    'SCOTOMA no puede preparar una sesión segura en este dispositivo. '
                    'No se habilitó la entrada de credenciales.',
                    key: Key('runtime-unavailable'),
                    textAlign: TextAlign.center,
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
