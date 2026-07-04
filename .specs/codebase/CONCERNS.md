# Codebase Concerns

**Analysis Date:** 2026-07-04

## Tech Debt

**Escrita não-atômica de `stack.json`:**

- Issue: `persistence::save` (`crates/copied-daemon/src/persistence.rs:44-54`) grava com `fs::write(path, json)` direto — sobrescreve o arquivo inteiro no lugar, sem escrever em arquivo temporário e renomear.
- Files: `crates/copied-daemon/src/persistence.rs:53`
- Why: caminho mais simples pra um projeto de aprendizado; `persist()` é chamado de forma síncrona a cada mutação (sem debounce), então o risco existe a cada Delete/Pin/Unpin/clipboard change.
- Impact: se o processo for morto (`kill -9`, OOM killer, queda de energia) exatamente durante o `write`, `stack.json` fica truncado/parcial. No próximo boot, `persistence::load` detecta JSON inválido e **descarta a pilha inteira**, voltando pra vazia (comportamento correto de "não crashar", mas perde todo o histórico e pins, não só a última mutação).
- Fix approach: escrever em `stack.json.tmp` no mesmo diretório e usar `fs::rename` (atômico na mesma partição) pra substituir o arquivo final.

**`socket_path()` duplicada entre daemon e cliente:**

- Issue: a mesma lógica (`$XDG_RUNTIME_DIR` + `.join("copied.sock")`) está implementada de forma idêntica em dois lugares.
- Files: `crates/copied-daemon/src/ipc.rs:35-43`, `crates/copied-cli/src/ipc_client.rs:46-53`
- Why: os dois binários não compartilham um crate de "config"/paths — só `copied-core` (tipos de protocolo).
- Impact: baixo hoje (5 linhas, comportamento simples), mas qualquer mudança futura no path do socket exige lembrar de editar os dois arquivos em sincronia.
- Fix approach: mover `socket_path()` pra `copied-core` como função pública única, usada por ambos.

## Security Considerations

**Socket IPC sem autenticação/autorização:**

- Risk: `UnixListener::bind` (`crates/copied-daemon/src/ipc.rs:50`) aceita qualquer conexão local, sem checar UID do peer. Qualquer processo do mesmo usuário (ou de outro usuário, se `$XDG_RUNTIME_DIR` for acessível) pode listar a pilha inteira (incluindo itens pinados) ou disparar `CopyToClipboard`/`Delete` sem restrição.
- Files: `crates/copied-daemon/src/ipc.rs:46-63`
- Current mitigation: `$XDG_RUNTIME_DIR` normalmente é `/run/user/<uid>` com permissão `0700`, o que já restringe acesso a outros usuários do sistema no caso comum. Nenhuma checagem adicional no código.
- Recommendations: aceitável pro escopo declarado (uso pessoal, single-user desktop, spec não menciona multi-usuário/multi-tenant); documentar essa premissa explicitamente se o projeto crescer além de uso pessoal.

## Fragile Areas

**`copied-daemon::watcher` — dependência de configuração externa ao processo:**

- Files: `crates/copied-daemon/src/watcher.rs:60-88`
- Why fragile: o protocolo `zwlr_data_control_manager_v1` só é anunciado pelo `cosmic-comp` se a variável de ambiente `COSMIC_DATA_CONTROL_ENABLED=1` estiver setada **no processo do compositor** (não no daemon) — uma condição de sistema fora do controle do código, configurada por `deploy/install.sh` escrevendo `/etc/profile.d/copied-clipboard.sh` e exigindo reboot. Se a flag não estiver ativa, a primeira tentativa de conexão falha e o daemon **encerra o processo** (`std::process::exit(1)`), o que por sua vez interage com o `Restart=on-failure` do systemd (loop de restart rápido até o usuário perceber o log).
- Safe modification: qualquer mudança na lógica de detecção de falha inicial (`ever_connected` em `run()`) deve preservar a distinção entre "nunca conectou" (fatal, exit) vs. "desconectou depois de conectar" (retry com backoff) — são tratados propositalmente de forma diferente.
- Test coverage: nenhuma (não é testável sem um compositor Wayland real rodando); mitigado só por verificação manual.

## Test Coverage Gaps

**Orquestração de comandos IPC (`handle_command`, `handle_connection`):**

- What's not tested: `crates/copied-daemon/src/ipc.rs` — dispatch de `Command` → `Stack` → efeitos colaterais (persist, cleanup de imagem evictada, escrita real no clipboard) não tem nenhum teste automatizado. É a camada que mais orquestra side-effects (a lógica pura em `stack.rs` tem 14 testes; a camada que a conecta a I/O real, zero).
- Risk: regressões na ordem de operações (ex: esquecer de chamar `cleanup_image_if_any` antes de `state.persist()`, ou inverter a checagem de `PinError::LimitReached` vs `NotFound`) não seriam pegas por `cargo test`.
- Priority: Medium — a lógica de domínio isolada (`stack.rs`) já cobre os casos de borda mais importantes; o que falta é testar a colagem entre `Stack`, `persistence` e `Response`, o que exigiria um `DaemonState` de teste com dir temporário (viável sem Wayland/systemd real, ao contrário de `watcher`/`clipboard_write`).

**Validação de tamanho de conteúdo (spec CLIP-07, Edge Cases) não implementada:**

- What's not tested: a spec (`.specs/features/clipboard-manager/spec.md`, seção Edge Cases) exige que texto >1MB seja truncado/rejeitado e imagem >20MB seja rejeitada com aviso. Busca no código-fonte (`grep` por limites de tamanho) não encontrou nenhuma checagem desse tipo em `watcher.rs`, `stack.rs` ou `persistence.rs` — `watcher::process_offer` lê o pipe até EOF sem limite (`reader.read_to_end`), e `Stack::push_text`/`insert` não rejeitam por tamanho.
- Risk: colar um arquivo/imagem muito grande no clipboard pode inflar `stack.json` e o cache de imagens sem limite, ou consumir memória do daemon proporcional ao conteúdo colado — atualmente sem nenhuma salvaguarda.
- Priority: Medium — não é um bug de uso normal (a maioria dos conteúdos copiados é pequena), mas é um requisito de spec explicitamente não coberto por nenhum teste ou implementação.
