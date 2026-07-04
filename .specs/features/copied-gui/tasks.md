# copied-gui Tasks

**Design**: `.specs/features/copied-gui/design.md`
**Status**: Draft

---

## Execution Plan

### Phase 0: Foundation (Sequential — bloqueia todo o resto)
```
T1
```

### Phase 1a: Daemon track (roda em paralelo à Phase 1b — arquivos diferentes)
```
T1 ──┬──→ T2 ──┬──→ T4
     │         ├──→ T5
     └──→ T3 ──┘
       └────────────→ T6 (depende de T2 + T3)
```

### Phase 1b: GUI scaffold track (roda em paralelo à Phase 1a)
```
T1 ──→ T7 ──┬──→ T8  ──┐
            └──→ T9  ──┼──→ T10 ──→ T11 ──→ T12
                        └──────────┘
```

### Phase 2: MVP Checkpoint

Com T1–T6 (daemon) e T7–T12 (GUI) concluídas: demo manual de paridade + busca.

### Phase 3: P2 (Sequencial — ambas mexem em app.rs/Message)
```
T12, T5 ──→ T13 ──→ T14
```

### Phase 4: P3
```
T15 [P, sem dependências — dado puro] ──┐
T14 ────────────────────────────────────┴──→ T16
```

### Phase 5: Cutover (Sequencial)
```
T16 → T17 → T18 → T19
```

---

## Task Breakdown

### T1: copied-core — extensão de protocolo

**What**: Adicionar `socket_path()`, enum `Category`, `Command::GetImageBytes`/`Command::SetCategory`, `Response::ImageBytes`, campo `category` em `ItemView`, com testes de roundtrip serde.
**Where**: `crates/copied-core/src/lib.rs`
**Depends on**: None
**Reuses**: padrão `#[serde(tag = "...")]` já usado em `Command`/`Response`/`ItemKindView`
**Requirement**: GUI-05, GUI-06 (infra de protocolo)

**Tools**: NONE (edição direta de arquivo Rust)

**Done when**:
- [ ] `Category` enum (`Texto`/`Url`/`Codigo`/`Imagem`/`Outro`) com `#[serde(tag = "value")]` e `impl Default` (`Outro`)
- [ ] `Command::GetImageBytes { id }`, `Command::SetCategory { id, category }` adicionados
- [ ] `Response::ImageBytes { mime, data_base64 }` adicionado
- [ ] `ItemView.category: Category` adicionado
- [ ] `pub fn socket_path() -> io::Result<PathBuf>` (lê `$XDG_RUNTIME_DIR`, mesma lógica hoje duplicada em `copied-daemon::ipc` e `copied-cli::ipc_client`)
- [ ] `copied-daemon::ipc.rs` atualizado pra usar `copied_core::socket_path()` em vez da cópia local
- [ ] Gate check passa: `cargo test -p copied-core && cargo clippy -p copied-core -- -D warnings`
- [ ] Testes novos: roundtrip de `Category`, `Command::GetImageBytes`, `Command::SetCategory`, `Response::ImageBytes` (4 testes novos, seguindo o padrão de `roundtrip::<T>()` já existente)

**Tests**: unit (`cargo test -p copied-core`)
**Gate**: quick

---

### T2: copied-daemon::stack — campo category + set_category

**What**: Adicionar `category: Category` em `Item` (com `#[serde(default)]`), `Stack::set_category(id, category) -> Result<(), CategoryError>`.
**Where**: `crates/copied-daemon/src/stack.rs`
**Depends on**: T1
**Reuses**: padrão `Result<(), Error>` explícito de `pin`/`unpin` (`PinError`)
**Requirement**: GUI-06, GUI-07

**Tools**: NONE

**Done when**:
- [ ] `Item.category: Category` com `#[serde(default)]`
- [ ] `CategoryError::NotFound` (mesmo shape de `PinError`)
- [ ] `Stack::set_category` atualiza a categoria de item pinado ou não-pinado
- [ ] Gate check passa: `cargo test -p copied-daemon && cargo clippy -p copied-daemon -- -D warnings`
- [ ] Testes novos: `set_category_updates_existing_item`, `set_category_unknown_id_returns_not_found`, `set_category_works_on_pinned_item` (3 testes)

