import 'package:flutter/material.dart';
import 'package:scotoma/src/home/home_screen.dart';
import 'package:scotoma/src/session/session_backend.dart';

class ScotomaApp extends StatelessWidget {
  const ScotomaApp({required this.backend, super.key});

  final SessionBackend backend;

  @override
  Widget build(BuildContext context) {
    final colorScheme = ColorScheme.fromSeed(
      seedColor: const Color(0xFF70E1F5),
      brightness: Brightness.dark,
      surface: const Color(0xFF11161B),
    );

    return MaterialApp(
      title: 'SCOTOMA',
      debugShowCheckedModeBanner: false,
      theme: ThemeData(
        colorScheme: colorScheme,
        scaffoldBackgroundColor: const Color(0xFF080B0E),
        useMaterial3: true,
        inputDecorationTheme: InputDecorationTheme(
          filled: true,
          fillColor: const Color(0xFF151B21),
          border: OutlineInputBorder(
            borderRadius: BorderRadius.circular(14),
            borderSide: const BorderSide(color: Color(0xFF34414B)),
          ),
          enabledBorder: OutlineInputBorder(
            borderRadius: BorderRadius.circular(14),
            borderSide: const BorderSide(color: Color(0xFF34414B)),
          ),
          focusedBorder: OutlineInputBorder(
            borderRadius: BorderRadius.circular(14),
            borderSide: BorderSide(color: colorScheme.primary, width: 2),
          ),
        ),
      ),
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
    theme: ThemeData.dark(useMaterial3: true),
    home: const Scaffold(
      body: SafeArea(
        child: Center(
          child: Padding(
            padding: EdgeInsets.all(24),
            child: Text(
              'SCOTOMA no puede preparar una sesión segura en este dispositivo. '
              'No se habilitó la entrada de credenciales.',
              key: Key('runtime-unavailable'),
              textAlign: TextAlign.center,
            ),
          ),
        ),
      ),
    ),
  );
}
