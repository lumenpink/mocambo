class SearchResultItem {
  final int id;
  final String storagePath;
  final String fileName;
  final int fileSize;
  final String sha256;
  final String mimeType;
  final String snippet;
  final double score;
  final String matchType;

  const SearchResultItem({
    required this.id,
    required this.storagePath,
    required this.fileName,
    required this.fileSize,
    required this.sha256,
    required this.mimeType,
    required this.snippet,
    required this.score,
    required this.matchType,
  });

  factory SearchResultItem.fromJson(Map<String, dynamic> json) =>
      SearchResultItem(
        id: json['id'] as int,
        storagePath: json['storage_path'] as String,
        fileName: json['file_name'] as String,
        fileSize: json['file_size'] as int,
        sha256: json['sha256'] as String,
        mimeType: json['mime_type'] as String,
        snippet: json['snippet'] as String? ?? '',
        score: (json['score'] as num?)?.toDouble() ?? 0.0,
        matchType: json['match_type'] as String? ?? 'text',
      );
}
