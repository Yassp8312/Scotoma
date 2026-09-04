import 'package:flutter/material.dart';
import 'package:scotoma/src/theme/scotoma_theme.dart';

class ScotomaBackground extends StatelessWidget {
  const ScotomaBackground({required this.child, super.key});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    return DecoratedBox(
      decoration: const BoxDecoration(
        color: ScotomaColors.background,
        gradient: RadialGradient(
          center: Alignment(-0.72, -0.82),
          radius: 1.25,
          colors: [Color(0xFF211338), ScotomaColors.background],
          stops: [0, 0.58],
        ),
      ),
      child: child,
    );
  }
}
