import 'package:flutter/material.dart';
import '../models/search_result.dart';
import '../services/api_client.dart';

class SearchPage extends StatefulWidget {
  final ApiClient apiClient;

  const SearchPage({super.key, required this.apiClient});

  @override
  State<SearchPage> createState() => _SearchPageState();
}

class _SearchPageState extends State<SearchPage> {
  final TextEditingController _controller = TextEditingController();
  List<SearchResultItem> _results = [];
  bool _loading = false;

  Future<void> _performSearch(String query) async {
    if (query.trim().isEmpty) return;
    setState(() => _loading = true);
    final results = await widget.apiClient.search(query);
    setState(() {
      _results = results;
      _loading = false;
    });
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: TextField(
          controller: _controller,
          decoration: const InputDecoration(
            hintText: 'Buscar documentos ou fotos...',
            border: InputBorder.none,
          ),
          onSubmitted: _performSearch,
        ),
        actions: [
          IconButton(
            icon: const Icon(Icons.search),
            onPressed: () => _performSearch(_controller.text),
          ),
        ],
      ),
      body: _loading
          ? const Center(child: CircularProgressIndicator())
          : _results.isEmpty
              ? const Center(child: Text('Nenhum resultado encontrado'))
              : ListView.builder(
                  itemCount: _results.length,
                  itemBuilder: (context, index) {
                    final item = _results[index];
                    return ListTile(
                      leading: Icon(
                        item.matchType == 'semantic_image'
                            ? Icons.photo
                            : Icons.description,
                      ),
                      title: Text(item.fileName),
                      subtitle: Text(item.snippet),
                      trailing: Text(
                        item.matchType,
                        style: const TextStyle(fontSize: 11, color: Colors.grey),
                      ),
                    );
                  },
                ),
    );
  }
}