**Tests**: unit
**Gate**: quick

---

### T3: copied-daemon::categorize — heurísticas de detecção [P]

**What**: Novo arquivo com `pub fn detect(text: &str) -> Category` (heurística leve: URL se `starts_with("http://")`/`"https://"`, código se contém padrões comuns tipo `{`/`;`/`fn `/`def `, senão `Outro`).
**Where**: `crates/copied-daemon/src/categorize.rs` (novo)
**Depends on**: T1
**Reuses**: nenhum código existente (módulo novo e isolado)
**Requirement**: GUI-06

**Tools**: NONE

**Done when**:
- [ ] `detect()` reconhece URL, um padrão simples de código, e cai em `Outro` no caso genérico
- [ ] Módulo registrado em `main.rs` (`mod categorize;`)
- [ ] Gate check passa: `cargo test -p copied-daemon && cargo clippy -p copied-daemon -- -D warnings`
- [ ] Testes novos: `detect_url`, `detect_code_like_text`, `detect_falls_back_to_outro` (3 testes)

**Tests**: unit
**Gate**: quick

---

### T4: copied-daemon::persistence — teste de migração

**What**: Teste garantindo que um `stack.json` gravado sem o campo `category` (formato anterior a esta feature) carrega com `Category::Outro` via `#[serde(default)]`, sem descartar a pilha.
**Where**: `crates/copied-daemon/src/persistence.rs`
**Depends on**: T2
**Reuses**: padrão de teste já existente (`tempfile::tempdir`, escreve JSON manualmente, chama `load`)
**Requirement**: GUI-07

**Tools**: NONE

**Done when**:
- [ ] Teste escreve um JSON de `StackSnapshot` sem o campo `category` num item, chama `load`, verifica que o item vem com `Category::Outro` e a pilha não foi descartada
- [ ] Gate check passa: `cargo test -p copied-daemon`
- [ ] Teste novo: `load_defaults_category_for_snapshot_without_field` (1 teste)

**Tests**: unit
**Gate**: quick

---

### T5: copied-daemon::ipc — dispatch de GetImageBytes/SetCategory

**What**: `handle_command` ganha os dois braços novos: `GetImageBytes` (lê arquivo via `std::fs::read(path)`, encode base64, `Response::ImageBytes`/`Response::Error` se leitura falhar) e `SetCategory` (chama `Stack::set_category`, mapeia `CategoryError` pro `Response::Error` no mesmo estilo de `not_found()`).
**Where**: `crates/copied-daemon/src/ipc.rs`
**Depends on**: T2
**Reuses**: `not_found()`, padrão de match de `Response::Error` já usado nos outros comandos
**Requirement**: GUI-05, GUI-06

**Tools**: NONE

**Done when**:
- [ ] Dep `base64` adicionada em `crates/copied-daemon/Cargo.toml`
- [ ] `GetImageBytes` retorna `Response::ImageBytes` em sucesso, `Response::Error` se o arquivo de imagem não existe (item deletado/cache limpo) ou `id` não encontrado
- [ ] `SetCategory` retorna `Response::Ack` em sucesso, `Response::Error` se `id` não encontrado
- [ ] Gate check passa: `cargo test -p copied-daemon && cargo clippy -p copied-daemon -- -D warnings`
- [ ] Testes novos (paga parte do débito flagado em `CONCERNS.md` sobre `handle_command` não testado): `handle_get_image_bytes_returns_bytes_for_existing_image`, `handle_get_image_bytes_errors_when_file_missing`, `handle_set_category_updates_item`, `handle_set_category_errors_on_unknown_id` (4 testes, usando `DaemonState` com `tempfile::tempdir`)

**Tests**: unit
**Gate**: quick

---

### T6: copied-daemon::main — auto-detecção na captura

**What**: `handle_clipboard_change` chama `categorize::detect` ao criar `Item::new_text`; itens de imagem recebem `Category::Imagem` direto (sem heurística).
**Where**: `crates/copied-daemon/src/main.rs`
**Depends on**: T2, T3
**Reuses**: `categorize::detect`, `Item::new_text`/`new_image` existentes
**Requirement**: GUI-06

**Tools**: NONE

