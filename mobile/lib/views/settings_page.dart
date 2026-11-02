import 'package:flutter/material.dart';
import 'package:http/http.dart' as http;
import '../config/app_config.dart';

class SettingsPage extends StatefulWidget {
  final AppConfig config;
  final ValueChanged<AppConfig>? onConfigSaved;

  const SettingsPage({
    super.key,
    required this.config,
    this.onConfigSaved,
  });

  @override
  State<SettingsPage> createState() => _SettingsPageState();
}

class _SettingsPageState extends State<SettingsPage> {
  final _formKey = GlobalKey<FormState>();
  late final TextEditingController _serverController;
  late final TextEditingController _usernameController;
  late final TextEditingController _passwordController;
  late final TextEditingController _retentionController;

  bool _isTesting = false;
  String? _testMessage;
  bool _testSuccess = false;

  @override
  void initState() {
    super.initState();
    _serverController = TextEditingController(text: widget.config.serverUrl);
    _usernameController = TextEditingController(text: widget.config.username);
    _passwordController = TextEditingController(text: widget.config.password);
    _retentionController =
        TextEditingController(text: widget.config.retentionDays.toString());
  }

  @override
  void dispose() {
    _serverController.dispose();
    _usernameController.dispose();
    _passwordController.dispose();
    _retentionController.dispose();
    super.dispose();
  }

  Future<void> _testConnection() async {
    final serverUrl = _serverController.text.trim();
    if (serverUrl.isEmpty) {
      setState(() {
        _testSuccess = false;
        _testMessage = 'Por favor, informe a URL do servidor para testar.';
      });
      return;
    }

    setState(() {
      _isTesting = true;
      _testMessage = null;
    });

    try {
      final cleanUrl = serverUrl.replaceAll(RegExp(r'/+$'), '');
      final healthUri = Uri.parse('$cleanUrl/health');
      final response = await http.get(healthUri).timeout(
            const Duration(seconds: 5),
          );

      if (response.statusCode >= 200 && response.statusCode < 300) {
        setState(() {
          _testSuccess = true;
          _testMessage = 'Conexão estabelecida com sucesso!';
        });
      } else {
        setState(() {
          _testSuccess = false;
          _testMessage =
              'Servidor respondeu com código de erro: ${response.statusCode}';
        });
      }
    } catch (e) {
      setState(() {
        _testSuccess = false;
        _testMessage = 'Falha ao conectar: $e';
      });
    } finally {
      setState(() {
        _isTesting = false;
      });
    }
  }

  void _saveSettings() {
    if (_formKey.currentState?.validate() ?? false) {
      final retention = int.tryParse(_retentionController.text.trim()) ?? 15;
      final newConfig = AppConfig(
        serverUrl: _serverController.text.trim(),
        username: _usernameController.text.trim(),
        password: _passwordController.text.trim(),
        retentionDays: retention,
      );

      widget.onConfigSaved?.call(newConfig);

      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(
          content: Text('Configurações salvas com sucesso!'),
          backgroundColor: Colors.green,
        ),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Configurações')),
      body: SingleChildScrollView(
        padding: const EdgeInsets.all(16),
        child: Form(
          key: _formKey,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              if (!widget.config.isConfigured)
                Container(
                  margin: const EdgeInsets.only(bottom: 16),
                  padding: const EdgeInsets.all(12),
                  decoration: BoxDecoration(
                    color: Colors.amber.shade100,
                    borderRadius: BorderRadius.circular(8),
                    border: Border.all(color: Colors.amber.shade400),
                  ),
                  child: Row(
                    children: [
                      Icon(Icons.info_outline, color: Colors.amber.shade900),
                      const SizedBox(width: 8),
                      const Expanded(
                        child: Text(
                          'Servidor não configurado. Por favor, informe a URL e o usuário para começar a sincronizar.',
                          style: TextStyle(fontSize: 13),
                        ),
                      ),
                    ],
                  ),
                ),
              TextFormField(
                controller: _serverController,
                decoration: const InputDecoration(
                  labelText: 'URL do Servidor',
                  hintText: 'https://mocambo-server.2lp.in',
                  prefixIcon: Icon(Icons.cloud_outlined),
                  border: OutlineInputBorder(),
                ),
                keyboardType: TextInputType.url,
                validator: (val) {
                  if (val == null || val.trim().isEmpty) {
                    return 'Informe a URL do servidor';
                  }
                  return null;
                },
              ),
              const SizedBox(height: 16),
              TextFormField(
                controller: _usernameController,
                decoration: const InputDecoration(
                  labelText: 'Usuário',
                  hintText: 'nome.usuario',
                  prefixIcon: Icon(Icons.person_outline),
                  border: OutlineInputBorder(),
                ),
                validator: (val) {
                  if (val == null || val.trim().isEmpty) {
                    return 'Informe o usuário';
                  }
                  return null;
                },
              ),
              const SizedBox(height: 16),
              TextFormField(
                controller: _passwordController,
                decoration: const InputDecoration(
                  labelText: 'Senha / Token de Acesso',
                  prefixIcon: Icon(Icons.lock_outline),
                  border: OutlineInputBorder(),
                ),
                obscureText: true,
              ),
              const SizedBox(height: 16),
              TextFormField(
                controller: _retentionController,
                decoration: const InputDecoration(
                  labelText: 'Janela de Retenção (dias)',
                  prefixIcon: Icon(Icons.schedule_outlined),
                  border: OutlineInputBorder(),
                  helperText: 'Dias antes de liberar o arquivo local (offload)',
                ),
                keyboardType: TextInputType.number,
                validator: (val) {
                  final n = int.tryParse(val ?? '');
                  if (n == null || n <= 0) {
                    return 'Informe um número positivo de dias';
                  }
                  return null;
                },
              ),
              const SizedBox(height: 20),
              if (_testMessage != null)
                Container(
                  margin: const EdgeInsets.only(bottom: 16),
                  padding: const EdgeInsets.all(12),
                  decoration: BoxDecoration(
                    color: _testSuccess ? Colors.green.shade50 : Colors.red.shade50,
                    borderRadius: BorderRadius.circular(8),
                    border: Border.all(
                      color: _testSuccess ? Colors.green.shade400 : Colors.red.shade400,
                    ),
                  ),
                  child: Row(
                    children: [
                      Icon(
                        _testSuccess ? Icons.check_circle : Icons.error_outline,
                        color: _testSuccess ? Colors.green : Colors.red,
                      ),
                      const SizedBox(width: 8),
                      Expanded(
                        child: Text(
                          _testMessage!,
                          style: TextStyle(
                            color: _testSuccess
                                ? Colors.green.shade900
                                : Colors.red.shade900,
                            fontSize: 13,
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
              OutlinedButton.icon(
                onPressed: _isTesting ? null : _testConnection,
                icon: _isTesting
                    ? const SizedBox(
                        width: 16,
                        height: 16,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      )
                    : const Icon(Icons.network_check),
                label: const Text('Testar Conexão'),
              ),
              const SizedBox(height: 12),
              FilledButton.icon(
                onPressed: _saveSettings,
                icon: const Icon(Icons.save),
                label: const Text('Salvar'),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
