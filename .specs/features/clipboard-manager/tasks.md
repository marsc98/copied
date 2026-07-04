# Clipboard Manager (copied) Tasks

**Design**: `.specs/features/clipboard-manager/design.md`
**Testing**: `.specs/codebase/TESTING.md`
**Status**: Draft

**Tools por task**: nenhum MCP de documentação disponível nesta sessão (sem Context7). Toda task usa skill `/find-docs` pra resolver dúvidas de API antes de codar, com `WebSearch`/`WebFetch` (ex: ler source no GitHub) como fallback. Aplica-se especialmente a T6 (watcher Wayland), T7 (wl-clipboard-rs), T11 (ratatui) — libs onde a API exata importa.

---

## Execution Plan

### Phase 1: Foundation (Sequential)
```
T1
```

### Phase 2: Núcleo independente (Parallel OK)
```
T1 ──┬→ T2 (core types)
     ├→ T3 (stack logic)
     ├→ T6 (watcher Wayland)
     ├→ T7 (clipboard_write)
     └→ T9 (systemd unit + install docs)
```

### Phase 3: Depende do núcleo (Parallel OK)
```
T3 ──→ T4 (persistence)
T2,T3 ──→ T5 (ipc server)
T2 ──→ T10 (ipc_client CLI)
```

### Phase 4: Integração (Parallel OK)
```
T3,T4,T5,T6,T7 ──→ T8 (main.rs wiring do daemon)
T10 ──────────────→ T11 (TUI App + render + keys)
```

### Phase 5: Verificação end-to-end (Sequential)
```
T8,T9,T11 ──→ T12 (verificação manual completa)
```

---

## Task Breakdown

### T1: Cargo workspace setup

**What**: Cria workspace Cargo com 3 crates (`copied-core`, `copied-daemon`, `copied-cli`), cada `Cargo.toml` já com as dependências corretas declaradas (conforme tabela de crates do design.md), mais `rust-toolchain.toml` e `.gitignore`.
**Where**: `Cargo.toml` (workspace root), `crates/copied-core/Cargo.toml`, `crates/copied-daemon/Cargo.toml`, `crates/copied-cli/Cargo.toml`
**Depends on**: None
**Reuses**: N/A (fundação)
**Requirement**: N/A (infra)

**Done when**:
- [ ] `cargo build --workspace` compila os 3 crates vazios sem erro
- [ ] `copied-core/Cargo.toml`: `serde` (derive), `serde_json`
- [ ] `copied-daemon/Cargo.toml`: `serde`, `serde_json`, `sha2`, `wl-clipboard-rs`, `wayland-client`, `copied-core` (path dep); `tempfile` como dev-dependency
- [ ] `copied-cli/Cargo.toml`: `ratatui`, `crossterm`, `copied-core` (path dep)
- [ ] Gate check passa: `cargo build --workspace`

**Tests**: none (infra)
**Gate**: quick (`cargo build --workspace`)

---

### T2: `copied-core` — tipos de domínio e protocolo IPC [P]

**What**: Define `Command`, `Response`, `ItemView`, `ItemId`, `ItemKindView` com `serde::{Serialize, Deserialize}`, conforme contrato do design.md.
**Where**: `crates/copied-core/src/lib.rs`
**Depends on**: T1
**Reuses**: N/A
**Requirement**: CLIP-03, CLIP-04, CLIP-05, CLIP-06 (contrato usado por todas as ações da TUI)

**Done when**:
- [ ] `enum Command { List, CopyToClipboard { id: ItemId }, Delete { id: ItemId }, Pin { id: ItemId }, Unpin { id: ItemId } }`
- [ ] `enum Response { Items(Vec<ItemView>), Ack, Error { message: String } }`
- [ ] `struct ItemView { id: ItemId, kind: ItemKindView, pinned: bool, copied_at: SystemTime }`
- [ ] Unit test: round-trip serialize/deserialize de cada variante de `Command` e `Response` via `serde_json`
- [ ] Gate check passa: `cargo test -p copied-core && cargo clippy -p copied-core -- -D warnings`

