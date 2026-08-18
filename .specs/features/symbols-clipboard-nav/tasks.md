# Cópia e Navegação por Teclado (Símbolos/Emojis) — Tasks

**Spec**: `.specs/features/symbols-clipboard-nav/spec.md`
**Status**: Draft

---

## Execution Plan

Cadeia sequencial — todo o trabalho de GUI (T3-T7) mexe no mesmo arquivo
(`app.rs`), e o protocolo (T1) precisa existir antes do daemon (T2) poder
compilar contra ele. Sem paralelismo real disponível.

```
T1 → T2 → T3 → T4 → T5 → T6 → T7 → T8
```

- **T1**: protocolo — novo `Command::CopyText` em `copied-core`
- **T2**: daemon — handler pra `Command::CopyText`
- **T3**: GUI — rotear cópia de símbolo/emoji pelo daemon (corrige o bug de raiz)
- **T4**: GUI — foco pula pra `List` ao ativar qualquer aba
- **T5**: GUI — grade de símbolos/emojis vira `.columns(9)` fixo
- **T6**: GUI — navegação ↑/↓ por linha, cruzando grupos com wraparound
- **T7**: GUI — auto-scroll ao mover seleção com ↑/↓
- **T8**: gate final do workspace + atualização de `TESTING.md`

---

## Task Breakdown

### T1: Novo comando `Command::CopyText` no protocolo

**What**: Adicionar variante `CopyText { text: String }` ao enum `Command` em
`copied-core`, seguindo o padrão de serde já usado pelas outras variantes, e
um teste de roundtrip (`command_copy_text_roundtrips`) seguindo o padrão
exato dos testes existentes (`command_copy_to_clipboard_roundtrips`, etc).
**Where**: `crates/copied-core/src/lib.rs`
**Depends on**: None
**Reuses**: helper `roundtrip::<T>()` já existente nos testes do módulo
**Requirement**: SCN-03, SCN-04

**Tools**:
- MCP: NONE
- Skill: NONE

**Done when**:
- [x] `Command::CopyText { text: String }` adicionado ao enum, com
      `#[derive]`s idênticos às demais variantes (serde já cobre o enum
      inteiro)
- [x] Novo teste `command_copy_text_roundtrips` cobre serialização/
      deserialização
- [x] Gate check passa: `cargo test -p copied-core && cargo clippy -p copied-core -- -D warnings`
- [x] Contagem de testes de `copied-core` sobe de 13 pra 14

**Tests**: unit (`cargo test -p copied-core`)
**Gate**: quick

---

### T2: Handler de `Command::CopyText` no daemon

**What**: Adicionar o braço `Command::CopyText { text } => ...` no `match` de
`handle_command` (`ipc.rs`), chamando `clipboard_write::write_text(&text)`
diretamente (sem tocar a pilha/stack) e retornando `Response::Ack` em sucesso
ou `Response::Error { message }` em falha — mesmo padrão de tratamento de
erro já usado em `copy_to_clipboard`.
**Where**: `crates/copied-daemon/src/ipc.rs`
**Depends on**: T1 (precisa da variante existir pra `match` compilar —
`handle_command` é exaustivo, sem `_ =>`)
**Reuses**: `clipboard_write::write_text` já existente, usado por
`copy_to_clipboard`
**Requirement**: SCN-03

**Tools**:
- MCP: NONE
- Skill: NONE

**Done when**:
- [x] `handle_command` trata `Command::CopyText` sem passar pela stack/
      `DaemonState` (não precisa de `id`, não precisa persistir nada)
- [x] Sucesso retorna `Response::Ack`; falha retorna `Response::Error` com
      mensagem descritiva
- [x] Gate check passa: `cargo test -p copied-daemon && cargo clippy -p copied-daemon -- -D warnings`

**Tests**: none — mesmo padrão já estabelecido para `Command::CopyToClipboard`
(escreve na clipboard real via `wl_clipboard_rs`, não testável em unit test
isolado sem compositor Wayland; a matriz de cobertura em `TESTING.md` já
escopa os testes de `copied-daemon::ipc` só a `GetImageBytes`/`SetCategory`)
**Gate**: quick

---

### T3: GUI roteia cópia de símbolo/emoji pelo daemon

