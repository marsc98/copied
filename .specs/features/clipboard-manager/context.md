# Clipboard Manager (copied) — Interview Decisions

**Date:** 2026-07-02
**Scope:** Aplicação Rust standalone rodando no Pop!_OS 24.04 (COSMIC/Wayland) que monitora a área de transferência do sistema, mantém pilha de até 15 itens, com TUI acionada por atalho de teclado para visualizar/copiar/excluir/pinar (até 5 pins).
**Source:** Entrevista informal (sem spec prévio, projeto greenfield)

---

## Decisões

### Captura de clipboard (Wayland/COSMIC)

- Pop!_OS 24.04 roda COSMIC, ambiente **Wayland-only** (XWayland só pra apps X11 legado, sem sessão X11 nativa).
- COSMIC vem com o protocolo `wlr-data-control` / `ext-data-control` **desabilitado por padrão** por segurança — é preciso setar `COSMIC_DATA_CONTROL_ENABLED=1` **antes do compositor iniciar** (`/etc/profile.d/`, requer sudo + reboot — não é config de unit systemd).
- **Correção pós-pesquisa técnica (fase /design):** `wl-clipboard-rs` **não tem e nunca teve** módulo `watch` (só existe como PR #83 aberta, não mergeada). A decisão original da entrevista ("usar wl-clipboard-rs pra escutar mudanças") estava errada.
- **Decisão atualizada:** watcher de clipboard implementado com cliente `wayland-client` próprio (protocolo `ext-data-control`/`wlr-data-control` de baixo nível), seguindo a referência do Ringboard (`SUPERCILEX/clipboard-history`). `wl-clipboard-rs` continua sendo usado, mas só pro módulo `copy` (escrever de volta no clipboard) — API madura e testada pra isso.
- **Rationale:** evita dependência de PR não mergeada ou fork de manutenção incerta; alinhado ao objetivo de aprender Rust de verdade (protocolo Wayland de baixo nível) sem depender de subprocess externo (`wl-paste --watch`).

### Arquitetura de processo

- **Daemon + cliente TUI via IPC** (unix socket).
- Daemon roda como systemd user service, sempre monitorando o clipboard e mantendo a pilha, independente da TUI estar aberta.
- Cliente TUI é um binário separado que conecta no socket, manda comandos (listar/copiar/excluir/pinar) e fecha.
- **Rationale:** padrão usado por clipboard managers reais (cliphist, clipmenu); só assim dá pra capturar tudo continuamente e abrir a view sob demanda.

### Ativação da TUI

- Atalho de teclado é cadastrado nas Configurações do COSMIC (Teclado > Atalhos personalizados), apontando pra um comando que abre uma janela de terminal nova rodando o cliente TUI (ex: `cosmic-term -e copied show`).
- Ao fechar (Esc) a janela de terminal some — sem depender de terminal já aberto.

### Persistência

- **JSON simples em disco** (`~/.local/share/copied/stack.json`), reescrito a cada mudança via `serde`.
- **Rationale:** SQLite seria overkill pra no máximo 20 registros (15 + 5 pins); JSON evita dependência extra e é proporcional ao volume de dados. Confirmado pelo usuário.
- Imagens capturadas **não** vão inline (base64) no JSON — salvas como arquivo em cache dir (`~/.cache/copied/images/`), JSON só referencia o path.

### Tipos de dado suportados

- **Texto + imagens** (PNG/JPEG). Texto mostra prévia direto na TUI; imagem mostra placeholder/thumbnail.
- Fora do escopo: arquivos (URIs) e HTML rico — não capturados.

### Comportamento da pilha

- Pilha tem **15 slots pra itens não-pinados**, comportamento LRU (mais antigo sai quando cheio).
- **Pins são um espaço à parte**, até 5, imunes a evicção — não contam nos 15.
- Copiar conteúdo idêntico a algo já existente na pilha **move a entrada pro topo** (dedup), não cria duplicata.

### UX / Keybindings da TUI

- Setas (↑/↓) pra navegar, Enter copia o item selecionado e fecha, Delete exclui, tecla dedicada pra pin/unpin, Esc fecha sem ação.

### Framework TUI

- **ratatui + crossterm**. Padrão de mercado atual pra TUIs em Rust (sucessor do tui-rs, usado por bottom, gitui, etc).

---

## Discricionariedade do Agente

- Layout visual exato da TUI (colunas, cores, indicador de pin, truncamento de prévia longa) — usar julgamento durante o design, mantendo consistência com ratatui idioms.
- Nome exato da tecla de pin/unpin e mapeamento de F-keys — decidir no design técnico.
- Estrutura interna do protocolo IPC (formato de mensagem no unix socket) — decidir no design técnico.

---

## Ideias Adiadas (fora do escopo)

- Sincronização em nuvem entre máquinas.
- GUI gráfica (fora do terminal).
- Suporte a arquivos/URIs e HTML rico no clipboard.

---

## Questões Abertas

- Risco conhecido: há relato de bug de "Clipboard Freeze" em algumas versões do COSMIC com apps via Wayland (GitHub pop-os/pop#3860) — vale testar cedo no ambiente real antes de investir na arquitetura completa.
