import 'dart:io';
import 'package:flutter/material.dart';
import '../config/app_config.dart';
import '../models/media_item.dart';

class GalleryPage extends StatefulWidget {
  final List<MediaItem> items;
  final AppConfig config;
  final bool isSyncing;
  final String? syncProgressMessage;
  final VoidCallback onTriggerOffload;
  final VoidCallback? onTriggerSync;
  final VoidCallback? onRefresh;
  final VoidCallback? onOpenSettings;

  const GalleryPage({
    super.key,
    required this.items,
    this.config = const AppConfig(),
    this.isSyncing = false,
    this.syncProgressMessage,
    required this.onTriggerOffload,
    this.onTriggerSync,
    this.onRefresh,
    this.onOpenSettings,
  });

  @override
  State<GalleryPage> createState() => _GalleryPageState();
}

class _GalleryPageState extends State<GalleryPage> {
  String _selectedSource = 'Todas';

  static const List<String> _sourceFilters = [
    'Todas',
    'Câmera',
    'WhatsApp',
    'Screenshots',
    'Telegram',
    'Download',
  ];

  @override
  Widget build(BuildContext context) {
    final filteredItems = _selectedSource == 'Todas'
        ? widget.items
        : widget.items.where((item) {
            if (_selectedSource == 'Câmera') {
              return item.source.toLowerCase() == 'camera';
            }
            return item.source.toLowerCase() == _selectedSource.toLowerCase();
          }).toList();

    return Scaffold(
      appBar: AppBar(
        title: const Text('Mocambo Timeline'),
        actions: [
          if (widget.onRefresh != null)
            IconButton(
              icon: const Icon(Icons.refresh),
              tooltip: 'Escanear / Atualizar Mídias',
              onPressed: widget.onRefresh,
            ),
          if (widget.onTriggerSync != null)
            IconButton(
              icon: const Icon(Icons.cloud_upload_outlined),
              tooltip: 'Sincronizar com o Servidor',
              onPressed: widget.onTriggerSync,
            ),
          IconButton(
            icon: const Icon(Icons.cloud_sync_outlined),
            tooltip: 'Executar Offload',
            onPressed: widget.onTriggerOffload,
          ),
        ],
      ),
      body: Column(
        children: [
          if (!widget.config.isConfigured)
            Container(
              margin: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
              padding: const EdgeInsets.all(14),
              decoration: BoxDecoration(
                color: Theme.of(context)
                    .colorScheme
                    .primaryContainer
                    .withOpacity(0.4),
                borderRadius: BorderRadius.circular(12),
                border: Border.all(
                  color: Theme.of(context)
                      .colorScheme
                      .primary
                      .withOpacity(0.3),
                ),
              ),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      Icon(
                        Icons.cloud_off_outlined,
                        color: Theme.of(context).colorScheme.primary,
                        size: 24,
                      ),
                      const SizedBox(width: 8),
                      const Expanded(
                        child: Text(
                          'Mocambo Vault não configurado',
                          style: TextStyle(
                            fontWeight: FontWeight.bold,
                            fontSize: 14,
                          ),
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 6),
                  const Text(
                    'Conecte-se ao seu servidor soberano (ex.: mocambo.lohn.in) para sincronização automática da galeria e integração com o Zotero.',
                    style: TextStyle(fontSize: 12),
                  ),
                  const SizedBox(height: 10),
                  Align(
                    alignment: Alignment.centerRight,
                    child: ElevatedButton.icon(
                      onPressed: widget.onOpenSettings,
                      icon: const Icon(Icons.settings, size: 16),
                      label: const Text('Configurar Servidor'),
                    ),
                  ),
                ],
              ),
            )
          else if (widget.isSyncing)
            Container(
              margin: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
              decoration: BoxDecoration(
                color: Colors.blue.shade50,
                borderRadius: BorderRadius.circular(8),
                border: Border.all(color: Colors.blue.shade200),
              ),
              child: Row(
                children: [
                  const SizedBox(
                    width: 16,
                    height: 16,
                    child: CircularProgressIndicator(strokeWidth: 2),
                  ),
                  const SizedBox(width: 10),
                  Expanded(
                    child: Text(
                      widget.syncProgressMessage ??
                          'Sincronizando com o servidor...',
                      style:
                          TextStyle(fontSize: 12, color: Colors.blue.shade900),
                    ),
                  ),
                ],
              ),
            ),
          SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
            child: Row(
              children: _sourceFilters.map((source) {
                final isSelected = _selectedSource == source;
                return Padding(
                  padding: const EdgeInsets.only(right: 6),
                  child: FilterChip(
                    label: Text(source),
                    selected: isSelected,
                    onSelected: (_) {
                      setState(() {
                        _selectedSource = source;
                      });
                    },
                  ),
                );
              }).toList(),
            ),
          ),
          Expanded(
            child: filteredItems.isEmpty
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
                          Text(
                            _selectedSource == 'Todas'
                                ? 'Nenhuma foto ou vídeo na timeline local.'
                                : 'Nenhuma mídia encontrada da origem "$_selectedSource".',
                            style: const TextStyle(color: Colors.grey, fontSize: 16),
                            textAlign: TextAlign.center,
                          ),
                          const SizedBox(height: 16),
                          Wrap(
                            spacing: 8,
                            runSpacing: 8,
                            alignment: WrapAlignment.center,
                            children: [
                              if (widget.onRefresh != null)
                                FilledButton.icon(
                                  onPressed: widget.onRefresh,
                                  icon: const Icon(Icons.search),
                                  label: const Text('Escanear Galeria Local'),
                                ),
                              if (widget.onTriggerSync != null)
                                OutlinedButton.icon(
                                  onPressed: widget.onTriggerSync,
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
                    itemCount: filteredItems.length,
                    itemBuilder: (context, index) {
                      final item = filteredItems[index];
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
                                      child: Icon(Icons.broken_image,
                                          color: Colors.grey),
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
                          if (item.source.isNotEmpty && item.source != 'Outros')
                            Positioned(
                              bottom: 4,
                              left: 4,
                              child: Container(
                                padding: const EdgeInsets.symmetric(
                                    horizontal: 4, vertical: 2),
                                decoration: BoxDecoration(
                                  color: Colors.black54,
                                  borderRadius: BorderRadius.circular(4),
                                ),
                                child: Text(
                                  item.source,
                                  style: const TextStyle(
                                      color: Colors.white, fontSize: 10),
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
          ),
        ],
      ),
    );
  }
}