**Tests**: unit
**Gate**: quick

---

### T3: `copied-daemon::stack` — lógica da pilha (LRU + pins + dedup) [P]

**What**: Implementa `Item`, `ItemKind`, `Stack` com `push`/`delete`/`pin`/`unpin`, LRU de 15 não-pinados, 5 pins separados, dedup por hash movendo pro topo.
**Where**: `crates/copied-daemon/src/stack.rs`
**Depends on**: T1
**Reuses**: N/A (lógica pura, sem I/O)
**Requirement**: CLIP-01, CLIP-05, CLIP-06

**Done when**:
- [ ] `struct Stack { items: VecDeque<Item>, pins: Vec<Item> }`
- [ ] `push` insere no topo; se hash já existe (pinado ou não), move pro topo sem duplicar
- [ ] `push` evicta o item não-pinado mais antigo quando `items.len() > 15`
- [ ] `pin` move item de `items` pra `pins`; retorna `Err(PinError::LimitReached)` se `pins.len() == 5`
- [ ] `unpin` move item de volta pra `items` (topo), aplicando eviction se necessário
- [ ] `delete` remove de `items` ou `pins`, o que existir
- [ ] Unit tests cobrindo: eviction no 16º item, dedup move-to-top, rejeição do 6º pin, delete de item pinado e não-pinado
- [ ] Gate check passa: `cargo test -p copied-daemon stack:: && cargo clippy -p copied-daemon -- -D warnings`

**Tests**: unit
**Gate**: quick

---

### T4: `copied-daemon::persistence` — JSON + cache de imagens

**What**: `load`/`save` de `Stack` em `stack.json` via `serde_json`, e `save_image`/`delete_image` no cache dir.
**Where**: `crates/copied-daemon/src/persistence.rs`
**Depends on**: T3
**Reuses**: `Item`/`Stack` de T3; hash de conteúdo já calculado pelo `stack` reaproveitado como nome de arquivo de imagem
**Requirement**: CLIP-02, CLIP-07

**Done when**:
- [ ] `load(path) -> Stack` retorna `Stack::default()` (vazio) e loga erro se arquivo ausente ou corrompido, sem panic
- [ ] `save(path, &Stack) -> io::Result<()>` grava JSON completo
- [ ] `save_image(cache_dir, bytes, mime) -> io::Result<PathBuf>` nomeia arquivo pelo hash do conteúdo
- [ ] `delete_image(path) -> io::Result<()>`
- [ ] Unit tests usando `tempfile::TempDir` (sem tocar `~/.local/share` real): save→load round-trip, load de arquivo corrompido retorna vazio, save_image + delete_image
- [ ] Gate check passa: `cargo test -p copied-daemon persistence:: && cargo clippy -p copied-daemon -- -D warnings`

**Tests**: unit
**Gate**: quick

---

### T5: `copied-daemon::ipc` — servidor unix socket

**What**: `UnixListener` em `$XDG_RUNTIME_DIR/copied.sock`, decodifica `Command` (NDJSON), aplica em `Arc<Mutex<Stack>>`, responde `Response`. Inclui conversão `Item -> ItemView`.
**Where**: `crates/copied-daemon/src/ipc.rs`
**Depends on**: T2, T3
**Reuses**: `copied_core::{Command, Response, ItemView}`, `Stack` de T3
**Requirement**: CLIP-03, CLIP-04, CLIP-05, CLIP-06

**Done when**:
- [ ] `serve(socket_path, state: Arc<Mutex<DaemonState>>) -> io::Result<()>` aceita conexões em loop, uma thread por conexão
- [ ] Cada linha NDJSON recebida vira `Command`, aplicada no estado, resposta serializada de volta
- [ ] `List` retorna pins primeiro (até 5) seguido de não-pinados (até 15), mais recente primeiro
- [ ] `Pin` no limite retorna `Response::Error { message: "Máximo de 5 pins atingido..." }`
- [ ] Socket antigo é removido antes de bind se já existir (`std::fs::remove_file` best-effort)
- [ ] Gate check passa: `cargo clippy -p copied-daemon -- -D warnings` (teste real de socket é manual, ver T12)

