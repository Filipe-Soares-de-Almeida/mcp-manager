# Changelog

Tradução de CHANGELOG.md. Cada versão tem as mesmas seções do original, com os títulos dos grupos
em inglês (Added, Changed, Deprecated, Removed, Fixed, Security): o app os traduz ao mostrar. Cada
entrada começa com o verbo do seu grupo: Adiciona, Altera, Descontinua, Remove, Corrige, Protege.

## [Unreleased]

## [0.1.0] - 2026-11-24

A primeira versão: os servidores MCP do Claude Desktop numa janela, e pelo chat do Claude.

### Added

- Adiciona uma janela para adicionar, editar, renomear, remover, desativar e ativar os servidores MCP do Claude Desktop.
- Adiciona uma fila que segura as alterações enquanto o Claude roda e as grava quando ele fecha.
- Adiciona o botão Reiniciar Claude, que fecha o Claude, aplica a fila e o abre de novo.
- Adiciona a desativação, que guarda o servidor no banco do app e o devolve igual.
- Adiciona um backup da config antes de cada gravação, e a tela de Backups que restaura qualquer cópia.
- Adiciona a busca da config pelo executável do Claude, no pacote MSIX e no instalador antigo.
- Adiciona um servidor MCP que deixa o Claude listar, adicionar, editar, remover, desativar e ativar servidores.
- Adiciona o preenchimento do editor com o JSON de um servidor colado de um README.
- Adiciona o ícone na bandeja, o início com o Windows e o atalho no Menu Iniciar.
- Adiciona o cartão de primeiros passos para o primeiro uso.
- Adiciona a interface em inglês e português do Brasil.
- Adiciona a busca de atualizações, que lista as versões ainda não instaladas.
- Adiciona a tela de Changelog com as notas de todas as versões.
- Adiciona a atualização pelo próprio app, que baixa a versão nova, confere a assinatura e a instala.
