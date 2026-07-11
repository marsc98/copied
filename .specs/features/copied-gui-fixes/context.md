# copied-gui — Ajustes pós-implementação (janela + teclado) — Contexto para próximo agente

**Date:** 2026-07-04
**Status:** Bugs reportados em uso real (verificação manual), ainda não investigados/corrigidos.
**Relação com a feature original:** `.specs/features/copied-gui/` (spec.md/design.md/tasks.md) está com as 19 tarefas concluídas e commitadas — este documento é uma continuação (ajustes pós-MVP), não uma revisão do que já foi feito.

---

## Como chegar aqui

`copied-gui` (crate novo, `crates/copied-gui/`) substituiu `copied-cli` (TUI) por uma janela popup gráfica via `iced` 0.14 + `iced_layershell` 0.18 (`zwlr_layer_shell_v1`), rodando lado a lado com `copied-daemon` (processo separado, comunicação via socket Unix `$XDG_RUNTIME_DIR/copied.sock`). Ambiente: Pop!_OS 24.04 / COSMIC (compositor `cosmic-comp`, Wayland).

Toda a implementação (T1–T19 de `.specs/features/copied-gui/tasks.md`) está feita, com gate automatizado verde (`cargo test --workspace`, `cargo clippy --workspace -- -D warnings`, `cargo fmt --check`). Os dois problemas abaixo só apareceram na verificação manual em ambiente real (não são pegos por nenhum teste automatizado existente — a UI reativa do iced e a integração com o compositor real são categoria "Manual" na matriz de testes, ver `.specs/codebase/TESTING.md`).

---

## Problema 1: abre um terminal junto com o popup

**Sintoma relatado pelo usuário:** ao acionar o atalho, abre o popup (esperado) **e também** um terminal (não esperado — deveria abrir só o popup gráfico).

**Hipóteses (não confirmadas, ordenadas por probabilidade):**

1. **Atalho de teclado do COSMIC ainda está configurado com o comando antigo.** Antes desta feature, `deploy/install.sh` instruía o usuário a bindar `cosmic-term -e copied` (terminal abrindo o TUI antigo). O `install.sh` foi atualizado nesta sessão (T18) pra instruir `copied` puro (`deploy/install.sh:88-98`), mas **o script só imprime a instrução — não reconfigura o atalho automaticamente** (é passo manual em COSMIC Settings > Keyboard > Custom Shortcuts). Se o usuário não editou o atalho existente (só rodou `install.sh` de novo), ele continua bindado a `cosmic-term -e copied`, o que abriria terminal + (dentro dele) o binário `copied` novo — que por sua vez abre o popup. Resultado bateria exatamente com o sintoma.
   - **Como confirmar:** checar o comando atual do atalho em COSMIC Settings > Keyboard > Custom Shortcuts.
