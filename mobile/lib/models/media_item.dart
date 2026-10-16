enum SyncStatus {
  pending,
  synced,
  offloaded,
}

class MediaItem {
  final String id;
  final String filePath;
  final String fileName;
  final int fileSize;
  final DateTime createdAt;
  final String sha256;
  SyncStatus syncStatus;

  MediaItem({
    required this.id,
    required this.filePath,
    required this.fileName,
    required this.fileSize,
    required this.createdAt,
    required this.sha256,
    this.syncStatus = SyncStatus.pending,
  });

  bool isEligibleForOffload({int retentionDays = 15, DateTime? referenceTime}) {
    final now = referenceTime ?? DateTime.now();
    final difference = now.difference(createdAt);
    return difference.inDays >= retentionDays;
  }

  Map<String, dynamic> toJson() => {
        'id': id,
        'filePath': filePath,
        'fileName': fileName,
        'fileSize': fileSize,
        'createdAt': createdAt.toIso8601String(),
        'sha256': sha256,
        'syncStatus': syncStatus.name,
      };

  factory MediaItem.fromJson(Map<String, dynamic> json) => MediaItem(
        id: json['id'] as String,
        filePath: json['filePath'] as String,
        fileName: json['fileName'] as String,
        fileSize: json['fileSize'] as int,
        createdAt: DateTime.parse(json['createdAt'] as String),
        sha256: json['sha256'] as String,
        syncStatus: SyncStatus.values.byName(
          json['syncStatus'] as String? ?? 'pending',
        ),
      );
}
