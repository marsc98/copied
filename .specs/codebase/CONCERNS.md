# Codebase Concerns

**Analysis Date:** 2026-07-04
**Atualizado:** 2026-07-04 (feature `copied-gui`)

## Tech Debt

**Escrita não-atômica de `stack.json`:**

- Issue: `persistence::save` (`crates/copied-daemon/src/persistence.rs:44-54`) grava com `fs::write(path, json)` direto — sobrescreve o arquivo inteiro no lugar, sem escrever em arquivo temporário e renomear.
- Files: `crates/copied-daemon/src/persistence.rs:53`
- Why: caminho mais simples pra um projeto de aprendizado; `persist()` é chamado de forma síncrona a cada mutação (sem debounce), então o risco existe a cada Delete/Pin/Unpin/clipboard change.
- Impact: se o processo for morto (`kill -9`, OOM killer, queda de energia) exatamente durante o `write`, `stack.json` fica truncado/parcial. No próximo boot, `persistence::load` detecta JSON inválido e **descarta a pilha inteira**, voltando pra vazia (comportamento correto de "não crashar", mas perde todo o histórico e pins, não só a última mutação).
- Fix approach: escrever em `stack.json.tmp` no mesmo diretório e usar `fs::rename` (atômico na mesma partição) pra substituir o arquivo final.

**~~`socket_path()` duplicada entre daemon e cliente~~ — Resolvido (feature `copied-gui`, 2026-07-04):**

`socket_path()` foi movida pra `copied-core::socket_path()` (única fonte de verdade), usada por `copied-daemon::ipc` (re-export) e por `copied-gui::ipc_client`. Não há mais duplicação — o cliente novo (`copied-gui`) importa direto de `copied-core` em vez de reimplementar.

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

**Orquestração de comandos IPC (`handle_command`, `handle_connection`) — parcialmente endereçado (feature `copied-gui`, 2026-07-04):**

- O que passou a ser testado: os dois braços novos de `handle_command` — `GetImageBytes` (sucesso e arquivo ausente) e `SetCategory` (sucesso e id desconhecido) — ganharam 4 testes usando `DaemonState` com `tempfile::tempdir` (`crates/copied-daemon/src/ipc.rs`, módulo `tests`), provando que esse padrão de teste é viável sem Wayland/systemd real.
- What's still not tested: os braços originais (`List`, `Delete`, `Pin`, `Unpin`, `CopyToClipboard`) continuam sem teste direto de `handle_command` — a cobertura desses fluxos vem só indiretamente da lógica pura em `stack.rs` (agora 17 testes) mais verificação manual via `copied-gui`.
- Risk: regressões na ordem de operações dos braços antigos (ex: esquecer `cleanup_image_if_any` antes de `state.persist()` em `Delete`/`Unpin`) não seriam pegas por `cargo test`.
- Priority: Medium — o padrão de teste já está estabelecido (ver `ipc.rs::tests::test_state()`); extensão pros braços restantes é um próximo passo de baixo risco, não uma mudança de abordagem.

**Validação de tamanho de conteúdo (spec CLIP-07, Edge Cases) não implementada:**

- What's not tested: a spec (`.specs/features/clipboard-manager/spec.md`, seção Edge Cases) exige que texto >1MB seja truncado/rejeitado e imagem >20MB seja rejeitada com aviso. Busca no código-fonte (`grep` por limites de tamanho) não encontrou nenhuma checagem desse tipo em `watcher.rs`, `stack.rs` ou `persistence.rs` — `watcher::process_offer` lê o pipe até EOF sem limite (`reader.read_to_end`), e `Stack::push_text`/`insert` não rejeitam por tamanho.
- Risk: colar um arquivo/imagem muito grande no clipboard pode inflar `stack.json` e o cache de imagens sem limite, ou consumir memória do daemon proporcional ao conteúdo colado — atualmente sem nenhuma salvaguarda.
- Priority: Medium — não é um bug de uso normal (a maioria dos conteúdos copiados é pequena), mas é um requisito de spec explicitamente não coberto por nenhum teste ou implementação.
