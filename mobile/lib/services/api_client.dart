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

  /// Checa se o arquivo existe e o hash confere no servidor via HEAD /api/files/check/{hash}
  Future<bool> checkFileExists(String sha256) async {
    try {
      final uri = Uri.parse('${config.cleanBaseUrl}/api/files/check/$sha256');
      final response = await _client.head(uri);
      return response.statusCode == 200;
    } catch (_) {
      return false;
    }
  }

  /// Busca semântica e textual via GET /api/search?q=...
  Future<List<SearchResultItem>> search(String query, {int limit = 20, int offset = 0}) async {
    try {
      final uri = Uri.parse(
        '${config.cleanBaseUrl}/api/search?q=${Uri.encodeComponent(query)}&limit=$limit&offset=$offset',
      );
      final response = await _client.get(uri);
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