**Tests**: none (verificação manual em T12 — socket real é integração, não unit)
**Gate**: quick (clippy apenas)

---

### T6: `copied-daemon::watcher` — cliente Wayland ext-data-control [P]

**What**: Cliente `wayland-client` próprio que escuta mudanças de seleção do clipboard via protocolo `ext-data-control`/`wlr-data-control` e emite `ClipboardChange` (Text ou Image) por `mpsc::Sender`.
**Where**: `crates/copied-daemon/src/watcher.rs`
**Depends on**: T1
**Reuses**: Referência de protocolo do repositório `SUPERCILEX/clipboard-history` (crate `wayland/`) — estudar, não copiar
**Requirement**: CLIP-01, CLIP-07

**Done when**:
- [ ] Resolvida a Open Question do design: crate exata de bindings do protocolo (`wayland-protocols-wlr`, `wayland-protocols-misc`, ou geração via `wayland-scanner` em `build.rs`) — adicionar ao `Cargo.toml` do `copied-daemon`
- [ ] `run(tx: mpsc::Sender<ClipboardChange>) -> !` conecta ao compositor, escuta evento `selection`, negocia mime type (texto vs imagem reconhecida), lê conteúdo via `receive`
- [ ] Conteúdo não reconhecido (nem UTF-8 nem PNG/JPEG) é ignorado, sem emitir evento
- [ ] Reconexão com backoff (1s, 2s, 5s, cap 30s) se a conexão Wayland cair
- [ ] Erro claro logado e processo encerra se `COSMIC_DATA_CONTROL_ENABLED` não estiver setada (conexão ao protocolo falha)
- [ ] Gate check passa: `cargo clippy -p copied-daemon -- -D warnings` (funcionalidade real é verificação manual em T12, protocolo Wayland não é mockável de forma útil)

**Tests**: none (verificação manual em T12)
**Gate**: quick (clippy apenas)

---

### T7: `copied-daemon::clipboard_write` — SET do clipboard [P]

**What**: Wrapper fino sobre `wl_clipboard_rs::copy` pra escrever texto ou imagem de volta no clipboard do sistema.
**Where**: `crates/copied-daemon/src/clipboard_write.rs`
**Depends on**: T1
**Reuses**: `wl-clipboard-rs::copy::{copy, Options, Source, MimeType}`
**Requirement**: CLIP-04

**Done when**:
- [ ] `write_text(content: &str) -> Result<(), Error>` usa `MimeType::Text`
- [ ] `write_image(bytes: Vec<u8>, mime: &str) -> Result<(), Error>` usa `MimeType::Specific`
- [ ] Usa modo não-foreground (default) — thread de serving fica presa no processo do daemon, não bloqueia a chamada
- [ ] Gate check passa: `cargo clippy -p copied-daemon -- -D warnings` (verificação de escrita real no clipboard é manual em T12)

**Tests**: none (verificação manual em T12)
**Gate**: quick (clippy apenas)

---

### T8: `copied-daemon::main` — wiring do daemon

**What**: `main.rs` sobe as 3 threads (watcher, ipc listener, main dono do estado), carrega `stack.json` no boot, persiste a cada mutação, conecta `ClipboardChange` → `Stack::push`, conecta `Command::CopyToClipboard` → `clipboard_write`.
**Where**: `crates/copied-daemon/src/main.rs`
**Depends on**: T3, T4, T5, T6, T7
**Reuses**: todos os módulos acima
**Requirement**: CLIP-01 até CLIP-07 (ponto de integração)

