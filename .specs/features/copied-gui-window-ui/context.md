# copied-gui — Melhorias de janela e UI — Interview Decisions

**Date:** 2026-07-04
**Scope:** Melhorar a UI da janela `copied-gui` — centralização na tela, truncamento de texto com expansão apenas no item selecionado, lista em formato tabular com botões de ação alinhados à direita, e polimento visual geral. Não inclui os bugs de teclado/foco/terminal documentados em `.specs/features/copied-gui-fixes/context.md` (usuário confirmou nesta sessão que o problema de teclado já está resolvido — não investigado a fundo aqui, fora de escopo).
**Source:** Discussão informal + análise do código atual (`crates/copied-gui/src/app.rs`, `crates/copied-gui/src/main.rs`).

---

## Decisões

### Centralização da janela

- Trocar `anchor: Anchor::Top | Anchor::Right` (`main.rs:27`) por `Anchor::empty()` (nenhuma âncora), mantendo `size: Some((420, 480))`.
- **Rationale:** protocolo `zwlr_layer_shell_v1` não tem opção nativa de "center"; sem âncora e com `size` explícito, compositores baseados em wlroots (cosmic-comp incluso) centralizam a superfície no output por padrão — padrão usado por outros launchers Wayland (wofi, rofi-wayland).
- **Risco sinalizado:** não há documentação oficial confirmando esse comportamento especificamente no `cosmic-comp`; validar rodando. Se não centralizar como esperado, próximo passo seria investigar `margin` calculado a partir da resolução do `wl_output` (alternativa descartada nesta rodada por exigir consulta ao output em runtime).

### Truncamento de texto + expansão on-select

- Manter truncamento atual (`render_content`, ~60 chars) para itens não selecionados.
- Quando `state.selected == item.id`, expandir o bloco do item **in-place**: trocar o `text()` truncado por uma versão com `.wrap()` (multi-linha, sem limite de linhas), empurrando os itens abaixo na lista scrollável.
- **Rationale:** mais simples de implementar (estado `selected` já existe em `render_item`), não introduz widget flutuante (tooltip) nem painel fixo consumindo espaço vertical sempre.
- Descartado: tooltip (corta nas bordas da janela pequena) e painel de detalhe fixo (consome espaço mesmo sem truncamento).

### Layout tabular (lista de itens)

- `preview` com `Length::Fill` (ocupa o espaço restante).
- `category_button` + `pin_button` agrupados num `row` de largura fixa (ex.: `Length::Fixed(140.0)`), ficando alinhados à direita naturalmente pelo `Fill` do preview.
- Sem cabeçalho de tabela fixo.
- Botões continuam em texto (não viram ícones) — mantém consistência com `Category::Texto` etc. já sendo texto.

### Polimento visual geral (pacote fechado)

- **Tema:** trocar tema padrão (claro) por `iced::Theme::Dark` (ou tema pronto similar do iced) — combina com estética COSMIC/launcher popup.
- **Espaçamento:** `padding` consistente (8-12px) em `container`s; `spacing` entre itens da lista (hoje ausente).
- **Estado de seleção:** substituir o marcador textual `"> "` (`app.rs:371-375`) por `container::style` com background diferente no item selecionado — destaque visual real, não só prefixo textual.
- **Estado de hover:** `state.hovered` já existe (`app.rs:42,197-204`) mas não é usado visualmente — aplicar background sutil diferente do "selected" quando `hovered == Some(id)`.
- **Abas (Stack/Símbolos):** dar destaque visual à aba ativa (hoje `button("Stack")`/`button("Símbolos")` em `view()`, `app.rs:296-300`, são botões normais sem indicar estado ativo).

---

## Agent's Discretion

- Valor exato de `Length::Fixed` pra largura da coluna de botões (140.0 é ponto de partida, ajustar conforme render real).
- Escolha exata do tema (`Theme::Dark` vs outro tema pronto do iced), desde que combine com estética COSMIC.
- Detalhes exatos de cor/contraste pros estados de seleção e hover (desde que visualmente distintos entre si e do estado neutro).
- Se `Anchor::empty()` não centralizar corretamente no `cosmic-comp` em teste manual, investigar alternativa de `margin` calculado — decisão de fallback fica com o implementador, reportando de volta se for necessário.

---

## Deferred Ideas

- Nenhuma — todas as ideias trazidas nesta sessão (centralização, truncamento/expansão, layout tabular, polimento visual) foram encaixadas no escopo.

---

## Open Questions

- Nenhuma pendente — todas as áreas mapeadas foram resolvidas com decisão explícita do usuário.
