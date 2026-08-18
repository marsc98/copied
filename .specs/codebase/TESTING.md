# Testing Strategy — copied

**Status**: Estratégia definida em `/taskify` (2026-07-03). Atualizado em 2026-07-04 (brownfield mapping) — testes unitários já escritos e presentes no código (confirmado por leitura direta dos arquivos-fonte).
**Atualizado novamente em 2026-07-04** (feature `copied-gui`): TUI (`copied-cli`) substituída por GUI (`copied-gui`); protocolo ganhou `GetImageBytes`/`SetCategory`.
**Atualizado novamente em 2026-08-18** (feature `symbols-emoji-catalog`): `copied-gui::symbols` ganhou `EMOJI_CATALOG` e teste correspondente; `copied-gui::app` ganhou a aba Emojis.
**Atualizado novamente em 2026-08-18** (feature `symbols-clipboard-nav`): novo `Command::CopyText` em `copied-core` (protocolo) com handler no daemon; `copied-gui::app` agora copia símbolo/emoji pelo daemon (removida escrita direta via `wl-clipboard-rs`) e ganhou navegação ↑/↓ por linha na grade de 9 colunas, com auto-scroll e cruzamento circular entre grupos.

## Test Frameworks

**Unit/Integration:** `cargo test` nativo (sem framework externo). Dev-dependency `tempfile` 3 pra isolar filesystem em testes de `persistence`, `ipc_client` e `instance_lock`.
**E2E:** nenhum framework — fluxos que dependem de Wayland/systemd real são manuais (ver tabela abaixo).
**Coverage tool:** nenhum configurado.

## Test Organization

**Location:** inline, `#[cfg(test)] mod tests { ... }` no fim de cada arquivo de módulo (nunca `*_test.rs` separado).
**Naming:** `snake_case`, descreve comportamento + condição (ex: `pin_rejects_sixth_pin_with_limit_reached`, `unpin_evicts_oldest_when_stack_is_full`).

## Test Coverage Matrix

| Camada                                              | Tipo de teste     | Onde                                | Comando        |
| ---------------------------------------------------- | ------------------ | ------------------------------------ | -------------- |
| `copied-core::lib` (Command/Response/ItemView/Category serde round-trip) | Unit (`cargo test`) | `crates/copied-core/src/lib.rs` (14 testes) | `cargo test -p copied-core` |
| `copied-daemon::stack` (LRU, dedup, pin/unpin/delete, categoria) | Unit (`cargo test`) | `crates/copied-daemon/src/stack.rs` (17 testes) | `cargo test -p copied-daemon` |
| `copied-daemon::categorize` (heurística URL/código/outro) | Unit (`cargo test`) | `crates/copied-daemon/src/categorize.rs` (3 testes) | `cargo test -p copied-daemon` |
| `copied-daemon::persistence` (load/save, corrupção, imagens, migração de categoria) | Unit (`cargo test`, `tempfile`) | `crates/copied-daemon/src/persistence.rs` (6 testes) | `cargo test -p copied-daemon` |
| `copied-daemon::ipc` (dispatch de `GetImageBytes`/`SetCategory`) | Unit (`cargo test`, `tempfile`) | `crates/copied-daemon/src/ipc.rs` (4 testes) | `cargo test -p copied-daemon` |
| `copied-gui::ipc_client` (protocolo de fio NDJSON)    | Unit/Integration (`cargo test`, socket real) | `crates/copied-gui/src/ipc_client.rs` (1 teste, `UnixListener` real em thread) | `cargo test -p copied-gui` |
| `copied-gui::instance_lock` (toggle de instância única, lock órfão) | Unit (`cargo test`, processos reais via `std::process::Command`) | `crates/copied-gui/src/instance_lock.rs` (3 testes) | `cargo test -p copied-gui` |
| `copied-gui::symbols` (catálogos estáticos não-vazios: símbolos e emojis) | Unit (`cargo test`, trivial) | `crates/copied-gui/src/symbols.rs` (2 testes) | `cargo test -p copied-gui` |
| `copied-daemon::watcher` (Wayland ext-data-control)   | Manual              | Rodar daemon real, copiar conteúdo, observar log/pilha | N/A |
| `copied-daemon::clipboard_write` (escrita real)       | Manual              | Rodar daemon real, copiar de volta, colar em outro app | N/A |
| `copied-gui::app` (event loop de UI: nav teclado/mouse, busca, preview, categoria, abas símbolos e emojis) | Manual              | Rodar `copied-gui` real, popular pilha, testar cada ação, incluindo cópia de símbolo/emoji via daemon (`Command::CopyText`) e navegação ↑/↓ por linha na grade (cruzamento de grupo e auto-scroll) | N/A |
| `copied-gui::main` (janela layer-shell, toggle de instância, Esc) | Manual              | Acionar o binário, testar popup/toggle/Esc num compositor Wayland real | N/A |
| systemd unit / autostart                              | Manual              | `systemctl --user status`, logout/login | N/A |

**Rationale**: mockar o protocolo Wayland e o systemd tem baixo retorno pra um projeto pessoal de aprendizado — a lógica de negócio pura (pilha, dedup, persistência, protocolo IPC, heurística de categoria) é isolável e testável sem I/O externo; a UI reativa (`iced`) e integrações de sistema (Wayland compositor real) são verificadas rodando o sistema real.

## Test Execution

```bash
cargo test --workspace          # roda todos os testes unitários/integração (50 testes no total)
cargo test -p copied-daemon      # só o crate do daemon
cargo test -p copied-gui         # só o crate do cliente gráfico
cargo clippy --workspace -- -D warnings
cargo fmt --check
```

## Gate Checks

- **quick**: `cargo test -p <crate> && cargo clippy -p <crate> -- -D warnings`
- **full**: `cargo test --workspace && cargo clippy --workspace -- -D warnings && cargo fmt --check`

## Convenções

- Testes unitários ficam inline no mesmo arquivo do módulo, em `#[cfg(test)] mod tests { ... }` — padrão idiomático Rust, sem arquivos `*_test.rs` separados.
- Nenhum teste depende de conexão Wayland real, systemd, ou filesystem fora de `tempfile::TempDir` (persistence tests usam dir temporário, não `~/.local/share` real).
