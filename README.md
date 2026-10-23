# mocambo

Sovereign personal cloud storage, WebDAV engine, and smart offload

## 📋 Conformidade GOST

Este projeto adota a norma **[GOST](https://codeberg.org/lumenpink/gost)** (Gosplan Standard for Technical Software).
- **Versão GOST:** `1.5.0`
- **Linguagens declaradas:** `rust`
- **Licença:** `GPL-3.0`

### Compilação Estática e Otimização Soberana
- **Perfil de Release:** `opt-level = 3`, `lto = "fat"`, `codegen-units = 1`, `panic = "abort"` e `strip = true`.
- **Linking Estático musl:** Suporte a `target-feature=+crt-static` para distribuição `static-pie` universal.

## 🚀 Como Iniciar

```bash
# Clonar o repositório
git clone <url-do-repositorio>
cd mocambo

# Verificar conformidade
cccp check
```

## 📄 Licença

Distribuído sob a licença [GPL-3.0](./LICENSE).
