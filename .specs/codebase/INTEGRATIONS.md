# External Integrations

**Analyzed:** 2026-07-04

## Wayland Compositor (clipboard read/write)

**Service:** Wayland compositor local (COSMIC/`cosmic-comp` no Pop!_OS 24.04), via protocolo `zwlr_data_control_manager_v1` (`wlr-data-control-unstable-v1`).
**Purpose:** Detectar mudanças de seleção do clipboard do sistema (leitura) e escrever de volta um item da pilha (escrita).
**Implementation:**
- Leitura: `crates/copied-daemon/src/watcher.rs` — implementação própria do protocolo via `wayland-client` 0.31 + `wayland-protocols-wlr` 0.3. Conecta via `Connection::connect_to_env()`, faz bind de `ZwlrDataControlManagerV1` e `WlSeat`, negocia MIME type (`TEXT_MIME_TYPES`/`IMAGE_MIME_TYPES`), lê conteúdo via pipe (`offer.receive(mime, fd)` + `std::io::pipe()`).
- Escrita: `crates/copied-daemon/src/clipboard_write.rs` — usa `wl-clipboard-rs` 0.9 (módulo `copy`), não o protocolo raw.
**Configuration:** Requer variável de ambiente `COSMIC_DATA_CONTROL_ENABLED=1` setada no processo do compositor (não no daemon) — sem ela, `cosmic-comp` não advertise o protocolo e o watcher falha ao conectar na primeira tentativa (`watcher.rs::run`, exit(1) com mensagem explicando a flag). Esse setup é feito pelo `deploy/install.sh` via `/etc/profile.d/copied-clipboard.sh` (requer reboot).
**Authentication:** N/A (protocolo local via socket Wayland do usuário, sem credenciais).
**Reconnection:** backoff incremental (1s, 2s, 5s, 30s — `RECONNECT_BACKOFFS_SECS`) após a primeira conexão bem-sucedida; falha na primeira tentativa é fatal (exit 1), falhas subsequentes retry indefinidamente.

## systemd (user service — autostart/supervisão)

**Service:** `systemd --user`
**Purpose:** Iniciar o daemon automaticamente no login e reiniciá-lo em caso de crash.
**Implementation:** Unit file em `deploy/copied-daemon.service` — `Type=simple`, `Restart=on-failure`, `RestartSec=1`, `WantedBy=graphical-session.target`, `ExecStart=%h/.cargo/bin/copied-daemon`.
**Configuration:** Instalada e habilitada por `deploy/install.sh` (copia pra `~/.config/systemd/user/`, roda `systemctl --user enable --now`).
**Authentication:** N/A (serviço de sessão do próprio usuário, sem privilégio elevado — só o passo de `/etc/profile.d/` no install script pede sudo).

## Filesystem (XDG base dirs — não é um serviço externo, mas é integração de sistema)

**Purpose:** Persistência da pilha e cache de imagens, socket IPC.
**Locations:**
- `$XDG_DATA_HOME/copied/stack.json` (fallback `~/.local/share/copied/stack.json`) — resolvido em `copied-daemon/src/main.rs::stack_path`
- `$XDG_CACHE_HOME/copied/images/` (fallback `~/.cache/copied/images/`) — `main.rs::image_cache_dir`
- `$XDG_RUNTIME_DIR/copied.sock` — `copied_core::socket_path()`, única definição usada por `copied-daemon` (re-exportada em `ipc.rs`) e `copied-gui/src/ipc_client.rs` (duplicação resolvida, ver CONCERNS.md)

## Unix Domain Socket (IPC interno)

**Purpose:** Protocolo entre `copied-daemon` e `copied` (cliente gráfico `copied-gui`) — não é integração externa, mas é o único canal de comunicação entre os dois binários do produto.
**Implementation:** NDJSON sobre `UnixListener`/`UnixStream`, servidor em `copied-daemon/src/ipc.rs`, cliente em `copied-gui/src/ipc_client.rs`. Contrato de mensagens definido em `copied-core/src/lib.rs`.

## Background Jobs

**Queue system:** Nenhum — não há fila de jobs. Persistência é síncrona e imediata a cada mutação (`state.persist()` chamado inline, sem thread separada de I/O).

## APIs Externas / Webhooks

Nenhum. Produto roda inteiramente local, sem chamadas de rede.
