# Project Structure

**Root:** `copied/` (nome do diretório do repositório)

## Directory Tree

```
copied/
├── Cargo.toml                    # workspace root (3 members)
├── Cargo.lock
├── rust-toolchain.toml           # channel = "stable"
├── .gitignore                    # /target
├── crates/
│   ├── copied-core/
│   │   ├── Cargo.toml
│   │   └── src/lib.rs            # Command/Response/ItemView (protocolo IPC)
│   ├── copied-daemon/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs           # bootstrap: threads, xdg paths, loop principal
│   │       ├── stack.rs          # domínio puro: LRU 15 + pins 5 + dedup
│   │       ├── persistence.rs    # stack.json + cache de imagens em disco
│   │       ├── ipc.rs            # UnixListener, DaemonState, dispatch de Command
│   │       ├── watcher.rs        # cliente Wayland wlr-data-control (leitura)
│   │       └── clipboard_write.rs # escrita no clipboard via wl-clipboard-rs
│   └── copied-cli/
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs            # bootstrap: conecta IPC, inicia ratatui
│           ├── app.rs             # estado da TUI, event loop, render
│           └── ipc_client.rs      # cliente do socket unix (send/recv NDJSON)
├── deploy/
│   ├── copied-daemon.service      # unit systemd --user
│   └── install.sh                 # build + instala unit + instruções
├── target/                        # build artifacts (gitignored)
└── .specs/                        # docs deste workflow (SDD)
    ├── codebase/                  # este diretório (brownfield mapping)
    └── features/clipboard-manager/ # spec.md, context.md, design.md, tasks.md
```

**Nota:** não é repositório git (`git status` confirma: "not a git repository"). Diretório existe apenas localmente.

## Module Organization

### `copied-core` — Protocolo compartilhado

**Purpose:** Tipos de domínio serializáveis compartilhados entre daemon e cliente; única fonte de verdade do contrato IPC.
**Location:** `crates/copied-core/src/lib.rs` (114 linhas, arquivo único)
**Key items:** `Command`, `Response`, `ItemView`, `ItemKindView`, `ItemId`

### `copied-daemon` — Processo background

**Purpose:** Captura clipboard, mantém pilha em memória, persiste em disco, serve IPC.
**Location:** `crates/copied-daemon/src/` (6 arquivos, ~1000 linhas)
**Key files:** `stack.rs` (382 linhas, maior arquivo do projeto — domínio), `watcher.rs` (305 linhas — integração Wayland mais complexa), `ipc.rs` (217 linhas — orquestração)

### `copied-cli` — Cliente TUI

**Purpose:** Interface de terminal invocada sob demanda (atalho de teclado) pra visualizar/manipular a pilha.
**Location:** `crates/copied-cli/src/` (3 arquivos, ~314 linhas)
**Key files:** `app.rs` (193 linhas — estado + render + input), `ipc_client.rs` (98 linhas — protocolo de fio)

## Where Things Live

**Captura de clipboard (leitura):**
- Integração Wayland: `crates/copied-daemon/src/watcher.rs`
- Lógica de negócio (LRU/dedup/pins): `crates/copied-daemon/src/stack.rs`
- Persistência: `crates/copied-daemon/src/persistence.rs`

**Escrita de volta no clipboard:**
- `crates/copied-daemon/src/clipboard_write.rs` (chamado por `ipc.rs::copy_to_clipboard`)

**Protocolo IPC (contrato):**
- Definição: `crates/copied-core/src/lib.rs`
- Servidor: `crates/copied-daemon/src/ipc.rs`
- Cliente: `crates/copied-cli/src/ipc_client.rs`

**Interface do usuário:**
- `crates/copied-cli/src/app.rs` (ratatui — draw + handle_events)

**Configuração/Deploy:**
- Unit systemd: `deploy/copied-daemon.service`
- Script de instalação: `deploy/install.sh`
- Paths XDG (runtime): resolvidos em código, não config file — `copied-daemon/src/main.rs::xdg_dir` e `ipc.rs::socket_path`