**Done when**:
- [ ] `Arc<Mutex<DaemonState>>` compartilhado entre threads via `Arc::clone`
- [ ] Boot: carrega `~/.local/share/copied/stack.json` (via T4), sobe watcher (T6) numa thread, sobe IPC listener (T5) noutra
- [ ] Toda mutação de estado (push/delete/pin/unpin) dispara `persistence::save` antes de responder ao cliente
- [ ] `Command::CopyToClipboard` lê o item (texto direto, ou bytes do arquivo de imagem) e chama `clipboard_write` correspondente
- [ ] Disco cheio / erro de escrita em `save` é logado, daemon continua operando em memória (não crasha)
- [ ] Gate check passa: `cargo build -p copied-daemon --release && cargo clippy -p copied-daemon -- -D warnings`

**Tests**: none (integração real é T12)
**Gate**: quick (build + clippy)

---

### T9: systemd unit + script de instalação [P]

**What**: Arquivo `.service` e instruções/scripts de setup (env var system-wide + reboot, `systemctl --user enable`).
**Where**: `deploy/copied-daemon.service`, `deploy/install.sh` (ou seção no README)
**Depends on**: T1
**Reuses**: unit file já rascunhado no design.md
**Requirement**: CLIP-08

**Done when**:
- [ ] `deploy/copied-daemon.service` com `WantedBy=graphical-session.target`, `Restart=on-failure`, `ExecStart=%h/.cargo/bin/copied-daemon`
- [ ] Script ou instruções pra `/etc/profile.d/copied-clipboard.sh` com `COSMIC_DATA_CONTROL_ENABLED=1` (requer sudo + reboot, documentado como pré-requisito manual)
- [ ] Instruções de instalação do atalho de teclado no COSMIC Settings (comando: `cosmic-term -e copied show`)

**Tests**: none (config estática)
**Gate**: none

---

### T10: `copied-cli::ipc_client` — cliente do socket [P]

**What**: Conecta ao unix socket, envia `Command` serializado (NDJSON), lê `Response`.
**Where**: `crates/copied-cli/src/ipc_client.rs`
**Depends on**: T2
**Reuses**: `copied_core::{Command, Response}`
**Requirement**: CLIP-03 (pré-requisito de conexão)

**Done when**:
- [ ] `IpcClient::connect(socket_path) -> io::Result<Self>`
- [ ] `send(&mut self, cmd: Command) -> io::Result<Response>` escreve linha NDJSON, lê linha de resposta
- [ ] Erro de conexão (socket ausente) retorna `io::Error` claro, propagado pra T11 tratar
- [ ] Gate check passa: `cargo clippy -p copied-cli -- -D warnings`

**Tests**: none (integração real com servidor é manual em T12)
**Gate**: quick (clippy apenas)

---

### T11: `copied-cli::app` — TUI (ratatui) com navegação e ações

**What**: `App` struct com loop `ratatui`, lista pins + não-pinados, navegação por setas, Enter copia+fecha, Delete exclui, tecla dedicada pin/unpin, Esc fecha sem ação.
**Where**: `crates/copied-cli/src/app.rs`, `crates/copied-cli/src/main.rs`
**Depends on**: T10
**Reuses**: `IpcClient` de T10, `copied_core::ItemView`
**Requirement**: CLIP-03, CLIP-04, CLIP-05, CLIP-06

**Done when**:
- [ ] `main.rs` conecta via `IpcClient`; se falhar, imprime "daemon não encontrado — inicie o serviço" e encerra (sem abrir a TUI)
- [ ] Lista vazia mostra mensagem de estado vazio, sem erro
- [ ] Setas navegam; Enter envia `CopyToClipboard` e fecha; Delete envia `Delete` e atualiza lista sem fechar; tecla de pin envia `Pin`/`Unpin` conforme estado do item selecionado
- [ ] `Response::Error` (ex: limite de pins) exibe mensagem na TUI sem crashar
- [ ] Esc fecha sem enviar nenhum comando
- [ ] Gate check passa: `cargo build -p copied-cli --release && cargo clippy -p copied-cli -- -D warnings`

