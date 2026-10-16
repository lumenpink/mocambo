import 'dart:io';
import '../config/app_config.dart';
import '../models/media_item.dart';
import 'api_client.dart';

class OffloadResult {
  final int totalEvaluated;
  final int eligibleCount;
  final int offloadedCount;
  final int skippedCount;

  const OffloadResult({
    required this.totalEvaluated,
    required this.eligibleCount,
    required this.offloadedCount,
    required this.skippedCount,
  });
}

class OffloadManager {
  final AppConfig config;
  final ApiClient apiClient;

  OffloadManager({
    required this.config,
    required this.apiClient,
  });

  /// Executa o ciclo de retenção e offload inteligente de 15 dias.
  /// Mídias com idade >= 15 dias são checadas via HEAD /api/files/check/{hash}.
  /// Se o backend confirmar 200 OK, o arquivo local é removido com segurança.
  Future<OffloadResult> processOffload(
    List<MediaItem> items, {
    DateTime? referenceTime,
    bool Function(String path)? fileDeleter,
  }) async {
    final now = referenceTime ?? DateTime.now();
    int eligible = 0;
    int offloaded = 0;
    int skipped = 0;

    for (final item in items) {
      if (item.syncStatus == SyncStatus.offloaded) {
        continue;
      }

      // Regra 1: Validação da idade mínima de 15 dias
      if (!item.isEligibleForOffload(
        retentionDays: config.retentionDays,
        referenceTime: now,
      )) {
        skipped++;
        continue;
      }

      eligible++;

      // Regra 2: Validação criptográfica de integridade pré-expurgo
      // Consulta se o arquivo existe e o hash bate no servidor
      final existsOnServer = await apiClient.checkFileExists(item.sha256);
      if (!existsOnServer) {
        skipped++;
        continue;
      }

      // Regra 3: Exclusão local segura apenas após confirmação do servidor
      bool deleted = false;
      if (fileDeleter != null) {
        deleted = fileDeleter(item.filePath);
      } else {
        final file = File(item.filePath);
        if (file.existsSync()) {
          try {
            file.deleteSync();
            deleted = true;
          } catch (_) {
            deleted = false;
          }
        } else {
          deleted = true;
        }
      }

      if (deleted) {
        item.syncStatus = SyncStatus.offloaded;
        offloaded++;
      } else {
        skipped++;
      }
    }

    return OffloadResult(
      totalEvaluated: items.length,
      eligibleCount: eligible,
      offloadedCount: offloaded,
      skippedCount: skipped,
    );
  }
}
