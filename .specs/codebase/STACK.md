# Tech Stack

**Analyzed:** 2026-07-04

## Core

- Language: Rust, edition 2021, toolchain `stable` (pinned via `rust-toolchain.toml`)
- Build/package manager: Cargo, workspace com `resolver = "2"`
- Runtime: nativo (binários systemd user service + CLI), sem async runtime (ver ARCHITECTURE.md — decisão explícita contra tokio)

## Workspace (3 crates)

- `copied-core` — tipos de domínio + protocolo IPC compartilhado (lib)
- `copied-daemon` — daemon background: watcher Wayland, stack, persistência, servidor IPC (bin)
- `copied-cli` — cliente TUI, binário `copied` (bin)

## Frontend

- TUI: `ratatui` 0.30, `crossterm` 0.29 (input de teclado + render terminal)
- Não há frontend web/gráfico — produto é deliberadamente terminal-only

## Backend / IPC

- Protocolo: NDJSON (JSON delimitado por `\n`) sobre Unix domain socket em `$XDG_RUNTIME_DIR/copied.sock`
- Serialização: `serde` 1 + `serde_json` 1, com `#[serde(tag = "...")]` para enums (`Command`, `Response`, `ItemKindView`)
- Concorrência: threads nativas (`std::thread`, `std::sync::mpsc`, `Arc<Mutex<DaemonState>>`) — uma thread por conexão de cliente IPC
- Sem banco de dados: persistência é arquivo JSON (`stack.json`) + arquivos de imagem em cache dir

## Integração com o sistema

- Clipboard watch: `wayland-client` 0.31 + `wayland-protocols-wlr` 0.3 (protocolo `zwlr_data_control_manager_v1`, implementação própria do watcher)
- Clipboard write: `wl-clipboard-rs` 0.9 (módulo `copy`)
- Hash de conteúdo (dedup): `sha2` 0.10 (SHA-256)
- IDs: `uuid` 1 (`v4`, feature `serde`)
- Autostart: systemd user unit (`deploy/copied-daemon.service`)

## Testing

- Unit/Integration: `cargo test` nativo, testes inline em `#[cfg(test)] mod tests` no mesmo arquivo do módulo
- Dev-dependency: `tempfile` 3 (isolamento de filesystem em testes de persistência/IPC)
- E2E/Manual: sem framework — fluxos que dependem de Wayland real ou systemd são verificados manualmente (ver TESTING.md)

## Development Tools

- Lint: `cargo clippy` (gate check exige `-D warnings`)
- Formatação: `cargo fmt --check`
- Deploy: `deploy/install.sh` (script bash idempotente) + `deploy/copied-daemon.service` (unit systemd `--user`)
