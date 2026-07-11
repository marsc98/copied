# copied-gui — Pins, categorias/ações, formatação e botões — Interview Decisions

**Date:** 2026-07-05
**Scope:** Ajustes na janela `copied-gui` (`crates/copied-gui/src/app.rs`, possivelmente `main.rs` pro tema): (1) seção de pins limitada a 3, acima da listagem, fora da contagem; (2) remoção do conceito de categoria da UI, troca do botão de categoria por delete, atalhos "p"/"d"; (3) numeração em negrito com separador "|" em negrito; (4) botões com borda mais arredondada, cor primária mais azulada, e indicação da tecla equivalente. Estendido durante a interview pra incluir navegação por teclado (↑↓←→ + Enter) na aba Símbolos, hoje inexistente.
**Relação com trabalho anterior:** Continuação de `.specs/features/copied-gui-window-ui/context.md` (centralização, truncamento, layout tabular, tema Dark, hover/seleção). Não relacionado a `.specs/features/copied-gui-fixes/context.md` (bugs de terminal/foco — outro escopo, ainda não investigado).
**Source:** Discussão informal (interview) + leitura de `crates/copied-gui/src/app.rs` (estado atual, já com `Focus`/`FocusNext`/`EnterPressed` de um fix de teclado em andamento não commitado) e `crates/copied-gui/src/main.rs`.

---

## Decisões

### 1. Pins

- Limite de **3 pins**. Ao tentar pinar um 4º (tecla "p" ou clique), a ação é bloqueada e uma mensagem de status aparece (ex.: "Limite de 3 pins atingido") — sem auto-despinar o mais antigo.
- Itens pinados aparecem **numa seção própria, acima da listagem normal**, **sem número** (só marcador/botões). A listagem numerada abaixo recomeça em 1 normalmente.
- Item pinado aparece **só na seção de pins** — não fica duplicado na listagem numerada de baixo.
- Quebra visual entre as duas seções: **espaçamento maior + linha divisória horizontal fina** (cor de borda do tema), não só espaçamento.
- **Rationale:** evita ambiguidade de contagem (pins não competem com a numeração "real") e deixa a hierarquia visual clara (pins = atalho rápido, lista = histórico).

### 2. Categorias, botão de delete, atalhos de teclado

- Remoção do conceito de categoria é **só na UI** (`app.rs`): remove `category_button`, `CategoryClicked`, `next_category`, `PendingAction::SetCategory` e o envio de `Command::SetCategory`. **Backend intocado** — `copied-core::Category`, `copied-daemon` (`categorize.rs`, `persistence.rs`, `stack.rs`, `ipc.rs`) continuam existindo e funcionando; o campo `category` do item simplesmente não é mais exibido/editável na GUI.
- O botão que antes era de categoria vira **botão de delete**, com label "Delete (d)". Delete é **direto, sem confirmação** — clicar já manda `Command::Delete { id }`, igual ao comportamento atual da tecla Delete/nova tecla "d".
- Tecla **"p"** (pin/unpin) e tecla **"d"** (delete) **somam-se** aos atalhos existentes (F2 pin/unpin, Delete deletar) — não os substituem. Todos continuam funcionando.
- "p"/"d" só têm efeito quando o **foco está na lista** (`Focus::List`, item selecionado) — se o foco está na busca (`Focus::Search`), digitar "p"/"d" escreve normalmente no campo, sem acionar pin/delete.
- Botão de pin ganha o mesmo tratamento de hint: label "Pin (p)" / "Unpin (p)" conforme estado.
- **Rationale:** manter F2/Delete evita quebrar hábito já existente; escopar "p"/"d" ao foco de lista evita o bug óbvio de não conseguir digitar essas letras na busca; remoção só-UI de categoria evita mexer em persistência/protocolo por uma mudança que é puramente de interface.

### 3. Numeração e separador

