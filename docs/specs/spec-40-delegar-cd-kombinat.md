# Especificação do Ticket #40: Delegar Entregas e Reset de Container do Mocambo ao Kombinat

## 1. Contexto e Motivação
O workflow `.forgejo/workflows/ci.yml` executava uploads diretos para R2, rotação manual na Banca, trigger direto na Pratilera e SSH direto no `obizero`. Com o Kombinat, a CI publica artefatos via `cccp package upload` nos Generic Packages e mantém apenas build/push da imagem Docker para a Gambi.
