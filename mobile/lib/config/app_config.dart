import 'dart:convert';
import 'dart:io';

class AppConfig {
  final String serverUrl;
  final String username;
  final String password;
  final int retentionDays;

  const AppConfig({
    this.serverUrl = '',
    this.username = '',
    this.password = '',
    this.retentionDays = 15,
  });

  bool get isConfigured =>
      serverUrl.trim().isNotEmpty && username.trim().isNotEmpty;

  String get cleanBaseUrl => serverUrl.replaceAll(RegExp(r'/+$'), '');
  String get checkUrlPrefix => '$cleanBaseUrl/api/files/check';
  String get searchUrl => '$cleanBaseUrl/api/search';
  String get webdavUrl => '$cleanBaseUrl/dav';
  String get healthUrl => '$cleanBaseUrl/health';

  Map<String, dynamic> toJson() => {
        'serverUrl': serverUrl,
        'username': username,
        'password': password,
        'retentionDays': retentionDays,
      };

  factory AppConfig.fromJson(Map<String, dynamic> json) => AppConfig(
        serverUrl: json['serverUrl'] as String? ?? '',
        username: json['username'] as String? ?? '',
        password: json['password'] as String? ?? '',
        retentionDays: json['retentionDays'] as int? ?? 15,
      );
}

class ConfigStore {
  final String filePath;

  const ConfigStore({required this.filePath});

  Future<AppConfig> load() async {
    final file = File(filePath);
    if (!await file.exists()) {
      return const AppConfig();
    }
    try {
      final content = await file.readAsString();
      final data = jsonDecode(content) as Map<String, dynamic>;
      return AppConfig.fromJson(data);
    } catch (_) {
      return const AppConfig();
    }
  }

  Future<void> save(AppConfig config) async {
    final file = File(filePath);
    await file.parent.create(recursive: true);
    await file.writeAsString(jsonEncode(config.toJson()));
  }
}