**Done when**:
- [ ] Texto capturado recebe categoria de `categorize::detect(&content)`
- [ ] Imagem capturada recebe `Category::Imagem` sempre
- [ ] Gate check passa: `cargo test -p copied-daemon && cargo clippy -p copied-daemon -- -D warnings`

**Tests**: none (orquestração de captura já é categoria "Manual" na matriz de teste — depende de Wayland real)
**Gate**: quick

---

### T7: copied-gui — scaffold do crate

**What**: Criar `crates/copied-gui/` como novo membro do workspace: `Cargo.toml` (deps: `iced`, `iced_layershell`, `copied-core`, `wl-clipboard-rs`, `base64`), `main.rs` stub que conecta ao socket e lista itens (smoke test sem UI ainda), `ipc_client.rs` movido de `copied-cli` (sem alterar o protocolo de fio, usando `copied_core::socket_path()`).
**Where**: `crates/copied-gui/Cargo.toml`, `crates/copied-gui/src/main.rs`, `crates/copied-gui/src/ipc_client.rs`, `Cargo.toml` (workspace root)
**Depends on**: T1
**Reuses**: `copied-cli/src/ipc_client.rs` (movido quase sem mudança)
**Requirement**: GUI-01 (infra)

**Tools**: NONE

**Done when**:
- [ ] `copied-gui` listado em `[workspace] members` do `Cargo.toml` raiz
- [ ] `cargo build -p copied-gui` compila
- [ ] `ipc_client.rs` tem o mesmo teste que tinha em `copied-cli` (`send_writes_command_line_and_reads_response_line`), passando
- [ ] Versão exata de `iced`/`iced_layershell` confirmada no `Cargo.lock` (ver Open Questions do `design.md`)
- [ ] Gate check passa: `cargo test -p copied-gui && cargo clippy -p copied-gui -- -D warnings`

**Tests**: unit (ipc_client, herdado)
**Gate**: quick

---

### T8: copied-gui::instance_lock — toggle de instância única

**What**: `acquire_or_signal_existing() -> LockOutcome` — lock file `$XDG_RUNTIME_DIR/copied-gui.pid`, checa se o PID gravado está vivo antes de decidir sinalizar (existente) vs assumir (lock órfão).
**Where**: `crates/copied-gui/src/instance_lock.rs` (novo)
**Depends on**: T7
**Reuses**: nenhum (módulo novo, isolado)
**Requirement**: GUI-02

**Tools**: NONE

**Done when**:
- [ ] `LockOutcome::Acquired(LockGuard)` — grava o PID, remove o arquivo no `Drop` do guard
- [ ] `LockOutcome::SignaledExisting` — detecta PID vivo, envia sinal, retorna sem travar
- [ ] Lock órfão (PID gravado não existe mais) é tratado como se não houvesse instância rodando
- [ ] Gate check passa: `cargo test -p copied-gui && cargo clippy -p copied-gui -- -D warnings`
- [ ] Testes novos (usando `tempfile::tempdir` pra isolar o lock file, sem depender de `$XDG_RUNTIME_DIR` real): `acquire_succeeds_when_no_lock_exists`, `signals_existing_when_pid_alive`, `acquires_when_lock_file_is_stale` (3 testes)

**Tests**: unit
**Gate**: quick

---

### T9: copied-gui::ipc_worker — ponte thread + canal

**What**: `spawn() -> (Sender<Command>, Receiver<Response>)` — thread nativa dona do `IpcClient` bloqueante, consome `std::sync::mpsc::Receiver<Command>`, produz em `futures::channel::mpsc::UnboundedSender<Response>`.
**Where**: `crates/copied-gui/src/ipc_worker.rs` (novo)
**Depends on**: T7
**Reuses**: `IpcClient` (T7), padrão thread+canal de `copied-daemon::watcher`/`main.rs`
**Requirement**: GUI-01 (infra)

**Tools**: NONE

**Done when**:
- [ ] Thread conecta uma vez ao iniciar; se falhar, envia um evento de erro pelo canal em vez de encerrar o processo
- [ ] Cada `Command` recebido é enviado sequencialmente (`IpcClient::send`, bloqueante) e a `Response` correspondente é encaminhada
- [ ] Gate check passa: `cargo test -p copied-gui && cargo clippy -p copied-gui -- -D warnings`