2. **Comportamento do próprio compositor/tiling ao criar a layer-shell surface.** Se a hipótese 1 for descartada (atalho já está `copied` puro e mesmo assim abre terminal), investigar se é o COSMIC (tiling) tratando a superfície de layer-shell de um jeito que força/abre um terminal de fallback, ou se há alguma lógica no próprio `main.rs`/`instance_lock.rs` disparando um processo externo por engano (não deveria — não há nenhum `Command::new`/`std::process::Command` no crate `copied-gui` além de `instance_lock`'s uso de `libc::kill`, ver `crates/copied-gui/src/instance_lock.rs`).

**Sugestão do usuário (caso seja tiling):** se o problema for o tiling do COSMIC brigando com a superfície layer-shell, considerar abrir como **janela normal (top-level, não layer-shell)** em vez de popup overlay. Isso é uma mudança arquitetural (trocar `iced_layershell::application` por uma janela `iced` comum, ou alternar entre os dois modos) — teria que revisitar a decisão de design registrada em `.specs/features/copied-gui/context.md` (seção "Modelo de janela", linha 23-27) e `design.md` (Tech Decisions), que escolheu layer-shell deliberadamente pra parecer um launcher/popup (Rofi/Albert-like), não uma janela comum. Vale confirmar a causa raiz antes de reverter essa decisão.

**Arquivos relevantes:**
- `crates/copied-gui/src/main.rs` (bootstrap, `LayerShellSettings`)
- `deploy/install.sh:88-98` (instrução de atalho, texto já corrigido nesta sessão)
- `.specs/features/copied-gui/context.md` (linhas 23-27, decisão de usar layer-shell)
- `.specs/features/copied-gui/design.md` (Tech Decisions, "Keyboard interactivity da layer-shell surface" e Open Questions sobre foco)

---

## Problema 2: comandos de teclado não funcionam

**Sintoma relatado pelo usuário:** dentro do popup aberto, teclas (setas, Enter, Delete, F2, Esc) não produzem efeito.

**Hipóteses (não confirmadas, ordenadas por probabilidade):**

1. **`iced::keyboard::listen()` só recebe eventos "Ignored".** A implementação em `crates/copied-gui/src/app.rs:439` (`keyboard_subscription`) usa `iced::keyboard::listen()`, que internamente (`iced_futures::keyboard::listen`, ver `iced_futures-0.14.0/src/keyboard.rs`) só emite eventos com `status: core::event::Status::Ignored` — ou seja, teclas **já consumidas por algum widget focado não chegam nesse listener**. A `view()` (`app.rs`, função `view`) tem um `text_input` (campo de busca) sempre presente na árvore de widgets. Se esse `text_input` estiver com foco (por padrão, ou por causa de como o `iced_layershell` inicializa foco de teclado), ele pode estar consumindo as teclas de navegação (setas, Enter, Delete) antes delas virarem "Ignored" — o que bateria exatamente com o sintoma ("nada funciona").
   - **Como confirmar:** testar se digitar texto na busca funciona (se sim, o widget está recebendo input normalmente, reforçando essa hipótese) e se navegação funciona quando o campo de busca não está em foco.
2. **A superfície layer-shell nunca recebe foco de teclado do compositor.** `main.rs` usa `KeyboardInteractivity::OnDemand` (`crates/copied-gui/src/main.rs:30`) — modo que, em alguns compositores, só concede foco de teclado à superfície mediante alguma interação explícita (ex: clique). Se o COSMIC não estiver concedendo foco automaticamente ao abrir, nenhuma tecla chegaria à aplicação, independente do listener. Esse ponto já estava sinalizado como **Open Question não verificada** em `.specs/features/copied-gui/design.md` (seção Open Questions, linha ~168) — na época não foi possível confirmar via documentação do `iced_layershell` se existe evento explícito de foco.
   - **Como confirmar:** testar se clicar dentro do popup antes de apertar uma tecla faz a navegação funcionar (indicaria problema de foco, não de listener).

**Arquivos relevantes:**
- `crates/copied-gui/src/app.rs` — `keyboard_subscription` (função, perto do fim do arquivo) e `view`/`view_stack` (onde o `text_input` é montado)
- `crates/copied-gui/src/main.rs` — `KeyboardInteractivity::OnDemand`
- `.specs/features/copied-gui/design.md` — Open Questions (foco/perda de foco não confirmados na implementação original)

---

## O que NÃO foi investigado ainda

Nenhuma das hipóteses acima foi testada/confirmada — este documento existe pra dar contexto suficiente pra próximo agente **investigar antes de corrigir**, não pra aplicar a primeira hipótese cegamente. Em particular:

- Não foi confirmado qual é o comando atualmente bindado no atalho do COSMIC (Problema 1, hipótese 1).
- Não foi possível reproduzir os bugs neste ambiente de desenvolvimento (sandbox sem `COSMIC_DATA_CONTROL_ENABLED` ativo no compositor — o daemon derruba no watcher Wayland; ver `.specs/codebase/CONCERNS.md`, seção "Fragile Areas"). Os testes automatizados (`cargo test --workspace`) continuam todos verdes; os bugs são de comportamento em ambiente real, não pegos por eles.
- Não foi verificado se `iced_layershell` 0.18 expõe algum evento/callback de foco de teclado que o código atual não está consumindo.

## Sugestão de próximo passo

Dado que a causa raiz de ambos os problemas ainda é incerta (múltiplas hipóteses plausíveis, não excludentes entre si), recomenda-se rodar **`/interview`** com o usuário focado nesses dois sintomas — pra confirmar reprodução, descartar hipóteses (ex: perguntar diretamente qual comando está no atalho hoje) e então decidir entre `/execute` modo rápido (se a causa for simples, ex: só reconfigurar o atalho) ou um ciclo `/design` + `/taskify` (se a causa exigir mudança arquitetural, ex: trocar layer-shell por janela top-level, ou mudar a estratégia de foco de teclado).

## Agent's Discretion

- Se confirmado que o atalho antigo é a causa do Problema 1, a correção é fora do repositório (reconfiguração manual do usuário em COSMIC Settings) — não há código pra mudar, só reforçar a instrução/documentação se necessário.
- Se o Problema 2 for causado pelo `text_input` capturando foco, a correção mais simples é conferir se `iced` permite não focar automaticamente nenhum widget no boot, ou mover a navegação por teclado pra um nível que intercepte antes do `text_input` (ex: usar `on_key_press`/eventos de nível de aplicação em vez de depender do filtro "Ignored" de `keyboard::listen()`).
