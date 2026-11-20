# mocambo

Armazenamento em nuvem pessoal, motor WebDAV e descarregamento inteligente

## 📋 Conformidade GOST

Este projeto adota a norma técnica **[GOST](.gost/GOST.md)** v3.0.0 (Perfil Kollontai 2.1.0 / Lilya 2.1.0).
- **Linguagens declaradas:** `rust`
- **Licença:** `GPL-3.0-or-later`

### Compilação Estática e Otimização
- **Perfil de Release:** `opt-level = 3`, `lto = "fat"`, `codegen-units = 1`, `panic = "abort"` e `strip = true`.
- **Linking Estático musl:** Suporte a `target-feature=+crt-static` para distribuição `static-pie` universal.

## 🚀 Como Iniciar

```bash
# Clonar o repositório
git clone <url-do-repositorio>
cd mocambo

# Verificar conformidade
cccp check --strict
```

## 📄 Licença & Matriz Modular

Em conformidade com a norma técnica [GOST v3.0.0](.gost/GOST.md):

* **Código-Fonte e Binário CLI (`src/`):** Licenciados sob a [GPL-3.0-or-later](./LICENSE).
* **Identidade Visual e Emblema:** Licenciados sob a [Creative Commons Atribuição-SemDerivações 4.0 Internacional (CC-BY-ND 4.0)](LICENSE-LOGO).

---

Feito com amor travesti por [Lumen Pink](https://lumen.pink).
