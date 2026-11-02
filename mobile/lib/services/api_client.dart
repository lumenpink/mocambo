import 'dart:convert';
import 'package:http/http.dart' as http;
import '../config/app_config.dart';
import '../models/search_result.dart';

class ApiClient {
  final AppConfig config;
  final http.Client _client;

  ApiClient({
    required this.config,
    http.Client? client,
  }) : _client = client ?? http.Client();

  Map<String, String> get _headers {
    final headers = <String, String>{};
    if (config.password.isNotEmpty) {
      headers['Authorization'] = 'Bearer ${config.password}';
    }
    return headers;
  }

  /// Realiza o handshake com o backend checando o endpoint /health.
  Future<bool> handshake() async {
    if (!config.isConfigured) return false;
    try {
      final uri = Uri.parse(config.healthUrl);
      final response = await _client
          .get(uri, headers: _headers)
          .timeout(const Duration(seconds: 5));
      return response.statusCode == 200;
    } catch (_) {
      return false;
    }
  }

  /// Checa se o arquivo existe e o hash confere no servidor via HEAD /api/files/check/{hash}
  Future<bool> checkFileExists(String sha256) async {
    try {
      final uri = Uri.parse('${config.cleanBaseUrl}/api/files/check/$sha256');
      final response = await _client.head(uri, headers: _headers);
      return response.statusCode == 200;
    } catch (_) {
      return false;
    }
  }

  /// Alias semântico para verificação rápida de hash SHA-256.
  Future<bool> checkHash(String sha256) async => checkFileExists(sha256);

  /// Vincula um arquivo duplicado no servidor via POST /api/files/dedup evitando reenvio de bytes.
  Future<bool> dedupFile({
    required String hash,
    required String storagePath,
    required String fileName,
    String? source,
    String? originalPath,
  }) async {
    try {
      final uri = Uri.parse('${config.cleanBaseUrl}/api/files/dedup');
      final headers = Map<String, String>.from(_headers);
      headers['Content-Type'] = 'application/json';
      final payload = jsonEncode({
        'sha256': hash,
        'storage_path': storagePath,
        'file_name': fileName,
        'source': source,
        'original_path': originalPath,
      });
      final response = await _client.post(uri, headers: headers, body: payload);
      return response.statusCode == 201 || response.statusCode == 200;
    } catch (_) {
      return false;
    }
  }

  /// Busca semântica e textual via GET /api/search?q=...
  Future<List<SearchResultItem>> search(String query,
      {int limit = 20, int offset = 0}) async {
    try {
      final uri = Uri.parse(
        '${config.cleanBaseUrl}/api/search?q=${Uri.encodeComponent(query)}&limit=$limit&offset=$offset',
      );
      final response = await _client.get(uri, headers: _headers);
      if (response.statusCode == 200) {
        final data = jsonDecode(response.body) as Map<String, dynamic>;
        final results = data['results'] as List<dynamic>? ?? [];
        return results
            .map((item) => SearchResultItem.fromJson(item as Map<String, dynamic>))
            .toList();
      }
      return [];
    } catch (_) {
      return [];
    }
  }

  void close() {
    _client.close();
  }
}