**What**: Trocar `Message::SymbolClicked(symbol)` e o braço
`Tab::Symbols | Tab::Emojis` de `Message::EnterPressed` pra mandar
`Command::CopyText { text: symbol.to_string() }` via `state.send(...,
PendingAction::Copy)` em vez de chamar `copy_symbol_to_clipboard` local e
`iced::exit()` na hora — reusa o fluxo já existente
`Response::Ack => PendingAction::Copy => iced::exit()`. Remover a função
`copy_symbol_to_clipboard` e a dependência `wl-clipboard-rs` do
`copied-gui` por completo (não sobra nenhum uso).
**Where**: `crates/copied-gui/src/app.rs`, `crates/copied-gui/Cargo.toml`
**Depends on**: T2 (precisa do daemon já tratar o comando pra fazer sentido
funcionalmente — a GUI só compilaria sem T2, mas ficaria quebrada em
runtime)
**Reuses**: `state.send`/`PendingAction::Copy`, já usados por
`Message::ItemClicked`
**Requirement**: SCN-01, SCN-02, SCN-04

**Tools**:
- MCP: NONE
- Skill: NONE

**Done when**:
- [x] `SymbolClicked` manda `Command::CopyText` pro daemon em vez de
      escrever na clipboard direto
- [x] `EnterPressed` (foco `Search`/`List`, aba Símbolos/Emojis) faz o mesmo
- [x] Função `copy_symbol_to_clipboard` removida de `app.rs`
- [x] Dependência `wl-clipboard-rs` removida de `crates/copied-gui/Cargo.toml`
- [x] `cargo build -p copied-gui` compila sem a dependência
- [x] Gate check passa: `cargo test -p copied-gui && cargo clippy -p copied-gui -- -D warnings`
- [ ] Verificação manual: `cargo run -p copied-gui`, clicar num emoji,
      confirmar que `wl-paste` (ou colar em outro app) mostra o conteúdo
      **depois** da GUI já ter fechado
      **PENDENTE**: `wl-paste`/`xclip`/`xsel` indisponíveis neste ambiente —
      verificação manual não pôde ser executada aqui, precisa ser feita pelo
      usuário

**Tests**: none (matriz `TESTING.md` cobre `copied-gui::app` como Manual)
**Gate**: quick

---

### T4: Foco pula pra `List` ao ativar qualquer aba

**What**: Extrair um helper (ex: `fn activate_tab(state: &mut AppState, tab:
Tab)`) que seta `active_tab`, seta `focus = Focus::List` (em vez do atual
`Focus::TabStack`/`TabSymbols`/`TabEmojis`), e chama
`clamp_symbol_selection()` quando `tab != Tab::Stack` — usar esse helper
tanto em `Message::TabSelected` quanto nos três braços
`Focus::TabStack`/`TabSymbols`/`TabEmojis` de `Message::EnterPressed`, que
hoje só setam `active_tab` sem chamar `clamp_symbol_selection` (inconsistência
existente entre clique e Enter que essa unificação também corrige).
**Where**: `crates/copied-gui/src/app.rs`
**Depends on**: T3 (mesmo arquivo — sequenciado pra evitar conflito de
edição; sem dependência funcional real)
**Reuses**: `clamp_symbol_selection` já existente
**Requirement**: SCN-04

**Tools**:
- MCP: NONE
- Skill: NONE

**Done when**:
- [x] Clicar numa aba (Stack, Símbolos ou Emojis) deixa `state.focus ==
      Focus::List` imediatamente
- [x] Apertar Enter com foco em `Focus::TabStack`/`TabSymbols`/`TabEmojis`
      tem o mesmo efeito (ativa a aba E deixa foco em `List`)
- [x] Trocar pra Símbolos/Emojis (por clique ou Enter) chama
      `clamp_symbol_selection` nos dois caminhos igualmente
- [x] `Focus::TabStack`/`TabSymbols`/`TabEmojis` continuam existindo (ainda
      necessários pro ciclo de `Tab` e o realce visual do botão da aba)
- [x] Gate check passa: `cargo test -p copied-gui && cargo clippy -p copied-gui -- -D warnings`

**Tests**: none (matriz `TESTING.md` cobre `copied-gui::app` como Manual)
**Gate**: quick

---

### T5: Grade de símbolos/emojis com colunas fixas

**What**: Trocar `grid::Grid::with_children(buttons).fluid(SYMBOL_BUTTON_SIZE)`
por `.columns(SYMBOL_GRID_COLUMNS)`, com nova constante `const
SYMBOL_GRID_COLUMNS: usize = 9;` declarada junto de `SYMBOL_BUTTON_SIZE`.
**Where**: `crates/copied-gui/src/app.rs`
**Depends on**: T4 (mesmo arquivo — sequenciado)
**Reuses**: `SYMBOL_BUTTON_SIZE` já existente
**Requirement**: SCN-05

**Tools**:
- MCP: NONE
- Skill: NONE

