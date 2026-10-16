import 'package:flutter_test/flutter_test.dart';
import 'package:mocambo_mobile/config/app_config.dart';
import 'package:mocambo_mobile/models/media_item.dart';
import 'package:mocambo_mobile/services/api_client.dart';
import 'package:mocambo_mobile/services/offload_manager.dart';

class MockApiClient extends ApiClient {
  final Set<String> existingHashes;

  MockApiClient({required this.existingHashes})
      : super(config: const AppConfig(serverUrl: 'http://mock:8080'));

  @override
  Future<bool> checkFileExists(String sha256) async {
    return existingHashes.contains(sha256);
  }
}

void main() {
  group('OffloadManager 15-day smart retention', () {
    const config = AppConfig(
      serverUrl: 'http://mock:8080',
      retentionDays: 15,
    );

    test('mídia com menos de 15 dias não deve ser expurgada', () async {
      final now = DateTime(2026, 10, 16, 12, 0);
      final recentItem = MediaItem(
        id: '1',
        filePath: '/tmp/recent.jpg',
        fileName: 'recent.jpg',
        fileSize: 1024,
        createdAt: now.subtract(const Duration(days: 10)),
        sha256: 'hash_recent',
      );

      final mockApi = MockApiClient(existingHashes: {'hash_recent'});
      final manager = OffloadManager(config: config, apiClient: mockApi);

      bool fileDeleted = false;
      final result = await manager.processOffload(
        [recentItem],
        referenceTime: now,
        fileDeleter: (_) {
          fileDeleted = true;
          return true;
        },
      );

      expect(result.eligibleCount, 0);
      expect(result.offloadedCount, 0);
      expect(result.skippedCount, 1);
      expect(fileDeleted, false);
      expect(recentItem.syncStatus, SyncStatus.pending);
    });

    test('mídia com 15 dias ou mais DEVE ser expurgada se existir no servidor', () async {
      final now = DateTime(2026, 10, 16, 12, 0);
      final oldItem = MediaItem(
        id: '2',
        filePath: '/tmp/old.jpg',
        fileName: 'old.jpg',
        fileSize: 2048,
        createdAt: now.subtract(const Duration(days: 16)),
        sha256: 'hash_old_synced',
      );

      final mockApi = MockApiClient(existingHashes: {'hash_old_synced'});
      final manager = OffloadManager(config: config, apiClient: mockApi);

      bool fileDeleted = false;
      final result = await manager.processOffload(
        [oldItem],
        referenceTime: now,
        fileDeleter: (_) {
          fileDeleted = true;
          return true;
        },
      );

      expect(result.eligibleCount, 1);
      expect(result.offloadedCount, 1);
      expect(result.skippedCount, 0);
      expect(fileDeleted, true);
      expect(oldItem.syncStatus, SyncStatus.offloaded);
    });

    test('mídia com 15 dias ou mais NÃO deve ser expurgada se não existir no servidor', () async {
      final now = DateTime(2026, 10, 16, 12, 0);
      final unconfirmedItem = MediaItem(
        id: '3',
        filePath: '/tmp/not_synced.jpg',
        fileName: 'not_synced.jpg',
        fileSize: 2048,
        createdAt: now.subtract(const Duration(days: 20)),
        sha256: 'hash_not_on_server',
      );

      final mockApi = MockApiClient(existingHashes: {});
      final manager = OffloadManager(config: config, apiClient: mockApi);

      bool fileDeleted = false;
      final result = await manager.processOffload(
        [unconfirmedItem],
        referenceTime: now,
        fileDeleter: (_) {
          fileDeleted = true;
          return true;
        },
      );

      expect(result.eligibleCount, 1);
      expect(result.offloadedCount, 0);
      expect(result.skippedCount, 1);
      expect(fileDeleted, false);
      expect(unconfirmedItem.syncStatus, SyncStatus.pending);
    });
  });
}