**Tests**: none (integração com socket real é melhor coberta manualmente nesta camada; a lógica de protocolo em si já é testada em `ipc_client.rs`)
**Gate**: quick

---

### T10: copied-gui::app — estado, update, view (paridade)

**What**: `AppState`, `enum Message`, `update`/`view` cobrindo list/navegação (teclado ↑↓ e mouse hover/clique)/copy/delete/pin-unpin, ligados ao `ipc_worker` via `subscription`.
**Where**: `crates/copied-gui/src/app.rs` (novo)
**Depends on**: T8, T9
**Reuses**: lógica de `copied-cli/src/app.rs` (fluxo de ações idêntico, adaptado pro modelo Elm do iced)
**Requirement**: GUI-01, GUI-03

**Tools**: NONE

**Done when**:
- [ ] Lista renderizada com pins primeiro, igual ao TUI hoje
- [ ] Enter/clique copia e fecha; Delete exclui e atualiza lista; F2-equivalente/clique pina/despina
- [ ] Estado vazio (pilha sem itens) exibido
- [ ] Erros do daemon (`Response::Error`) exibidos sem fechar a janela
- [ ] Scroll do mouse rola a lista
- [ ] Gate check passa: `cargo test -p copied-gui && cargo clippy -p copied-gui -- -D warnings`

**Tests**: none (event loop de UI é categoria "Manual" na matriz de teste, mesmo tratamento que `copied-cli::app` recebia)
**Gate**: quick
**Verificação manual**: rodar `copied-gui` com pilha populada e vazia; testar cada ação por teclado e por mouse; comparar contra o comportamento documentado de `copied-cli`.

---

### T11: copied-gui::app — busca/filtro

**What**: Campo de busca; filtro client-side sobre `items: Vec<ItemView>` já carregado (sem nova chamada IPC), case-insensitive, com estado vazio de busca distinto do estado vazio geral.
**Where**: `crates/copied-gui/src/app.rs`
**Depends on**: T10
**Reuses**: `AppState.items` já carregado por T10
**Requirement**: GUI-04

**Tools**: NONE

**Done when**:
- [ ] Digitar filtra a lista em tempo real
- [ ] Campo vazio mostra tudo
- [ ] Nenhum resultado mostra estado vazio de busca (texto diferente de "pilha vazia")
- [ ] Limpar o campo restaura a lista completa
- [ ] Gate check passa: `cargo test -p copied-gui && cargo clippy -p copied-gui -- -D warnings`

**Tests**: none (mesma categoria manual de T10)
**Gate**: quick
**Verificação manual**: popular a pilha com itens variados, testar termos com 0/1/N resultados.

---

### T12: copied-gui::main — janela layer-shell + wiring final (MVP)

**What**: `main.rs` final: checa `instance_lock`, monta `LayerShellSettings` (`anchor`, `size`, `exclusive_zone: 0`, `KeyboardInteractivity::OnDemand`), chama `iced_layershell::application(...).settings(...).run()`; Esc fecha a janela.
**Where**: `crates/copied-gui/src/main.rs`
**Depends on**: T8, T11
**Reuses**: `instance_lock` (T8), `app.rs` (T10/T11)
**Requirement**: GUI-01, GUI-02

**Tools**: NONE

**Done when**:
- [ ] Popup abre sem decoração, fora da taskbar
- [ ] Esc fecha a janela de forma confiável
- [ ] Segundo acionamento do atalho (segunda execução do binário) fecha a instância existente em vez de abrir uma nova
- [ ] Gate check passa: `cargo test -p copied-gui && cargo clippy -p copied-gui -- -D warnings`

**Tests**: none (manual — depende de compositor Wayland real)
**Gate**: quick
**Verificação manual**: acionar o binário, confirmar popup sem decoração; acionar de novo com o popup aberto, confirmar toggle; testar Esc; testar perda de foco (best-effort, ver Open Questions do design).

---

**✅ MVP Checkpoint**: Com T1–T6, T7–T12 concluídas, `copied-gui` cobre paridade total (GUI-01 a GUI-04) e o daemon já serve os comandos novos (GUI-05/06/07 no lado do servidor). Demoável e verificável isoladamente antes de seguir pra P2/P3.

