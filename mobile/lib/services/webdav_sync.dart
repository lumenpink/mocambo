import 'dart:io';
import 'package:http/http.dart' as http;
import '../config/app_config.dart';
import '../models/media_item.dart';

class SyncResult {
  final int totalPending;
  final int successfulUploads;
  final int failedUploads;

  const SyncResult({
    required this.totalPending,
    required this.successfulUploads,
    required this.failedUploads,
  });
}

class WebDavSyncService {
  final AppConfig config;
  final http.Client _client;

  WebDavSyncService({
    required this.config,
    http.Client? client,
  }) : _client = client ?? http.Client();

  Map<String, String> get _headers {
    final headers = <String, String>{};
    if (config.password.isNotEmpty) {
      headers['Authorization'] = 'Bearer ${config.password}';
    }
    return headers;
  }

  /// Sincroniza um item de mídia para o WebDAV do Mocambo (/dav/Timeline/YYYY/MM/arquivo.ext)
  Future<bool> uploadMedia(MediaItem item) async {
    try {
      final file = File(item.filePath);
      if (!file.existsSync()) {
        return false;
      }

      final bytes = await file.readAsBytes();
      final year = item.createdAt.year.toString();
      final month = item.createdAt.month.toString().padLeft(2, '0');
      final targetUri = Uri.parse(
        '${config.cleanBaseUrl}/dav/Timeline/$year/$month/${item.fileName}',
      );

      final response = await _client.put(
        targetUri,
        headers: _headers,
        body: bytes,
      );

      if (response.statusCode >= 200 && response.statusCode < 300) {
        item.syncStatus = SyncStatus.synced;
        return true;
      }
      return false;
    } catch (_) {
      return false;
    }
  }

  /// Sincroniza em lote todos os itens pendentes
  Future<SyncResult> syncPendingMedia(
    List<MediaItem> items, {
    void Function(int current, int total)? onProgress,
  }) async {
    final pending =
        items.where((item) => item.syncStatus == SyncStatus.pending).toList();
    int success = 0;
    int failed = 0;

    for (int i = 0; i < pending.length; i++) {
      onProgress?.call(i + 1, pending.length);
      final ok = await uploadMedia(pending[i]);
      if (ok) {
        success++;
      } else {
        failed++;
      }
    }

    return SyncResult(
      totalPending: pending.length,
      successfulUploads: success,
      failedUploads: failed,
    );
  }

  void close() {
    _client.close();
  }
}
