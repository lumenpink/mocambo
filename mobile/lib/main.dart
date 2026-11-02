import 'package:flutter/material.dart';
import 'config/app_config.dart';
import 'models/media_item.dart';
import 'services/api_client.dart';
import 'services/offload_manager.dart';
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
  late ApiClient _apiClient;
  late OffloadManager _offloadManager;
  final List<MediaItem> _items = [];
  int _currentIndex = 0;

  @override
  void initState() {
    super.initState();
    _initServices();
  }

  void _initServices() {
    _apiClient = ApiClient(config: _config);
    _offloadManager = OffloadManager(config: _config, apiClient: _apiClient);
  }

  void _updateConfig(AppConfig newConfig) {
    setState(() {
      _config = newConfig;
      _apiClient.close();
      _initServices();
    });
  }

  @override
  void dispose() {
    _apiClient.close();
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
      GalleryPage(items: _items, onTriggerOffload: _handleOffload),
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
