import 'dart:io';
import 'package:http/http.dart' as http;
import '../config/app_config.dart';
import '../models/media_item.dart';
import 'api_client.dart';

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
  final ApiClient _apiClient;

  WebDavSyncService({
    required this.config,
    http.Client? client,
    ApiClient? apiClient,
  })  : _client = client ?? http.Client(),
        _apiClient = apiClient ?? ApiClient(config: config, client: client);

  Map<String, String> get _headers {
    final headers = <String, String>{};
    if (config.password.isNotEmpty) {
      headers['Authorization'] = 'Bearer ${config.password}';
    }
    return headers;
  }

  /// Sincroniza um item de mídia para o WebDAV do Mocambo com deduplicação prévia e metadados de origem
  Future<bool> uploadMedia(MediaItem item) async {
    try {
      final file = File(item.filePath);
      if (!file.existsSync()) {
        return false;
      }

      final year = item.createdAt.year.toString();
      final month = item.createdAt.month.toString().padLeft(2, '0');
      final storagePath = '/dav/Timeline/$year/$month/${item.fileName}';

      // 1. Pré-checagem de deduplicação por hash SHA-256 no servidor
      final alreadyExists = await _apiClient.checkHash(item.sha256);
      if (alreadyExists) {
        final linked = await _apiClient.dedupFile(
          hash: item.sha256,
          storagePath: storagePath,
          fileName: item.fileName,
          source: item.source,
          originalPath: item.originalRelativePath,
        );
        if (linked) {
          item.syncStatus = SyncStatus.synced;
          return true;
        }
      }

      // 2. Upload completo via WebDAV PUT com cabeçalhos de contexto de procedência
      final bytes = await file.readAsBytes();
      final targetUri = Uri.parse('${config.cleanBaseUrl}$storagePath');

      final headers = Map<String, String>.from(_headers);
      headers['X-Mocambo-Source'] = item.source;
      if (item.originalRelativePath.isNotEmpty) {
        headers['X-Mocambo-Original-Path'] = item.originalRelativePath;
      }

      final response = await _client.put(
        targetUri,
        headers: headers,
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