- Formato do item na listagem muda de `"1. conteúdo"` para **`"1 | conteúdo"`** — o ponto é substituído pelo pipe (não coexistem).
- **Número em negrito** e **pipe "|" em negrito**; conteúdo em peso normal.
- Como itens pinados não aparecem mais na listagem numerada (decisão da seção 1), o marcador textual `"[pin] "` que existia em `render_item` deixa de ser necessário ali (nenhum item da lista numerada estará pinado) — pode ser removido.
- **Nota de implementação:** `iced::widget::text` não faz negrito parcial dentro de uma única string: vai precisar compor um `row![text_bold("1"), text_bold(" | "), text(conteúdo)]` (ou `rich_text`/spans, se a versão do `iced` em uso — 0.14 — expuser isso) em vez de um único `text(format!(...))` como hoje.

### 4. Botões — borda, cor, hints de tecla

- **Border-radius moderado (~8px)** em todos os botões (tema/estilo customizado via `button::Style`, aplicado de forma consistente — tabs, pin, delete, símbolos).
- **Cor primária mais azulada, menos roxa**: trocar o preset `iced::Theme::Dark` (usado hoje em `main.rs:38`) por um tema customizado via `iced::Theme::custom`, sobrescrevendo só a cor primária do `Palette::DARK` para `#207f99`, mantendo o resto do palette (background, texto, success, danger) igual ao Dark.
- **Hints de tecla nos botões**: texto direto no label, formato `"Ação (tecla)"` — ex.: `"Pin (p)"`, `"Delete (d)"`. Sem badge/estilo separado para a letra.
- **Rationale:** mudança mínima de tema (só primary color) preserva todo o resto do polimento visual já decidido em `copied-gui-window-ui`; hint como texto simples evita compor widgets extras por botão.

### 5. Navegação por teclado na aba Símbolos (extensão de escopo trazida durante a interview)

- Símbolos ganham conceito de **seleção** (hoje só existem botões clicáveis, sem estado de seleção/destaque).
- **↑/↓ navega entre grupos** (ex.: pula de "Setas" pra "Matemática"); **←/→ navega entre símbolos dentro do grupo atual**.
- Ao entrar na aba Símbolos, o **primeiro símbolo do primeiro grupo já vem pré-selecionado**, com o mesmo destaque visual (`item_style`/cor primária) usado na seleção da lista de stack.
- **Enter** no símbolo selecionado copia e fecha o popup — mesmo efeito de `SymbolClicked` (clique).
- **Rationale:** consistência de UX entre as duas abas (sempre há uma seleção visível e navegável por teclado); modelo 2D (grupo × posição no grupo) reflete o layout real (grupos empilhados, símbolos em linha dentro de cada grupo), evitando pular "errado" entre grupos de tamanhos diferentes.

---

## Agent's Discretion

- Texto exato da mensagem de status ao atingir o limite de 3 pins (ex.: "Limite de 3 pins atingido.") — só precisa deixar claro que o limite foi alcançado.
- Espessura/estilo exato da linha divisória entre pins e lista (usar `container::Style`/`border` consistente com o resto do visual já decidido em `copied-gui-window-ui`).
- Estrutura de estado pra seleção 2D nos Símbolos (ex.: `selected_group: usize, selected_index: usize` vs. outra representação) — desde que ↑↓←→ e Enter se comportem como decidido.
- Limpeza de código morto resultante da remoção de categoria na GUI (import de `Category` se ficar sem uso direto em `app.rs`, etc.) — não é uma decisão de produto, é higiene de implementação.
- Largura exata da coluna de botões de ação (pin + delete) no `render_item`, ajustando o `Length::Fixed` atual (150.0) se o novo hint de texto ("Pin (p)"/"Delete (d)") não couber bem.

---

## Deferred Ideas

- Remoção completa do conceito de `Category` do sistema (backend/persistência/`copied-daemon`) — descartado nesta rodada; decisão explícita foi manter só a remoção na UI. Se um dia isso for revisitado, merece seu próprio `/design` (mexe em protocolo IPC e dados já persistidos).

---

## Open Questions

- Nenhuma pendente — todas as áreas mapeadas (incluindo a extensão de Símbolos) foram resolvidas com decisão explícita do usuário.