---

### T13: copied-gui::app — preview de imagem inline

**What**: Ao listar um item de imagem, enviar `Command::GetImageBytes` via `ipc_worker`; ao receber `Response::ImageBytes`, decodificar base64 + validar como imagem, cachear em `image_cache: HashMap<ItemId, image::Handle>`, renderizar miniatura; fallback textual (`[Imagem MIME, KB]`) em qualquer falha (arquivo ausente, bytes corrompidos).
**Where**: `crates/copied-gui/src/app.rs`
**Depends on**: T5, T12
**Reuses**: fallback textual já existente em `copied-cli::app::render_item` (mesma lógica de formatação, adaptada)
**Requirement**: GUI-05

**Tools**: NONE

**Done when**:
- [ ] Item de imagem dispara `GetImageBytes` uma vez (cacheado, não repete a cada frame)
- [ ] Miniatura renderizada em caso de sucesso
- [ ] Fallback textual em caso de `Response::Error` ou falha de decodificação, sem panic
- [ ] Gate check passa: `cargo test -p copied-gui && cargo clippy -p copied-gui -- -D warnings`

**Tests**: none (renderização — manual)
**Gate**: quick
**Verificação manual**: copiar uma imagem real, abrir popup, confirmar miniatura; remover o arquivo de cache manualmente e confirmar fallback sem crash.

---

### T14: copied-gui::app — categorização (auto + edição)

**What**: Exibir categoria (rótulo/ícone) por item; ação de UI pra editar, disparando `Command::SetCategory` via `ipc_worker`; atualiza exibição em `Response::Ack`.
**Where**: `crates/copied-gui/src/app.rs`
**Depends on**: T5, T13
**Reuses**: `ItemView.category` (T1)
**Requirement**: GUI-06

**Tools**: NONE

**Done when**:
- [ ] Categoria visível na lista pra cada item
- [ ] Ação de edição envia `SetCategory` e reflete a resposta
- [ ] `Response::Error` (id não encontrado) exibido sem quebrar a lista
- [ ] Gate check passa: `cargo test -p copied-gui && cargo clippy -p copied-gui -- -D warnings`

**Tests**: none (manual)
**Gate**: quick
**Verificação manual**: copiar URL/código/texto genérico, confirmar categorias auto-atribuídas; editar uma manualmente.

---

### T15: copied-gui::symbols — catálogo estático [P]

**What**: `pub const CATALOG: &[SymbolGroup]` com símbolos de matemática/ícones, sem I/O, sem dependência de nada além de dados estáticos.
**Where**: `crates/copied-gui/src/symbols.rs` (novo)
**Depends on**: None (pode ser feito a qualquer momento, dado puro)
**Reuses**: nada
**Requirement**: GUI-08

**Tools**: NONE

**Done when**:
- [ ] Catálogo não-vazio, agrupado (ex: matemática, setas, ícones gerais)
- [ ] Gate check passa: `cargo test -p copied-gui && cargo clippy -p copied-gui -- -D warnings`
- [ ] Teste trivial: `catalog_is_not_empty_and_groups_have_symbols` (1 teste)

**Tests**: unit (trivial, dado estático)
**Gate**: quick

---

### T16: copied-gui::app — aba de símbolos

**What**: Tab switch (Stack ↔ Símbolos) reaproveitando o mesmo campo de busca (T11) pra filtrar símbolos; seleção escreve direto via `wl-clipboard-rs` (sem passar pelo `ipc_worker`/daemon) e fecha a janela.
**Where**: `crates/copied-gui/src/app.rs`
**Depends on**: T14, T15
**Reuses**: `wl-clipboard-rs` (mesmo crate de `copied-daemon::clipboard_write`), campo de busca de T11
**Requirement**: GUI-08

**Tools**: NONE

**Done when**:
- [ ] Alternar de aba mostra o catálogo de símbolos
- [ ] Busca filtra símbolos igual filtra itens da pilha
- [ ] Selecionar um símbolo copia via `wl-clipboard-rs` e fecha a janela
- [ ] Gate check passa: `cargo test -p copied-gui && cargo clippy -p copied-gui -- -D warnings`

