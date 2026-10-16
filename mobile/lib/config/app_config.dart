class AppConfig {
  final String serverUrl;
  final String username;
  final String password;
  final int retentionDays;

  const AppConfig({
    required this.serverUrl,
    this.username = 'default',
    this.password = '',
    this.retentionDays = 15,
  });

  String get cleanBaseUrl => serverUrl.replaceAll(RegExp(r'/+$'), '');
  String get checkUrlPrefix => '$cleanBaseUrl/api/files/check';
  String get searchUrl => '$cleanBaseUrl/api/search';
  String get webdavUrl => '$cleanBaseUrl/dav';
}
