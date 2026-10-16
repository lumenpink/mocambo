import 'package:flutter/material.dart';
import '../models/media_item.dart';

class GalleryPage extends StatelessWidget {
  final List<MediaItem> items;
  final VoidCallback onTriggerOffload;

  const GalleryPage({
    super.key,
    required this.items,
    required this.onTriggerOffload,
  });

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Mocambo Timeline'),
        actions: [
          IconButton(
            icon: const Icon(Icons.cloud_sync_outlined),
            tooltip: 'Executar Offload de 15 Dias',
            onPressed: onTriggerOffload,
          ),
        ],
      ),
      body: items.isEmpty
          ? const Center(
              child: Text(
                'Nenhuma foto ou vídeo na timeline local.',
                style: TextStyle(color: Colors.grey),
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
                return Stack(
                  fit: StackFit.expand,
                  children: [
                    Container(
                      color: Colors.grey.shade300,
                      child: Center(
                        child: Icon(
                          Icons.image,
                          size: 36,
                          color: Colors.grey.shade600,
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
                      ),
                  ],
                );
              },
            ),
    );
  }
}
