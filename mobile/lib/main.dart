import 'dart:io';
import 'package:flutter/material.dart';
import 'config/app_config.dart';
import 'models/media_item.dart';
import 'services/api_client.dart';
import 'services/media_scanner.dart';
import 'services/offload_manager.dart';
import 'services/permission_service.dart';
import 'services/webdav_sync.dart';
import 'views/gallery_page.dart';
import 'views/search_page.dart';
import 'views/settings_page.dart';

void main() {
  WidgetsFlutterBinding.ensureInitialized();
  runApp(const MocamboApp());
}

class MocamboApp extends StatefulWidget {
  const MocamboApp({super.key});

  @override
  State<MocamboApp> createState() => _MocamboAppState();
}

class _MocamboAppState extends State<MocamboApp> {
  AppConfig _config = const AppConfig();
  ConfigStore? _configStore;
  late ApiClient _apiClient;
  late WebDavSyncService _webDavSync;
  late OffloadManager _offloadManager;
  final MediaScanner _mediaScanner = const MediaScanner();
  final List<MediaItem> _items = [];
  int _currentIndex = 0;

  @override
  void initState() {
    super.initState();
    _initServices();
    _loadConfig();
  }

  Future<void> _loadConfig() async {
    final configFile = await AppConfig.defaultConfigFile();
    final store = ConfigStore(filePath: configFile);
    _configStore = store;
    final loaded = await store.load();
    if (loaded.isConfigured || loaded.serverUrl.isNotEmpty) {
      if (!mounted) return;
      setState(() {
        _config = loaded;
        _initServices();
      });
    }
    await _handleScanMedia();
  }

  void _initServices() {
    _apiClient = ApiClient(config: _config);
    _webDavSync = WebDavSyncService(config: _config);
    _offloadManager = OffloadManager(config: _config, apiClient: _apiClient);
  }

  void _updateConfig(AppConfig newConfig) {
    setState(() {
      _config = newConfig;
      _apiClient.close();
      _webDavSync.close();
      _initServices();
    });
    _configStore?.save(newConfig);
  }

  Future<void> _handleScanMedia() async {
    // 1. Solicita permissão em runtime (no Android)
    await PermissionService.requestStoragePermissions();

    final home = Platform.environment['HOME'] ?? '';
    final candidates = [
      '/storage/emulated/0/DCIM/Camera',
      '/storage/emulated/0/DCIM',
      '/storage/emulated/0/Pictures',
      '/storage/emulated/0/Download',
      '/sdcard/DCIM/Camera',
      '/sdcard/Pictures',
      '/sdcard/Download',
      if (home.isNotEmpty) '$home/Pictures',
      './test_media',
    ];

    final allScanned = <MediaItem>[];
    for (final dir in candidates) {
      try {
        final d = Directory(dir);
        if (d.existsSync()) {
          final scanned = await _mediaScanner.scanDirectory(dir);
          allScanned.addAll(scanned);
        }
      } catch (_) {
        // Ignora diretórios inacessíveis silenciosamente
      }
    }

    if (!mounted) return;
    setState(() {
      for (final item in allScanned) {
        final idx = _items.indexWhere((i) => i.id == item.id);
        if (idx >= 0) {
          _items[idx] = item;
        } else {
          _items.add(item);
        }
      }
      _items.sort((a, b) => b.createdAt.compareTo(a.createdAt));
    });
  }

  Future<void> _handleSyncWithServer() async {
    if (!_config.isConfigured) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(
          content: Text(
            'Servidor não configurado. Por favor, acesse os Ajustes primeiro.',
          ),
          backgroundColor: Colors.orange,
        ),
      );
      return;
    }

    final healthy = await _apiClient.handshake();
    if (!healthy) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(
            'Falha de handshake: servidor inalcançável em ${_config.cleanBaseUrl}.',
          ),
          backgroundColor: Colors.red,
        ),
      );
      return;
    }

    final syncResult = await _webDavSync.syncPendingMedia(_items);
    final offloadResult = await _offloadManager.processOffload(_items);

    if (!mounted) return;
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text(
          'Sincronizado: ${syncResult.successfulUploads} enviados (${syncResult.failedUploads} erros). '
          'Offload: ${offloadResult.offloadedCount} liberados.',
        ),
        backgroundColor: Colors.green,
      ),
    );
    setState(() {});
  }

  @override
  void dispose() {
    _apiClient.close();
    _webDavSync.close();
    super.dispose();
  }

  Future<void> _handleOffload() async {
    final result = await _offloadManager.processOffload(_items);
    if (!mounted) return;
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text(
          'Offload de ${_config.retentionDays} dias: ${result.offloadedCount} liberados, ${result.skippedCount} mantidos.',
        ),
      ),
    );
    setState(() {});
  }

  @override
  Widget build(BuildContext context) {
    final pages = [
      GalleryPage(
        items: _items,
        onTriggerOffload: _handleOffload,
        onTriggerSync: _handleSyncWithServer,
        onRefresh: _handleScanMedia,
      ),
      SearchPage(apiClient: _apiClient),
      SettingsPage(
        config: _config,
        onConfigSaved: _updateConfig,
      ),
    ];

    return MaterialApp(
      title: 'Mocambo',
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(seedColor: Colors.deepPurple),
        useMaterial3: true,
      ),
      home: Scaffold(
        body: pages[_currentIndex],
        bottomNavigationBar: NavigationBar(
          selectedIndex: _currentIndex,
          onDestinationSelected: (index) => setState(() => _currentIndex = index),
          destinations: const [
            NavigationDestination(icon: Icon(Icons.photo_library), label: 'Galeria'),
            NavigationDestination(icon: Icon(Icons.search), label: 'Busca'),
            NavigationDestination(icon: Icon(Icons.settings), label: 'Ajustes'),
          ],
        ),
      ),
    );
  }
}
