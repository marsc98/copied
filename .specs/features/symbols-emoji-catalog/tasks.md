# Catálogo de Símbolos e Emojis — Tasks

**Design**: `.specs/features/symbols-emoji-catalog/design.md`
**Status**: Draft

---

## Execution Plan

Cadeia sequencial forte (o spike de fonte em T3 decide o conteúdo de T4) — sem paralelismo real disponível.

```
T1 → T2 → T3 → T4 → T5
```

- **T1**: dados (symbols.rs)
- **T2**: UI/navegação (app.rs) — fatia vertical demoable com fonte padrão
- **T3**: spike manual — decide se T4 bundla fonte(s)
- **T4**: bundling de fonte (condicional ao resultado de T3) + corte de bandeiras se necessário
- **T5**: gate final + atualização de TESTING.md

---

## Task Breakdown

### T1: Expandir os catálogos de dados

**What**: Adicionar as 8 categorias de texto novas ao `CATALOG` existente (Números Especiais, Pontuação e Tipografia, Moeda, Letras Gregas, Técnicos e Legais, Jogos, Clima e Natureza, Música) e criar o novo `EMOJI_CATALOG` com as 9 subcategorias de emoji do `symbols.md`, seguindo exatamente o conteúdo já revisado em `.specs/helpers/symbols.md`/`symbols.rs`.
**Where**: `crates/copied-gui/src/symbols.rs`
**Depends on**: None
**Reuses**: struct `SymbolGroup` existente (sem alteração de forma), conteúdo já curado em `.specs/helpers/symbols.md`
**Requirement**: SYM-01, SYM-08

**Tools**:
- MCP: NONE
- Skill: NONE

**Done when**:
- [x] `CATALOG` contém os 3 grupos originais + 8 novos, sem duplicar símbolos dentro de um grupo
- [x] `EMOJI_CATALOG` (novo `pub const`) contém as 9 subcategorias de emoji
- [x] Teste existente `catalog_is_not_empty_and_groups_have_symbols` continua passando
- [x] Novo teste equivalente cobre `EMOJI_CATALOG` (não-vazio, todo grupo com nome e símbolos)
- [x] Gate check passa: `cargo test -p copied-gui && cargo clippy -p copied-gui -- -D warnings`

**Tests**: unit (`cargo test -p copied-gui`, 2 testes em `symbols.rs` após a mudança)
**Gate**: quick

---

### T2: Ligar a aba Emojis e generalizar navegação por catálogo

**What**: Adicionar `Tab::Emojis` e `Focus::TabEmojis` (estendendo o ciclo Stack → Símbolos → Emojis → Busca → Lista), criar `active_catalog(tab: Tab) -> &'static [SymbolGroup]`, generalizar `visible_symbol_groups`/`view_symbols` pra aceitar o catálogo ativo (renomeando pra algo como `view_symbol_catalog` reutilizado pelas duas abas), corrigir `clamp_symbol_selection`/`move_symbol_group`/`move_symbol_index` (hoje hardcoded em `symbols::CATALOG`) pra usar `active_catalog(state.active_tab)`, e adicionar o terceiro botão de aba na `tab_bar`.
**Where**: `crates/copied-gui/src/app.rs`
**Depends on**: T1 (precisa de `EMOJI_CATALOG` existindo pra compilar/referenciar)
**Reuses**: `symbol_button_style`, padrão de `Focus`/`Tab` já existente, lógica de filtro por busca já existente em `visible_symbol_groups`
**Requirement**: SYM-02, SYM-04, SYM-05

**Tools**:
- MCP: NONE
- Skill: NONE

**Done when**:
- [x] Aba "Emojis" aparece na `tab_bar`, alterna `active_tab` e renderiza `EMOJI_CATALOG` com busca funcional
- [x] Aba "Símbolos" continua funcionando exatamente como antes (sem regressão)
- [x] Navegação por teclado (Tab, setas, clique) na aba Emojis opera sobre `EMOJI_CATALOG`, não sobre `CATALOG`
- [x] Ciclo de foco por teclado é Stack → Símbolos → Emojis → Busca → Lista
- [x] Gate check passa: `cargo test -p copied-gui && cargo clippy -p copied-gui -- -D warnings`

**Tests**: none automatizado (linha `copied-gui::app` em TESTING.md é Manual) — validação manual fica em T3
**Gate**: quick

---

### T3: Spike manual de renderização (decide fonte)

**What**: Rodar `cargo run -p copied-gui` (ambiente Wayland real do usuário) e inspecionar visualmente: (a) aba Símbolos — pelo menos um símbolo de cada grupo novo, com atenção a Letras Gregas, Frações e Moeda; (b) aba Emojis — pelo menos um emoji simples (ex: 😀) e a subcategoria Bandeiras (incluindo a sequência ZWJ 🏳️‍🌈). Registrar decisão: fonte padrão do iced cobre os símbolos de texto? Emoji renderiza colorido, mono, ou tofu? Bandeiras compõem visualmente ou quebram em glifos soltos?
**Where**: N/A (execução manual, sem deliverable de código — decisão registrada no início de T4)
**Depends on**: T2 (precisa da UI funcional nas duas abas)
**Reuses**: N/A
**Requirement**: SYM-06

**Tools**:
- MCP: NONE
- Skill: NONE — sandbox sem display Wayland; o agente prepara a amostra (T1/T2) e avisa quando pronta pra build, o usuário roda `cargo run -p copied-gui` no ambiente dele e reporta o resultado visual de volta pro agente

**Done when**:
- [x] Decisão registrada: bundlar fonte de texto (sim/não)? bundlar fonte de emoji (sim/não)? manter subcategoria Bandeiras (sim/não)?

