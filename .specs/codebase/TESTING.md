# Testing Strategy — copied

**Status**: Estratégia definida em `/taskify` (2026-07-03). Atualizado em 2026-07-04 (brownfield mapping) — testes unitários já escritos e presentes no código (confirmado por leitura direta dos arquivos-fonte).

## Test Frameworks

**Unit/Integration:** `cargo test` nativo (sem framework externo). Dev-dependency `tempfile` 3 pra isolar filesystem em testes de `persistence` e `ipc_client`.
**E2E:** nenhum framework — fluxos que dependem de Wayland/systemd real são manuais (ver tabela abaixo).
**Coverage tool:** nenhum configurado.

## Test Organization

**Location:** inline, `#[cfg(test)] mod tests { ... }` no fim de cada arquivo de módulo (nunca `*_test.rs` separado).
**Naming:** `snake_case`, descreve comportamento + condição (ex: `pin_rejects_sixth_pin_with_limit_reached`, `unpin_evicts_oldest_when_stack_is_full`).

## Test Coverage Matrix

| Camada                                              | Tipo de teste     | Onde                                | Comando        |
| ---------------------------------------------------- | ------------------ | ------------------------------------ | -------------- |
| `copied-core::lib` (Command/Response/ItemView serde round-trip) | Unit (`cargo test`) | `crates/copied-core/src/lib.rs` (9 testes) | `cargo test -p copied-core` |
| `copied-daemon::stack` (LRU, dedup, pin/unpin/delete) | Unit (`cargo test`) | `crates/copied-daemon/src/stack.rs` (14 testes) | `cargo test -p copied-daemon` |
| `copied-daemon::persistence` (load/save, corrupção, imagens) | Unit (`cargo test`, `tempfile`) | `crates/copied-daemon/src/persistence.rs` (5 testes) | `cargo test -p copied-daemon` |
| `copied-cli::ipc_client` (protocolo de fio NDJSON)    | Unit/Integration (`cargo test`, socket real) | `crates/copied-cli/src/ipc_client.rs` (1 teste, `UnixListener` real em thread) | `cargo test -p copied-cli` |
| `copied-daemon::watcher` (Wayland ext-data-control)   | Manual              | Rodar daemon real, copiar conteúdo, observar log/pilha | N/A |
| `copied-daemon::ipc` (dispatch de `Command`, orquestração de side-effects) | Nenhum (não testado) | — | N/A |
| `copied-daemon::clipboard_write` (escrita real)       | Manual              | Rodar daemon real, copiar de volta, colar em outro app | N/A |
| `copied-cli::app` (TUI: navegação, render, event loop) | Manual              | Rodar cliente real, navegar/testar teclas | N/A |
| systemd unit / autostart                              | Manual              | `systemctl --user status`, logout/login | N/A |

**Rationale**: mockar o protocolo Wayland e o systemd tem baixo retorno pra um projeto pessoal de aprendizado — a lógica de negócio pura (pilha, dedup, persistência, protocolo IPC) é isolável e testável sem I/O externo; o resto é verificado rodando o sistema real.

## Test Execution

```bash
cargo test --workspace          # roda todos os testes unitários/integração
cargo test -p copied-daemon      # só o crate do daemon
cargo clippy --workspace -- -D warnings
cargo fmt --check
```

## Gate Checks

- **quick**: `cargo test -p <crate> && cargo clippy -p <crate> -- -D warnings`
- **full**: `cargo test --workspace && cargo clippy --workspace -- -D warnings && cargo fmt --check`

## Convenções

- Testes unitários ficam inline no mesmo arquivo do módulo, em `#[cfg(test)] mod tests { ... }` — padrão idiomático Rust, sem arquivos `*_test.rs` separados.
- Nenhum teste depende de conexão Wayland real, systemd, ou filesystem fora de `tempfile::TempDir` (persistence tests usam dir temporário, não `~/.local/share` real).
