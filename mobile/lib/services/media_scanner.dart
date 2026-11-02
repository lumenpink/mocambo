import 'dart:io';
import 'package:crypto/crypto.dart';
import 'package:path/path.dart' as p;
import '../models/media_item.dart';

class MediaScanner {
  static const Set<String> supportedExtensions = {
    '.jpg',
    '.jpeg',
    '.png',
    '.webp',
    '.gif',
    '.mp4',
    '.mov',
    '.mkv',
  };

  const MediaScanner();

  /// Varre um diretório do dispositivo por fotos e vídeos compatíveis.
  Future<List<MediaItem>> scanDirectory(
    String directoryPath, {
    bool recursive = false,
  }) async {
    final dir = Directory(directoryPath);
    if (!await dir.exists()) {
      return [];
    }

    final items = <MediaItem>[];

    await for (final entity in dir.list(recursive: recursive, followLinks: false)) {
      if (entity is! File) continue;

      final ext = p.extension(entity.path).toLowerCase();
      if (!supportedExtensions.contains(ext)) continue;

      try {
        final stat = await entity.stat();
        final bytes = await entity.readAsBytes();
        final hash = sha256.convert(bytes).toString();
        final name = p.basename(entity.path);

        items.add(
          MediaItem(
            id: hash,
            filePath: entity.path,
            fileName: name,
            fileSize: stat.size,
            createdAt: stat.modified,
            sha256: hash,
            syncStatus: SyncStatus.pending,
          ),
        );
      } catch (_) {
        // Ignora arquivos bloqueados ou corrompidos
      }
    }

    // Ordena do mais recente para o mais antigo
    items.sort((a, b) => b.createdAt.compareTo(a.createdAt));
    return items;
  }
}
