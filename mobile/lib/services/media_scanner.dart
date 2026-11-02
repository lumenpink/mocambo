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

  /// Detecta a procedência/pasta de origem a partir do caminho do arquivo.
  static String detectSource(String fullPath) {
    final lower = fullPath.toLowerCase();
    if (lower.contains('whatsapp') || lower.contains('com.whatsapp')) {
      return 'WhatsApp';
    } else if (lower.contains('camera') ||
        lower.contains('dcim') ||
        lower.contains('100andro')) {
      return 'Camera';
    } else if (lower.contains('screenshot')) {
      return 'Screenshots';
    } else if (lower.contains('telegram')) {
      return 'Telegram';
    } else if (lower.contains('download')) {
      return 'Download';
    } else if (lower.contains('instagram')) {
      return 'Instagram';
    }
    return 'Outros';
  }

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
        final source = detectSource(entity.path);
        final relPath = p.relative(entity.path, from: directoryPath);

        items.add(
          MediaItem(
            id: hash,
            filePath: entity.path,
            fileName: name,
            fileSize: stat.size,
            createdAt: stat.modified,
            sha256: hash,
            source: source,
            originalRelativePath: relPath,
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
