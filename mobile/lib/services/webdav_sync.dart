import 'dart:io';
import 'package:http/http.dart' as http;
import '../config/app_config.dart';
import '../models/media_item.dart';

class WebDavSyncService {
  final AppConfig config;
  final http.Client _client;

  WebDavSyncService({
    required this.config,
    http.Client? client,
  }) : _client = client ?? http.Client();

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

  void close() {
    _client.close();
  }
}
