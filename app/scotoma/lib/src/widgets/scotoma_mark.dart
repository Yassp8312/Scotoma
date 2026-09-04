import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:scotoma/src/theme/scotoma_theme.dart';

class ScotomaMark extends StatelessWidget {
  const ScotomaMark({this.size = 96, this.showGlow = true, super.key});

  final double size;
  final bool showGlow;

  @override
  Widget build(BuildContext context) {
    return Semantics(
      image: true,
      label: 'Símbolo de SCOTOMA',
      child: ExcludeSemantics(
        child: RepaintBoundary(
          child: SizedBox.square(
            dimension: size,
            child: CustomPaint(painter: _ScotomaMarkPainter(showGlow)),
          ),
        ),
      ),
    );
  }
}

class _ScotomaMarkPainter extends CustomPainter {
  const _ScotomaMarkPainter(this.showGlow);

  final bool showGlow;

  @override
  void paint(Canvas canvas, Size size) {
    final rect = Offset.zero & size;
    final eye = Path()
      ..moveTo(size.width * 0.06, size.height * 0.5)
      ..cubicTo(
        size.width * 0.27,
        size.height * 0.08,
        size.width * 0.36,
        size.height * 0.08,
        size.width * 0.5,
        size.height * 0.13,
      )
      ..cubicTo(
        size.width * 0.67,
        size.height * 0.08,
        size.width * 0.76,
        size.height * 0.1,
        size.width * 0.94,
        size.height * 0.5,
      )
      ..cubicTo(
        size.width * 0.75,
        size.height * 0.9,
        size.width * 0.65,
        size.height * 0.92,
        size.width * 0.5,
        size.height * 0.87,
      )
      ..cubicTo(
        size.width * 0.35,
        size.height * 0.92,
        size.width * 0.25,
        size.height * 0.9,
        size.width * 0.06,
        size.height * 0.5,
      )
      ..close();

    if (showGlow) {
      canvas.drawPath(
        eye,
        Paint()
          ..color = ScotomaColors.violet.withValues(alpha: 0.18)
          ..maskFilter = MaskFilter.blur(BlurStyle.normal, size.width * 0.1),
      );
    }
    canvas.drawPath(
      eye,
      Paint()
        ..shader = const LinearGradient(
          colors: [
            ScotomaColors.violetLight,
            ScotomaColors.violet,
            ScotomaColors.violetDeep,
          ],
          stops: [0, 0.47, 1],
        ).createShader(rect),
    );

    canvas.save();
    canvas.clipPath(eye);
    final dotPaint = Paint()
      ..color = ScotomaColors.voidBlack.withValues(alpha: 0.84);
    final unit = size.width / 24;
    for (var column = -7; column <= 7; column++) {
      for (var row = -5; row <= 5; row++) {
        final distance = math.sqrt(column * column + row * row);
        if (distance < 2.7 || distance > 7.5 || (column + row).isOdd) continue;
        final dot = unit * (distance < 5 ? 1.05 : 0.76);
        final center = Offset(
          size.width * 0.5 + column * unit * 1.32,
          size.height * 0.5 + row * unit * 1.24,
        );
        canvas.drawRect(
          Rect.fromCenter(center: center, width: dot, height: dot),
          dotPaint,
        );
      }
    }
    canvas.restore();

    canvas.drawCircle(
      rect.center,
      size.width * 0.19,
      Paint()..color = ScotomaColors.voidBlack,
    );
    canvas.drawRect(
      Rect.fromCenter(
        center: rect.center,
        width: math.max(3, size.width * 0.035),
        height: size.height * 0.82,
      ),
      Paint()..color = ScotomaColors.voidBlack,
    );
  }

  @override
  bool shouldRepaint(covariant _ScotomaMarkPainter oldDelegate) =>
      oldDelegate.showGlow != showGlow;
}
