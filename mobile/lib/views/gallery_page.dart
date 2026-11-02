import 'dart:io';
import 'package:flutter/material.dart';
import '../models/media_item.dart';

class GalleryPage extends StatelessWidget {
  final List<MediaItem> items;
  final VoidCallback onTriggerOffload;
  final VoidCallback? onTriggerSync;
  final VoidCallback? onRefresh;

  const GalleryPage({
    super.key,
    required this.items,
    required this.onTriggerOffload,
    this.onTriggerSync,
    this.onRefresh,
  });

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Mocambo Timeline'),
        actions: [
          if (onRefresh != null)
            IconButton(
              icon: const Icon(Icons.refresh),
              tooltip: 'Escanear / Atualizar Mídias',
              onPressed: onRefresh,
            ),
          if (onTriggerSync != null)
            IconButton(
              icon: const Icon(Icons.cloud_upload_outlined),
              tooltip: 'Sincronizar com o Servidor',
              onPressed: onTriggerSync,
            ),
          IconButton(
            icon: const Icon(Icons.cloud_sync_outlined),
            tooltip: 'Executar Offload',
            onPressed: onTriggerOffload,
          ),
        ],
      ),
      body: items.isEmpty
          ? Center(
              child: Padding(
                padding: const EdgeInsets.all(24.0),
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Icon(
                      Icons.photo_library_outlined,
                      size: 64,
                      color: Colors.grey.shade400,
                    ),
                    const SizedBox(height: 16),
                    const Text(
                      'Nenhuma foto ou vídeo na timeline local.',
                      style: TextStyle(color: Colors.grey, fontSize: 16),
                      textAlign: TextAlign.center,
                    ),
                    const SizedBox(height: 16),
                    Wrap(
                      spacing: 8,
                      runSpacing: 8,
                      alignment: WrapAlignment.center,
                      children: [
                        if (onRefresh != null)
                          FilledButton.icon(
                            onPressed: onRefresh,
                            icon: const Icon(Icons.search),
                            label: const Text('Escanear Galeria Local'),
                          ),
                        if (onTriggerSync != null)
                          OutlinedButton.icon(
                            onPressed: onTriggerSync,
                            icon: const Icon(Icons.cloud_upload),
                            label: const Text('Sincronizar com o Servidor'),
                          ),
                      ],
                    ),
                  ],
                ),
              ),
            )
          : GridView.builder(
              padding: const EdgeInsets.all(4),
              gridDelegate: const SliverGridDelegateWithFixedCrossAxisCount(
                crossAxisCount: 3,
                crossAxisSpacing: 4,
                mainAxisSpacing: 4,
              ),
              itemCount: items.length,
              itemBuilder: (context, index) {
                final item = items[index];
                final file = File(item.filePath);
                final fileExists = file.existsSync();

                return Stack(
                  fit: StackFit.expand,
                  children: [
                    Container(
                      color: Colors.grey.shade200,
                      child: fileExists && item.syncStatus != SyncStatus.offloaded
                          ? Image.file(
                              file,
                              fit: BoxFit.cover,
                              errorBuilder: (_, __, ___) => const Center(
                                child: Icon(Icons.broken_image, color: Colors.grey),
                              ),
                            )
                          : Center(
                              child: Icon(
                                Icons.image,
                                size: 36,
                                color: Colors.grey.shade500,
                              ),
                            ),
                    ),
                    if (item.syncStatus == SyncStatus.offloaded)
                      Positioned(
                        top: 4,
                        right: 4,
                        child: Container(
                          padding: const EdgeInsets.all(2),
                          decoration: BoxDecoration(
                            color: Colors.black54,
                            borderRadius: BorderRadius.circular(4),
                          ),
                          child: const Icon(
                            Icons.cloud_done,
                            color: Colors.white,
                            size: 16,
                          ),
                        ),
                      )
                    else if (item.syncStatus == SyncStatus.synced)
                      Positioned(
                        top: 4,
                        right: 4,
                        child: Container(
                          padding: const EdgeInsets.all(2),
                          decoration: BoxDecoration(
                            color: Colors.blue.shade700,
                            borderRadius: BorderRadius.circular(4),
                          ),
                          child: const Icon(
                            Icons.check,
                            color: Colors.white,
                            size: 16,
                          ),
                        ),
                      ),
                  ],
                );
              },
            ),
    );
  }
}
