# Nova janela de interação (copied-gui) — Interview Decisions

**Date:** 2026-07-04
**Scope:** Redesenhar a camada de apresentação (janela de interação) do cliente `copied` — hoje um TUI em terminal (`copied-cli`, ratatui/crossterm) — avaliando e escolhendo uma tecnologia Rust que permita estilização rica e suporte muito mais interações do que as atuais (nav/copy/delete/pin), mantendo Rust como linguagem principal (objetivo de estudo). O binário continua sendo disparado pelo mesmo atalho de teclado/terminal de sempre — muda a janela que ele abre, não a forma de invocação. Fora de escopo original: mudanças no daemon/protocolo IPC além do mínimo necessário pra viabilizar a nova UI (na prática, o mínimo necessário cresceu um pouco durante a entrevista — ver Decisões).
**Source:** Discussão informal (sem spec prévio) + análise do código atual (`.specs/codebase/ARCHITECTURE.md`, `STACK.md`, `STRUCTURE.md`, `crates/copied-cli/src/app.rs`, `crates/copied-core/src/lib.rs`, `crates/copied-daemon/src/persistence.rs`).

---

## Decisões

### Invocação e coexistência com o TUI

- O app continua invocado do mesmo jeito (atalho de teclado / terminal), mas passa a abrir uma janela gráfica própria em vez de um terminal com TUI.
- O `copied-cli` (ratatui/crossterm) **não** é mantido como modo alternativo (`--tui`) rodando lado a lado — é aposentado após o novo crate ficar funcional.
- **Rationale:** "Manter a CLI" se refere à forma de invocação (comando/atalho), não à interface visual em si.

### Tecnologia de UI

- **iced** (arquitetura Elm, retained-mode).
- **Rationale:** Estilização via structs de Style/Theme; alinhamento direto com o desktop do usuário — COSMIC é construído sobre `libcosmic`, que por sua vez é construído sobre `iced`. Aprendizado de Rust transfere pro ecossistema que ele já usa no dia a dia. Existe suporte a `zwlr_layer_shell_v1` via `iced_layershell` pra construir popups estilo launcher/applet.
- Alternativas descartadas: `egui` (immediate-mode, estilização mais limitada), `gtk4-rs` (binding sobre C, foge do tema COSMIC/iced), `slint` (DSL própria além de Rust).

### Modelo de janela

