import 'package:flutter/material.dart';

abstract final class ScotomaColors {
  static const voidBlack = Color(0xFF08060F);
  static const background = Color(0xFF0C0914);
  static const surface = Color(0xFF14101F);
  static const surfaceHigh = Color(0xFF1C1729);
  static const violet = Color(0xFF9567F7);
  static const violetLight = Color(0xFFC3A4FF);
  static const violetDeep = Color(0xFF5123A6);
  static const mist = Color(0xFFEAE5F2);
  static const muted = Color(0xFFA9A2B6);
  static const outline = Color(0xFF3B324D);
  static const outlineVariant = Color(0xFF2A2338);
  static const secure = Color(0xFF72D6C1);
  static const danger = Color(0xFFE28BA1);
}

ThemeData buildScotomaTheme() {
  const scheme = ColorScheme.dark(
    primary: ScotomaColors.violet,
    onPrimary: Color(0xFF16072B),
    primaryContainer: Color(0xFF382064),
    onPrimaryContainer: Color(0xFFEBDDFF),
    secondary: ScotomaColors.violetLight,
    onSecondary: Color(0xFF24133D),
    secondaryContainer: Color(0xFF34234B),
    onSecondaryContainer: Color(0xFFEADDFB),
    tertiary: ScotomaColors.secure,
    onTertiary: Color(0xFF062A22),
    error: ScotomaColors.danger,
    onError: Color(0xFF3B0717),
    surface: ScotomaColors.surface,
    onSurface: ScotomaColors.mist,
    onSurfaceVariant: ScotomaColors.muted,
    outline: ScotomaColors.outline,
    outlineVariant: Color(0xFF2A2338),
    shadow: Colors.black,
    scrim: Colors.black,
    inverseSurface: ScotomaColors.mist,
    onInverseSurface: ScotomaColors.background,
    inversePrimary: ScotomaColors.violetDeep,
  );

  final base = ThemeData(
    brightness: Brightness.dark,
    colorScheme: scheme,
    scaffoldBackgroundColor: ScotomaColors.background,
    useMaterial3: true,
  );
  final rounded = RoundedRectangleBorder(
    borderRadius: BorderRadius.circular(14),
  );

  return base.copyWith(
    textTheme: base.textTheme.copyWith(
      displaySmall: base.textTheme.displaySmall?.copyWith(
        fontWeight: FontWeight.w700,
        letterSpacing: -1.2,
        height: 1.08,
      ),
      headlineMedium: base.textTheme.headlineMedium?.copyWith(
        fontWeight: FontWeight.w600,
        letterSpacing: -0.4,
      ),
      headlineSmall: base.textTheme.headlineSmall?.copyWith(
        fontWeight: FontWeight.w600,
      ),
      titleLarge: base.textTheme.titleLarge?.copyWith(
        fontWeight: FontWeight.w600,
      ),
      titleMedium: base.textTheme.titleMedium?.copyWith(
        fontWeight: FontWeight.w600,
      ),
      bodyLarge: base.textTheme.bodyLarge?.copyWith(height: 1.5),
      bodyMedium: base.textTheme.bodyMedium?.copyWith(height: 1.45),
      labelLarge: base.textTheme.labelLarge?.copyWith(
        fontWeight: FontWeight.w600,
        letterSpacing: 0.2,
      ),
    ),
    appBarTheme: const AppBarTheme(
      backgroundColor: Colors.transparent,
      surfaceTintColor: Colors.transparent,
      centerTitle: false,
      elevation: 0,
    ),
    cardTheme: CardThemeData(
      color: ScotomaColors.surface.withValues(alpha: 0.94),
      surfaceTintColor: Colors.transparent,
      elevation: 0,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(24),
        side: const BorderSide(color: ScotomaColors.outline),
      ),
    ),
    inputDecorationTheme: InputDecorationTheme(
      filled: true,
      fillColor: ScotomaColors.voidBlack.withValues(alpha: 0.56),
      contentPadding: const EdgeInsets.symmetric(horizontal: 18, vertical: 18),
      labelStyle: const TextStyle(color: ScotomaColors.muted),
      hintStyle: TextStyle(color: ScotomaColors.muted.withValues(alpha: 0.7)),
      border: OutlineInputBorder(
        borderRadius: BorderRadius.circular(14),
        borderSide: const BorderSide(color: ScotomaColors.outline),
      ),
      enabledBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(14),
        borderSide: const BorderSide(color: ScotomaColors.outline),
      ),
      focusedBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(14),
        borderSide: const BorderSide(
          color: ScotomaColors.violetLight,
          width: 2,
        ),
      ),
      errorBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(14),
        borderSide: const BorderSide(color: ScotomaColors.danger),
      ),
      focusedErrorBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(14),
        borderSide: const BorderSide(color: ScotomaColors.danger, width: 2),
      ),
    ),
    filledButtonTheme: FilledButtonThemeData(
      style: FilledButton.styleFrom(
        minimumSize: const Size(48, 52),
        shape: rounded,
      ),
    ),
    outlinedButtonTheme: OutlinedButtonThemeData(
      style: OutlinedButton.styleFrom(
        minimumSize: const Size(48, 50),
        shape: rounded,
        side: const BorderSide(color: ScotomaColors.outline),
      ),
    ),
    iconButtonTheme: IconButtonThemeData(
      style: IconButton.styleFrom(minimumSize: const Size.square(48)),
    ),
    snackBarTheme: SnackBarThemeData(
      backgroundColor: ScotomaColors.surfaceHigh,
      contentTextStyle: base.textTheme.bodyMedium?.copyWith(
        color: ScotomaColors.mist,
      ),
      shape: rounded,
      behavior: SnackBarBehavior.floating,
    ),
    dividerTheme: const DividerThemeData(color: ScotomaColors.outlineVariant),
    progressIndicatorTheme: const ProgressIndicatorThemeData(
      color: ScotomaColors.violetLight,
    ),
    tooltipTheme: TooltipThemeData(
      decoration: BoxDecoration(
        color: ScotomaColors.mist,
        borderRadius: BorderRadius.circular(8),
      ),
      textStyle: const TextStyle(color: ScotomaColors.background),
    ),
  );
}