**Tests**: none (UX real é verificação manual em T12)
**Gate**: quick (build + clippy)

---

### T12: Verificação manual end-to-end

**What**: Rodar o sistema completo (daemon real + TUI real) no Pop!_OS 24.04 e validar todos os critérios de aceitação P1 do spec.md.
**Where**: N/A (verificação, não código)
**Depends on**: T8, T9, T11
**Reuses**: N/A
**Requirement**: CLIP-01 até CLIP-08 (todos)

**Done when**:
- [x] Copiar texto 16x seguidas → pilha tem 15, mais antigo descartado (verificado via IPC direto + daemon real, sessão COSMIC ativa)
- [x] Copiar conteúdo repetido → move pro topo, sem duplicar
- [x] Enter copia item de volta ao clipboard (verificado: `CopyToClipboard` + leitura real do clipboard do sistema, bytes idênticos)
- [x] Delete remove item (testado pinado e não-pinado, libera slot de pin)
- [x] Pinar 5 itens, tentar 6º → rejeitado com a mensagem exata "Máximo de 5 pins atingido — despine algo primeiro"; despinar e pinar outro funciona
- [x] Copiar uma imagem (PNG sintético) → aparece na pilha como `Image{mime, size_bytes}`, copiar de volta confirma bytes idênticos byte-a-byte
- [x] Matar o daemon (`kill`) e reiniciar → pilha e pins persistidos corretamente (13 itens/3 pins restaurados do `stack.json`)
- [x] TUI sem daemon rodando → mensagem de erro clara ("daemon não encontrado..."), exit code 1, sem travar
- [ ] `COSMIC_DATA_CONTROL_ENABLED=1` setada system-wide, reboot feito — **pendente, ação do usuário** (já estava setada nesta sessão de teste, mas não confirmamos o passo de instalação fresco via `/etc/profile.d/`)
- [ ] `systemctl --user enable --now copied-daemon.service` — **pendente, ação do usuário** (não mexemos em systemd por decisão explícita — ver `deploy/install.sh`)
- [ ] Atalho de teclado abre a TUI mostrando pilha correta — **pendente, ação do usuário** (requer configurar atalho no COSMIC Settings + interação de teclado real)
- [ ] Logout/login → daemon volta a rodar sozinho (autostart) — **pendente, ação do usuário** (depende do item de systemd acima)

**Tests**: manual — automatizado via IPC direto + `sim-copy` (helper `wl-clipboard-rs` descartável) contra o daemon real rodando na sessão COSMIC do usuário; itens de instalação/systemd/atalho de teclado ficaram para verificação manual do usuário, por escolha dele.
**Gate**: full (`cargo test --workspace && cargo clippy --workspace -- -D warnings && cargo fmt --check`) ✅ passou (29 testes, 0 warnings, formatação ok)

---

## Requirement Traceability

| Requirement ID | Tasks              | Status  |
| --------------- | ------------------- | ------- |
| CLIP-01         | T3, T6, T8, T12      | Verified |
| CLIP-02         | T4, T8, T12          | Verified |
| CLIP-03         | T2, T5, T10, T11, T12 | Verified (erro sem daemon confirmado; navegação real de teclado não testada) |
| CLIP-04         | T2, T5, T7, T8, T11, T12 | Verified |
| CLIP-05         | T2, T3, T5, T11, T12 | Verified |
| CLIP-06         | T2, T3, T5, T11, T12 | Verified |
| CLIP-07         | T4, T6, T8, T12      | Verified |
| CLIP-08         | T9, T12              | Implementing (arquivos prontos, autostart não habilitado — ação do usuário) |

**Coverage:** 8 total, 7 verificados end-to-end automaticamente, 1 (CLIP-08) aguardando instalação manual do usuário.

**Coverage**: 8/8 requisitos mapeados, 0 unmapped ✅