**Done when**:
- [x] `SYMBOL_GRID_COLUMNS: usize = 9` declarada
- [x] `view_symbol_catalog` usa `.columns(SYMBOL_GRID_COLUMNS)` em vez de
      `.fluid(...)`
- [ ] Verificação manual: `cargo run -p copied-gui`, aba Símbolos e Emojis
      mostram 9 colunas por linha, densidade visual igual à de antes
      **PENDENTE**: sem display Wayland interativo neste ambiente pra
      screenshot — precisa ser verificado pelo usuário
- [x] Gate check passa: `cargo test -p copied-gui && cargo clippy -p copied-gui -- -D warnings`

**Tests**: none (matriz `TESTING.md` cobre `copied-gui::app` como Manual)
**Gate**: quick

---

### T6: Navegação ↑/↓ por linha, cruzando grupos

**What**: Substituir `move_symbol_group` por um novo método (ex:
`move_symbol_row(&mut self, delta: isize)`) que: (1) localiza grupo+índice
atual de `selected_symbol` em `visible_symbol_groups`; (2) calcula
`row = index / SYMBOL_GRID_COLUMNS`, `col = index % SYMBOL_GRID_COLUMNS`;
(3) aplica `row += delta`; (4) se `row` ficar fora dos limites do grupo atual
(negativo ou além do número de linhas do grupo), cruza pro grupo
anterior/seguinte — circulando pro outro extremo do catálogo se já estiver
no primeiro/último grupo — pousando na última/primeira linha do grupo
vizinho; (5) resolve o índice final como `row * SYMBOL_GRID_COLUMNS + col`,
ajustado (`.min(...)`) pro último item da linha se ela for mais curta que
`col`. Atualizar `Message::MoveSelection` (braço `Tab::Symbols | Tab::Emojis`)
pra chamar `move_symbol_row` em vez de `move_symbol_group`. Remover
`move_symbol_group` (fica sem uso).
**Where**: `crates/copied-gui/src/app.rs`
**Depends on**: T5 (precisa de `SYMBOL_GRID_COLUMNS` existir pra fazer a
matemática de linha/coluna)
**Reuses**: `visible_symbol_groups`, `active_catalog` já existentes; mesmo
padrão de wraparound circular (`rem_euclid`) já usado em
`move_selection`/`move_symbol_index`
**Requirement**: SCN-06, SCN-07

**Tools**:
- MCP: NONE
- Skill: NONE

**Done when**:
- [x] ↓ move a seleção uma linha abaixo no grupo atual, mesma coluna
- [x] ↑ move a seleção uma linha acima no grupo atual, mesma coluna
- [x] ↓ na última linha do grupo cruza pra primeira linha do próximo grupo,
      mesma coluna (ajustada pro último item se a linha for mais curta)
- [x] ↑ na primeira linha do grupo cruza pra última linha do grupo anterior,
      mesma coluna (ajustada)
- [x] ↓ no último item do último grupo circula pro primeiro item do
      primeiro grupo; ↑ no primeiro item do primeiro grupo circula pro
      último item do último grupo
- [x] `move_symbol_group` removido (sem uso restante)
- [x] ←/→ (`move_symbol_index`) permanecem sem nenhuma alteração de
      comportamento
- [x] Gate check passa: `cargo test -p copied-gui && cargo clippy -p copied-gui -- -D warnings`
- [ ] Verificação manual: `cargo run -p copied-gui`, navegar até o fim de um
      grupo grande (ex: "Números Especiais") e confirmar cruzamento de
      grupo mantendo coluna; navegar até o fim do catálogo e confirmar
      wraparound circular
      **PENDENTE**: sem display Wayland interativo neste ambiente. Lógica
      validada com 7 cenários de teste temporários (linha acima/abaixo,
      cruzamento pra frente/trás com ajuste de coluna curta, wraparound nos
      dois extremos, grupo único) rodados e removidos antes do commit — a
      matriz `TESTING.md` mantém `copied-gui::app` como Manual

**Tests**: none (matriz `TESTING.md` cobre `copied-gui::app` como Manual)
**Gate**: quick

---

### T7: Auto-scroll ao mover seleção com ↑/↓

**What**: Dar um `.id(SYMBOL_LIST_ID)` (nova const, mesmo padrão de
`LIST_ID`) ao `scrollable(...)` de `view_symbol_catalog`. Calcular a posição
absoluta da linha selecionada somando as linhas de todos os grupos
anteriores ao grupo atual (mesma lógica de `rows_in_group` de T6) mais a
linha dentro do grupo atual, dividido pelo total de linhas de todos os
grupos visíveis — igual à fração usada em `scroll_to_selection` pro Stack —
e retornar `iced::widget::operation::snap_to(SYMBOL_LIST_ID,
RelativeOffset { x: 0.0, y: fraction })` em vez de `Task::none()` no braço
`Tab::Symbols | Tab::Emojis` de `Message::MoveSelection`.
**Where**: `crates/copied-gui/src/app.rs`
**Depends on**: T6 (precisa da lógica de linha/grupo já estabelecida)
**Reuses**: padrão de `scroll_to_selection`/`snap_to` já usado pro Stack
**Requirement**: SCN-08

