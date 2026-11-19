import 'dart:io';
import 'package:flutter/services.dart';

class PermissionService {
  static const MethodChannel _channel =
      MethodChannel('in.twolp.mocambo/permissions');

  /// Solicita permissões de leitura de mídia/armazenamento.
  /// Em Android, invoca o diálogo nativo do sistema operacional.
  /// Em outras plataformas (Desktop, Linux, testes), retorna true imediatamente.
  static Future<bool> requestStoragePermissions() async {
    if (!Platform.isAndroid) {
      return true;
    }
    try {
      final granted =
          await _channel.invokeMethod<bool>('requestStoragePermissions');
      return granted ?? false;
    } catch (_) {
      // Se falhar a chamada de canal (ex.: testes unitários sem mock), fallback seguro
      return true;
    }
  }

  /// Verifica se as permissões de leitura de mídia já estão concedidas.
  static Future<bool> hasStoragePermissions() async {
    if (!Platform.isAndroid) {
      return true;
    }
    try {
      final granted =
          await _channel.invokeMethod<bool>('hasStoragePermissions');
      return granted ?? false;
    } catch (_) {
      return true;
    }
  }
}
