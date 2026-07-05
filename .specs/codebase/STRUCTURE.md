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
│   │   └── src/lib.rs            # Command/Response/ItemView/Category + socket_path() (protocolo IPC)
│   ├── copied-daemon/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs           # bootstrap: threads, xdg paths, loop principal, auto-detecção de categoria
│   │       ├── stack.rs          # domínio puro: LRU 15 + pins 5 + dedup + category
│   │       ├── categorize.rs     # heurística pura: detect(text) -> Category
│   │       ├── persistence.rs    # stack.json + cache de imagens em disco
│   │       ├── ipc.rs            # UnixListener, DaemonState, dispatch de Command
│   │       ├── watcher.rs        # cliente Wayland wlr-data-control (leitura)
│   │       └── clipboard_write.rs # escrita no clipboard via wl-clipboard-rs
│   └── copied-gui/
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs            # bootstrap: instance_lock, LayerShellSettings, iced_layershell::application(...).run()
│           ├── app.rs             # AppState, Message, update/view/subscription (iced, modelo Elm)
│           ├── ipc_worker.rs      # thread nativa + canal, ponte entre IpcClient bloqueante e Subscription do iced
│           ├── ipc_client.rs      # cliente do socket unix (send/recv NDJSON) — movido de copied-cli
│           ├── instance_lock.rs   # toggle de instância única (lock file + sinal Unix)
│           └── symbols.rs         # catálogo estático de símbolos (dado puro, sem I/O)
├── deploy/
│   ├── copied-daemon.service      # unit systemd --user
│   └── install.sh                 # build + instala unit + instruções
├── target/                        # build artifacts (gitignored)
└── .specs/                        # docs deste workflow (SDD)
    ├── codebase/                  # este diretório (brownfield mapping)
    └── features/                  # clipboard-manager/ e copied-gui/ (spec.md, context.md, design.md, tasks.md)
```

**Nota:** repositório git local (inicializado durante a feature `copied-gui`, 2026-07-04). Sem remote configurado.

## Module Organization

### `copied-core` — Protocolo compartilhado

**Purpose:** Tipos de domínio serializáveis compartilhados entre daemon e cliente; única fonte de verdade do contrato IPC; também concentra `socket_path()`.
**Location:** `crates/copied-core/src/lib.rs` (arquivo único)
**Key items:** `Command`, `Response`, `ItemView`, `ItemKindView`, `ItemId`, `Category`, `socket_path()`

### `copied-daemon` — Processo background

**Purpose:** Captura clipboard, categoriza automaticamente, mantém pilha em memória, persiste em disco, serve IPC.
**Location:** `crates/copied-daemon/src/` (7 arquivos)
**Key files:** `stack.rs` (maior arquivo do projeto — domínio), `watcher.rs` (integração Wayland mais complexa), `ipc.rs` (orquestração, agora com testes de dispatch), `categorize.rs` (heurística nova, isolada)

### `copied-gui` — Cliente gráfico (popup layer-shell)

**Purpose:** Janela overlay sem decoração (via `iced_layershell`) invocada sob demanda (atalho de teclado, sem terminal) pra visualizar/manipular a pilha, com busca, preview de imagem, categorização e aba de símbolos matemáticos/ícones. Substitui o antigo `copied-cli` (TUI).
**Location:** `crates/copied-gui/src/` (6 arquivos)
**Key files:** `app.rs` (maior arquivo — estado + update + view + subscription), `ipc_worker.rs` (ponte thread+canal pro modelo reativo do iced), `instance_lock.rs` (toggle de instância única)

## Where Things Live

**Captura de clipboard (leitura):**
- Integração Wayland: `crates/copied-daemon/src/watcher.rs`
- Lógica de negócio (LRU/dedup/pins/categoria): `crates/copied-daemon/src/stack.rs`
- Heurística de categoria: `crates/copied-daemon/src/categorize.rs`
- Persistência: `crates/copied-daemon/src/persistence.rs`

**Escrita de volta no clipboard:**
- Via daemon (item da pilha): `crates/copied-daemon/src/clipboard_write.rs` (chamado por `ipc.rs::copy_to_clipboard`)
- Direto do cliente (símbolo): `crates/copied-gui/src/app.rs` (`wl-clipboard-rs`, sem passar pelo daemon)

**Protocolo IPC (contrato):**
- Definição + `socket_path()`: `crates/copied-core/src/lib.rs`
- Servidor: `crates/copied-daemon/src/ipc.rs`
- Cliente: `crates/copied-gui/src/ipc_client.rs`

**Interface do usuário:**
- `crates/copied-gui/src/app.rs` (iced — update/view/subscription)
- Catálogo de símbolos: `crates/copied-gui/src/symbols.rs`

**Instância única / toggle:**
- `crates/copied-gui/src/instance_lock.rs` (lock file `$XDG_RUNTIME_DIR/copied-gui.pid` + `SIGUSR1`)

**Configuração/Deploy:**
- Unit systemd: `deploy/copied-daemon.service`
- Script de instalação: `deploy/install.sh`
- Paths XDG (runtime): resolvidos em código, não config file — `copied-daemon/src/main.rs::xdg_dir` e `copied-core::socket_path`