**Tools**:
- MCP: NONE
- Skill: NONE

**Done when**:
- [x] `view_symbol_catalog` dá `.id(SYMBOL_LIST_ID)` ao `scrollable`
- [x] Mover a seleção com ↑/↓ (inclusive cruzando grupo) retorna um `Task`
      de `snap_to` proporcional à posição da linha selecionada
- [x] `Message::MoveSymbolIndex` (←/→) continua retornando `Task::none()`,
      sem auto-scroll — sem alteração nesta task
- [x] Gate check passa: `cargo test -p copied-gui && cargo clippy -p copied-gui -- -D warnings`
- [ ] Verificação manual: `cargo run -p copied-gui`, navegar com ↓ por um
      grupo grande e confirmar que a view rola sozinha acompanhando a
      seleção
      **PENDENTE**: sem display Wayland interativo neste ambiente — precisa
      ser verificado pelo usuário

**Tests**: none (matriz `TESTING.md` cobre `copied-gui::app` como Manual)
**Gate**: quick

---

### T8: Gate final e atualização de documentação de testes

**What**: Rodar o gate completo do workspace e atualizar `TESTING.md` —
linha `copied-core::lib` (contagem de testes muda de 13 pra 14) e a linha
manual `copied-gui::app` ganha menção à cópia de símbolo/emoji via daemon e
à navegação por grade.
**Where**: `.specs/codebase/TESTING.md`
**Depends on**: T7
**Reuses**: N/A
**Requirement**: N/A (housekeeping)

**Tools**:
- MCP: NONE
- Skill: NONE

**Done when**:
- [ ] `cargo test --workspace` passa (contagem de testes atualizada de 49
      pra 50)
- [ ] `cargo clippy --workspace -- -D warnings` sem warnings
- [ ] `cargo fmt --check` sem diffs
- [ ] `TESTING.md` reflete a nova contagem de testes e a navegação por
      grade/cópia via daemon na linha manual de `copied-gui::app`

**Tests**: workspace completo
**Gate**: full

---

## Diagram-Definition Cross-Check

| Task | Depends On (corpo da task) | Diagrama mostra | Status |
| ---- | --------------------------- | ---------------- | ------ |
| T1   | None                         | (início da cadeia) | ✅ |
| T2   | T1                           | T1 → T2           | ✅ |
| T3   | T2                           | T2 → T3           | ✅ |
| T4   | T3                           | T3 → T4           | ✅ |
| T5   | T4                           | T4 → T5           | ✅ |
| T6   | T5                           | T5 → T6           | ✅ |
| T7   | T6                           | T6 → T7           | ✅ |
| T8   | T7                           | T7 → T8           | ✅ |

Nenhuma task marcada `[P]` — todo o trabalho de GUI é sequencial no mesmo
arquivo, e T1→T2 tem dependência real de compilação.

## Test Co-location Validation

| Task | Camada de código | Matriz exige | Task diz | Status |
| ---- | ------------------ | ------------- | --------- | ------ |
| T1   | `copied-core::lib` (Command serde) | Unit | unit (novo teste de roundtrip) | ✅ |
| T2   | `copied-daemon::ipc` (dispatch) | Unit só pra `GetImageBytes`/`SetCategory`; `CopyToClipboard` já é exceção sem teste | none, mesma exceção | ✅ |
| T3   | `copied-gui::app` | Manual | none | ✅ |
| T4   | `copied-gui::app` | Manual | none | ✅ |
| T5   | `copied-gui::app` | Manual | none | ✅ |
| T6   | `copied-gui::app` | Manual | none | ✅ |
| T7   | `copied-gui::app` | Manual | none | ✅ |
| T8   | workspace completo | — | workspace completo | ✅ |

---

## Requirement Traceability

| Requirement ID | Task(s)     |
| --------------- | ----------- |
| SCN-01           | T3          |
| SCN-02           | T3          |
| SCN-03           | T1, T2      |
| SCN-04           | T1, T2, T3, T4 |
| SCN-05           | T5          |
| SCN-06           | T6          |
| SCN-07           | T6          |
| SCN-08           | T7          |

**Coverage:** 8/8 requisitos mapeados.