**Tests**: none (manual)
**Gate**: quick
**Verificação manual**: abrir aba de símbolos, filtrar, selecionar, confirmar que foi pro clipboard e a janela fechou.

---

### T17: Remover copied-cli

**What**: Deletar `crates/copied-cli/`, remover do `[workspace] members`. Isso já remove `ratatui`/`crossterm` do workspace (eram deps só desse crate).
**Where**: `Cargo.toml` (raiz), `crates/copied-cli/` (removido)
**Depends on**: T16
**Reuses**: N/A
**Requirement**: Success Criteria (paridade + remoção do TUI antigo)

**Tools**: NONE

**Done when**:
- [ ] `crates/copied-cli/` não existe mais
- [ ] `Cargo.toml` raiz não lista `copied-cli` em `members`
- [ ] `grep -r ratatui\|crossterm` no workspace não retorna nada
- [ ] Gate check passa: `cargo build --workspace && cargo test --workspace && cargo clippy --workspace -- -D warnings && cargo fmt --check`

**Tests**: none (remoção)
**Gate**: full

---

### T18: deploy/install.sh — atualizar pra copied-gui

**What**: Trocar `cargo install --path crates/copied-cli` por `crates/copied-gui`; atualizar instrução do atalho de teclado: de `cosmic-term -e copied` (terminal) pra só `copied` (a janela agora é gráfica, não precisa mais de terminal).
**Where**: `deploy/install.sh`
**Depends on**: T17
**Reuses**: N/A
**Requirement**: GUI-01 (invocação sem terminal)

**Tools**: NONE

**Done when**:
- [ ] Step 1/4 instala `copied-gui` em vez de `copied-cli`
- [ ] Step 4/4 instrui o atalho como comando direto `copied`, sem `cosmic-term -e`
- [ ] `deploy/copied-daemon.service` revisado — confirmado que não precisa mudar (não referencia o cliente)

**Tests**: none (script de shell, verificação manual/leitura)
**Gate**: n/a (não é código Rust)

---

### T19: Atualizar docs de brownfield (.specs/codebase/)

**What**: Atualizar `STACK.md`, `ARCHITECTURE.md`, `STRUCTURE.md`, `TESTING.md`, `CONCERNS.md` pra refletir `copied-gui` no lugar de `copied-cli`, os novos comandos IPC, e remover os itens de tech debt resolvidos nesta feature (duplicação de `socket_path()`).
**Where**: `.specs/codebase/*.md`
**Depends on**: T18
**Reuses**: N/A
**Requirement**: N/A (manutenção de documentação)

**Tools**: NONE

**Done when**:
- [ ] `STACK.md` lista `iced`/`iced_layershell` em vez de `ratatui`/`crossterm`
- [ ] `ARCHITECTURE.md` — diagrama e fluxos atualizados pro cliente novo
- [ ] `STRUCTURE.md` — árvore de diretórios reflete `copied-gui`
- [ ] `TESTING.md` — matriz de cobertura inclui `copied-gui::instance_lock`, `copied-gui::symbols`, `copied-daemon::categorize`, e remove linhas de `copied-cli`
- [ ] `CONCERNS.md` — remove o item "socket_path() duplicada" (resolvido em T1); reavalia o item "orquestração de comandos IPC não testada" à luz dos testes novos de T5

**Tests**: none (documentação)
**Gate**: n/a

---

## Task Verification Standards

Todas as tarefas de código Rust têm gate `quick` (`cargo test -p <crate> && cargo clippy -p <crate> -- -D warnings`) exceto T17 (`full`, por mexer na composição do workspace) e T18/T19 (não são código Rust, verificação por leitura/execução manual do script).

## Requirement Coverage

| Requirement ID | Tasks | Status |
| --- | --- | --- |
| GUI-01 | T7, T8, T9, T10, T12 | Pending |
| GUI-02 | T8, T12 | Pending |
| GUI-03 | T10 | Pending |
| GUI-04 | T11 | Pending |
| GUI-05 | T1, T5, T13 | Pending |
| GUI-06 | T1, T2, T3, T5, T6, T14 | Pending |
| GUI-07 | T2, T4 | Pending |
| GUI-08 | T15, T16 | Pending |

**Coverage:** 8 total, 8 mapped to tasks, 0 unmapped ✅