**Decisão registrada (2026-08-18)**:
- Fonte de texto: **não bundlar** — fonte padrão cobre a maioria dos símbolos novos, sem tofu relatado.
- Fonte de emoji: **não bundlar** — emoji renderiza com mistura de mono/colorido (ex: 😀 mono, 🤣 colorido) via fallback do sistema, mas sem tofu e sem bandeiras quebradas; usuário decidiu que a inconsistência de cor não justifica +10-20MB de binário + atribuição de licença do Twemoji.
- Subcategoria "Emojis - Bandeiras": **manter** — bandeiras de país (🇧🇷, 🇺🇸, etc.) e a sequência ZWJ (🏳️‍🌈) renderizam corretamente.
- **Achado fora do escopo original do spike**: o layout de `row` sem quebra de linha (sem wrap nem scroll horizontal) escondia itens fora da área visível em grupos grandes — isso é o que inicialmente pareceu ser bandeiras "quebradas". Corrigido antes de fechar o spike: `row` → `grid::Grid::with_children(...).fluid(SYMBOL_BUTTON_SIZE)` em `view_symbol_catalog` (`app.rs`), com padding lateral/inferior ajustado (`right(18).bottom(15)`) pra não colar na scrollbar. Ver commits `7a19b52`, `37fc026`, `5637c2c`, `f3d7077`, `11294bd`.

**Tests**: none (manual, sem gate automatizado aplicável)
**Gate**: N/A

---

### T4: Bundlar fonte(s) conforme spike + aplicar decisão de bandeiras

**What**: Conforme decisão de T3: baixar/vendorizar `TwemojiMozilla.ttf` (COLR/CPAL v0, projeto `mozilla/twemoji-colr`) e/ou `NotoSans-Regular.ttf`/DejaVu Sans em `crates/copied-gui/fonts/`, encadear `.font(include_bytes!(...))` no builder `iced_layershell::application(...)` em `main.rs`, definir consts de fonte em `app.rs` (padrão de `BOLD_FONT`) e aplicá-las por widget nos grupos que precisarem (não como `default_font` global). Aplicar a decisão de bandeiras: remover o grupo "Emojis - Bandeiras" de `EMOJI_CATALOG` se T3 concluiu que quebra.
**Where**: `crates/copied-gui/fonts/*.ttf` (novos arquivos binários), `crates/copied-gui/src/main.rs`, `crates/copied-gui/src/app.rs`, `crates/copied-gui/src/symbols.rs` (só se remover bandeiras)
**Depends on**: T3
**Reuses**: padrão `BOLD_FONT` (`app.rs:48-51`) pra declarar as novas fontes como `iced::Font` const
**Requirement**: SYM-03, SYM-07, SYM-09

**Tools**:
- MCP: NONE
- Skill: NONE — download direto via `WebFetch`/`curl` das fontes oficiais (GitHub releases de `mozilla/twemoji-colr` pra `TwemojiMozilla.ttf`; Google Fonts pra `NotoSans-Regular.ttf`), só se T3 indicar necessidade

**Done when**:
- [x] Se T3 indicou tofu em texto: `NotoSans-Regular.ttf`/DejaVu Sans bundlada e aplicada aos grupos afetados — **N/A**, T3 não indicou tofu em texto
- [x] Se T3 indicou falha de emoji colorido: `TwemojiMozilla.ttf` bundlada e aplicada aos botões da aba Emojis — **N/A**, T3 concluiu que a inconsistência mono/colorido não justifica bundling
- [x] Arquivo de atribuição de licença adicionado pra qualquer fonte bundlada com exigência de atribuição (Twemoji é CC-BY-4.0) — **N/A**, nenhuma fonte bundlada
- [x] Grupo "Emojis - Bandeiras" removido de `EMOJI_CATALOG` se T3 concluiu que quebra; mantido caso contrário — mantido, bandeiras renderizam bem
- [x] Re-verificação manual rápida: rodar `cargo run -p copied-gui` de novo, confirmar que o problema identificado em T3 foi resolvido — problema real era layout (`row` sem wrap), corrigido nos commits do T3
- [x] Gate check passa: `cargo test -p copied-gui && cargo clippy -p copied-gui -- -D warnings`

**Tests**: none automatizado (fonte é asset visual, sem teste de renderização)
**Gate**: quick

---

### T5: Gate final e atualização de documentação de testes

**What**: Rodar o gate completo do workspace e atualizar `TESTING.md` — linha `copied-gui::symbols` (contagem de testes muda de 1 pra 2), linha `copied-gui::app` (Manual) ganha menção explícita à aba Emojis no que precisa ser testado manualmente.
**Where**: `.specs/codebase/TESTING.md`
**Depends on**: T4
**Reuses**: N/A
**Requirement**: N/A (housekeeping)

**Tools**:
- MCP: NONE
- Skill: NONE

**Done when**:
- [x] `cargo test --workspace` passa (contagem de testes atualizada de 48 pra 49+)
- [x] `cargo clippy --workspace -- -D warnings` sem warnings
- [x] `cargo fmt --check` sem diffs
- [x] `TESTING.md` reflete a nova contagem de testes e a aba Emojis na linha manual de `copied-gui::app`

**Tests**: workspace completo
**Gate**: full

---

## Requirement Traceability

| Requirement ID | Task(s) |
| --------------- | ------- |
| SYM-01           | T1      |
| SYM-02           | T2      |
| SYM-03           | T4      |
| SYM-04           | T2      |
| SYM-05           | T2      |
| SYM-06           | T3      |
| SYM-07           | T4      |
| SYM-08           | T1      |
| SYM-09           | T4      |

**Coverage:** 9/9 requisitos mapeados.
