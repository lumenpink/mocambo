import 'package:flutter/material.dart';
import '../config/app_config.dart';

class SettingsPage extends StatelessWidget {
  final AppConfig config;

  const SettingsPage({super.key, required this.config});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Configurações')),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          ListTile(
            title: const Text('Servidor Mocambo'),
            subtitle: Text(config.serverUrl),
            leading: const Icon(Icons.cloud),
          ),
          ListTile(
            title: const Text('Janela de Retenção'),
            subtitle: Text('${config.retentionDays} dias antes do offload'),
            leading: const Icon(Icons.schedule),
          ),
          ListTile(
            title: const Text('Usuário'),
            subtitle: Text(config.username),
            leading: const Icon(Icons.person),
          ),
        ],
      ),
    );
  }
}