- Overlay via **wlr-layer-shell** (`zwlr_layer_shell_v1`, através de `iced_layershell`): sem decoração, estilo popup/launcher (Rofi/Albert/Spotlight), sem entrar em taskbar, fecha ao perder foco ou Esc.
- **Rationale:** Mesma família de protocolo Wayland (`zwlr_*`) que o `watcher.rs` do daemon já usa (`zwlr_data_control_manager_v1`) — já há precedente no projeto pra depender de extensões wlr. Combina com o padrão esperado de um clipboard manager (popup rápido, não janela top-level comum).
- Posicionamento (perto do cursor vs centro da tela), transparência e dimensões exatas: **a critério do implementador** (ver Agent's Discretion).

### Ação ao selecionar um item ou símbolo

- Copia pro clipboard (via `wl-clipboard-rs`, igual hoje) e fecha a janela. Usuário cola manualmente (Ctrl+V).
- **Rationale:** Mantém o comportamento atual, evita a complexidade de input simulation em Wayland (que exigiria `enigo`/`virtual-keyboard` protocol e devolução de foco à janela anterior — descartado por complexidade desproporcional ao ganho).

### Novas interações — primeira versão

Priorizadas nesta ordem de relevância (todas in scope da primeira versão):

1. **Busca/filtro por texto** — filtro em tempo real sobre a lista já carregada em memória (`items: Vec<ItemView>`). Não exige mudança de protocolo.
2. **Preview de imagem inline** — miniatura real da imagem na lista (hoje é só `"[Imagem PNG, 45KB]"` textual). Exige extensão de protocolo (ver seção própria abaixo).
3. **Mouse** (clique pra selecionar/copiar, scroll, hover) — natural em janela gráfica, ausente no TUI atual (só teclado). Teclado continua funcionando em paralelo.
4. **Categorias** — ver seção própria abaixo.
5. **Aba de símbolos** — catálogo de símbolos (matemática, ícones etc.) pra ajudar na digitação. Ver seção própria abaixo.

### Aba de símbolos

- Lista **estática, embutida no binário** `copied-gui` (catálogo fixo de símbolos unicode: matemática, ícones etc.), **sem passar pelo daemon/IPC/stack.json**.
- Selecionar um símbolo funciona igual a copiar um item da pilha: copia pro clipboard e fecha a janela.
- **Rationale:** Não é conteúdo capturado do clipboard nem precisa persistir/sincronizar entre sessões — não há razão pra estender o domínio do daemon (`stack.rs`) só pra isso. Mantém o daemon fora do escopo.

### Categorias

- Campo **único e editável** por item: `category` (ex.: enum fechado `Texto`/`URL`/`Código`/`Imagem`/`Outro`, ou `String` — detalhe de implementação).
- **Detecção automática** por padrão de conteúdo (heurística/regex simples: parece URL, parece código, etc.; imagem já é distinguível via `ItemKindView::Image`) preenche o valor por padrão.
- Usuário pode **sobrescrever** a categoria via ação na UI.
- Exige: campo `category` em `Item` (`copied-daemon/src/stack.rs`) e `ItemView` (`copied-core/src/lib.rs`), novo `Command::SetCategory { id, category }` no protocolo IPC, e lógica de detecção automática na captura (`main.rs::handle_clipboard_change` ou dentro do próprio `Stack`).
- **Rationale:** Usuário quer as duas coisas — sugestão automática E controle manual — não apenas uma ou outra. Modelo de valor único (não lista de tags) escolhido por simplicidade: cobre "ajudar a organizar" sem virar sistema de tags completo (Add/Remove, filtro multi-select).
- **Nota de migração:** `stack.json` existente não tem esse campo — carregar snapshots antigos precisa de um default (`Outro` ou string vazia) via `#[serde(default)]` ou equivalente, pra não quebrar `persistence::load`.

### Protocolo IPC para preview de imagem

- Novo `Command::GetImageBytes { id }` / `Response::ImageBytes { mime, bytes }` (bytes provavelmente em base64 dentro do JSON, dado que o protocolo é NDJSON).
- **Rationale:** Mantém o princípio arquitetural já estabelecido no projeto — `copied-cli`/`copied-gui` nunca acessa diretamente os diretórios XDG do daemon (cache de imagens, `stack.json`); toda comunicação passa pelo socket. A alternativa (expor `cache_path`/hash no `ItemView` e o cliente ler o arquivo direto do disco) foi descartada por quebrar esse isolamento, mesmo sendo mais rápida (sem round-trip nem base64).

### Estrutura de crates

- **Novo crate `copied-gui`** no workspace, ao lado de (não substituindo in-place) `copied-cli`.
- `copied-cli` é removido depois que `copied-gui` estiver funcional; `ipc_client.rs` é copiado/adaptado pro novo crate (não compartilhado via lib, dado que `copied-cli` não depende de `copied-daemon` e a duplicação é temporária).
- **Rationale:** Escolha explícita do usuário sobre a alternativa recomendada (evoluir in-place) — permite manter o TUI antigo funcional/comparável lado a lado durante o desenvolvimento do novo, antes de apagar.

---

## Agent's Discretion

- Posicionamento exato do overlay (perto do cursor vs centro da tela), transparência, dimensões/tamanho da janela, tema de cores (desde que combine com estética COSMIC).
- Enum vs `String` pra representar `category` no domínio/protocolo.
- Heurísticas exatas de detecção automática de categoria (quais regex/padrões contam como "URL", "Código", etc.).
- Catálogo exato de símbolos na aba de símbolos (quais matemáticos, quais ícones) e sua organização/agrupamento dentro da aba.
- Formato exato de transporte dos bytes de imagem no novo `Response::ImageBytes` (base64 padrão vs outro encoding), desde que compatível com NDJSON.

---

## Deferred Ideas

- Nenhuma — todas as ideias trazidas durante a entrevista (busca, preview de imagem, mouse, categorias, aba de símbolos) foram encaixadas no escopo da primeira versão.

---

## Open Questions

- Nenhuma pendente — todas as áreas mapeadas foram resolvidas com decisão explícita do usuário.
