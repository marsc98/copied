# copied-gui Specification

## Problem Statement

O cliente atual do `copied` (`copied-cli`, ratatui/crossterm) abre um terminal com TUI toda vez que o atalho é acionado. Isso limita o app a uma lista textual navegável só por teclado — sem preview visual de imagem, sem mouse, sem busca, sem organização por categoria. O objetivo inicial de estudar Rust também motiva evoluir pra uma UI gráfica de verdade em vez de continuar restrito a um terminal.

## Proposed Solution

Substituir `copied-cli` por um novo crate, `copied-gui`, que abre um popup gráfico (via `iced` + `iced_layershell`, protocolo `zwlr_layer_shell_v1`) no lugar do terminal, mantendo a mesma forma de invocação (atalho de teclado). A janela replica tudo que o TUI faz hoje (listar, navegar, copiar, excluir, pin/unpin) e adiciona busca, mouse, preview de imagem inline, categorização e uma aba de símbolos — sem exigir async runtime no daemon nem quebrar o isolamento client/daemon via socket já estabelecido.

## Goals

- [ ] `copied-gui` cobre 100% das ações que `copied-cli` cobre hoje (listar, navegar, copiar, excluir, pin/unpin), sem regressão.
- [ ] `copied-cli`, `ratatui` e `crossterm` são removidos do workspace (`Cargo.toml` members e deps) após `copied-gui` estar funcional.
- [ ] Busca/filtro funciona sem exigir mudança de protocolo IPC.
- [ ] Preview de imagem e categorias funcionam via extensão do protocolo IPC (`copied-core`), preservando o isolamento onde o cliente nunca acessa diretamente os diretórios XDG do daemon.

## Out of Scope

| Feature | Reason |
| --- | --- |
| Modo `--tui` (ratatui mantido em paralelo) | Decidido na entrevista: "manter a CLI" = forma de invocação, não a interface visual. TUI é aposentado, não mantido como alternativa. |
| Auto-type / inserção direta no app focado (input simulation) | Decidido na entrevista: complexidade desproporcional em Wayland (protocolo virtual-keyboard, devolução de foco). Ação de seleção permanece copy-and-close. |
| Tags múltiplas por item | Decidido na entrevista: categoria é campo único editável, não lista de tags (menor superfície de protocolo/domínio). |
| Sincronização/persistência dos símbolos via daemon | Decidido na entrevista: catálogo de símbolos é estático e embutido no binário, não passa por `stack.json`/IPC. |

---

## User Stories

### P1: Popup gráfico substitui o terminal ⭐ MVP

**User Story**: Como usuário do `copied`, quero que o atalho de teclado abra um popup gráfico em vez de um terminal com TUI, pra ter uma base visual estilizável em Rust puro.

**Why P1**: Sem isso não existe `copied-gui` — é o alicerce técnico (iced + layer-shell + IPC) sobre o qual todo o resto é construído.

**Acceptance Criteria**:

1. WHEN o atalho é acionado THEN o sistema SHALL abrir uma janela overlay sem decoração (via `zwlr_layer_shell_v1`/`iced_layershell`), sem aparecer em taskbar/switcher.
2. WHEN o atalho é acionado com o popup já aberto na tela THEN o sistema SHALL fechar a instância existente (toggle) em vez de abrir uma segunda janela.
3. WHEN a janela abre THEN o sistema SHALL enviar `Command::List` ao daemon e renderizar os itens retornados, na mesma ordem que o TUI usa hoje (pins primeiro, depois itens por recência).
4. WHEN a pilha está vazia THEN a janela SHALL mostrar um estado vazio equivalente ao atual ("Pilha vazia — copie algo pra começar").
5. WHEN o usuário pressiona Esc, ou o popup perde foco THEN o sistema SHALL fechar a janela.
6. WHEN o daemon não está rodando ou o socket (`$XDG_RUNTIME_DIR/copied.sock`) não responde THEN a janela SHALL exibir uma mensagem de erro em vez de travar ou fechar silenciosamente.

**Independent Test**: Acionar o atalho com a pilha populada e vazia; verificar que o popup abre sem decoração, some da lista de janelas do compositor, e fecha com Esc/foco perdido/segundo acionamento do atalho.

---

### P1: Paridade de ações (teclado + mouse) ⭐ MVP

**User Story**: Como usuário, quero copiar, excluir e pin/unpin itens usando teclado ou mouse na nova janela, pra manter o mesmo fluxo de hoje sem regressão, agora também com clique.

**Why P1**: É a paridade funcional com o TUI atual — sem isso `copied-gui` não pode substituir `copied-cli`.

**Acceptance Criteria**:

1. WHEN o usuário navega com ↑/↓ (teclado) ou passa o mouse sobre um item (hover) THEN o sistema SHALL destacar visualmente o item correspondente como selecionado.
2. WHEN o usuário pressiona Enter ou clica num item THEN o sistema SHALL enviar `Command::CopyToClipboard { id }`, e em caso de `Response::Ack` SHALL fechar a janela.
3. WHEN o usuário pressiona Delete (teclado) ou aciona a ação de excluir por mouse THEN o sistema SHALL enviar `Command::Delete { id }` e, em caso de `Response::Ack`, atualizar a lista sem o item.
4. WHEN o usuário aciona pin/unpin (tecla equivalente ao F2 atual, ou clique no controle correspondente) THEN o sistema SHALL enviar `Command::Pin`/`Command::Unpin { id }` e atualizar a lista.
5. WHEN o daemon retorna `Response::Error { message }` (ex: limite de 5 pins atingido, item não encontrado) THEN a janela SHALL exibir a mensagem de erro visível, sem fechar.
6. WHEN o usuário faz scroll do mouse sobre a lista THEN o sistema SHALL rolar a lista (equivalente à navegação por teclado além dos limites visíveis).

**Independent Test**: Repetir o fluxo completo de copiar/excluir/pin/unpin usando só teclado, depois só mouse, comparando contra o comportamento hoje documentado em `copied-cli/src/app.rs`.

---

### P1: Busca/filtro por texto ⭐ MVP

**User Story**: Como usuário, quero filtrar a lista digitando um texto de busca, pra achar rapidamente um item quando a pilha tem vários itens parecidos.

**Why P1**: Apontada na entrevista como maior ganho de UX, e não exige mudança de protocolo (filtro client-side sobre `Vec<ItemView>` já carregado) — sem risco arquitetural adicional ao MVP.

**Acceptance Criteria**:

1. WHEN o usuário digita num campo de busca THEN o sistema SHALL filtrar a lista exibida pra mostrar só itens de texto cujo `preview` contenha a substring digitada (case-insensitive), sem nova chamada IPC.
2. WHEN o campo de busca está vazio THEN o sistema SHALL exibir todos os itens (comportamento atual).
3. WHEN nenhum item corresponde ao texto digitado THEN o sistema SHALL exibir um estado vazio de busca (distinto do estado "pilha vazia").
4. WHEN o usuário limpa o campo de busca (Esc no campo, ou botão de limpar) THEN o sistema SHALL restaurar a lista completa sem fechar a janela.

**Independent Test**: Popular a pilha com itens variados, digitar termos que combinam com 0, 1 e N itens, verificar a lista filtrada em cada caso.

---

### P2: Preview de imagem inline

**User Story**: Como usuário, quero ver uma miniatura real da imagem copiada em vez de só o texto "[Imagem PNG, 45KB]", pra reconhecer visualmente qual print está na pilha.

**Why P2**: Ganho de UX real, mas exige extensão de protocolo (`Command::GetImageBytes`/`Response::ImageBytes`) — maior superfície de mudança que P1, então entra depois da base estar validada.

**Acceptance Criteria**:

1. WHEN a lista contém um item de imagem THEN o cliente SHALL enviar `Command::GetImageBytes { id }` e, em caso de `Response::ImageBytes { mime, bytes }`, renderizar uma miniatura decodificada no lugar do texto `[Imagem MIME, KB]`.
2. WHEN o daemon responde `Response::Error` a um `GetImageBytes` (ex: arquivo de cache ausente/removido) THEN a lista SHALL cair de volta pro fallback textual atual (`[Imagem MIME, KB]`) em vez de quebrar a UI ou remover o item.
3. WHEN os bytes recebidos não decodificam como imagem válida (arquivo corrompido) THEN o cliente SHALL logar o erro e usar o mesmo fallback textual, sem panic.
4. WHEN um item de imagem é excluído/pinado/copiado THEN o comportamento SHALL ser idêntico ao de um item de texto (o preview não muda o fluxo de ações).

**Independent Test**: Copiar uma imagem real pro clipboard, abrir o popup, verificar a miniatura; simular arquivo de cache ausente (remover manualmente) e verificar fallback textual sem crash.

---

### P2: Categorização automática e editável

**User Story**: Como usuário, quero que cada item tenha uma categoria detectada automaticamente (Texto/URL/Código/Imagem/Outro) que eu possa sobrescrever, pra organizar visualmente a pilha.

**Why P2**: Estrutural — exige mudança em `Item`/`ItemView`, novo comando IPC e migração de `stack.json` existente. Mais arriscado que P1, mas menos urgente que preview de imagem pela ordem de prioridade dada na entrevista.

**Acceptance Criteria**:

1. WHEN um item de texto é capturado pelo watcher THEN o sistema SHALL detectar automaticamente uma categoria por heurística de conteúdo (ex: parece URL, parece código) e atribuí-la ao item; itens de imagem SHALL receber categoria `Imagem` automaticamente.
2. WHEN nenhuma heurística corresponde THEN o sistema SHALL atribuir a categoria default `Outro`.
3. WHEN o usuário aciona a ação de editar categoria num item THEN o cliente SHALL enviar `Command::SetCategory { id, category }` e, em caso de `Response::Ack`, atualizar a categoria exibida.
4. WHEN o daemon carrega um `stack.json` gravado antes desta feature (sem campo `category`) THEN o sistema SHALL aplicar a categoria default (`Outro`) a esses itens sem erro de deserialização e sem perder os demais campos.
5. WHEN a lista é exibida THEN a categoria de cada item SHALL ser visível (rótulo/ícone), permitindo agrupamento ou filtro visual por categoria.

**Independent Test**: Copiar uma URL, um trecho de código e um texto genérico; verificar categorias auto-atribuídas; editar uma manualmente; reiniciar o daemon com um `stack.json` antigo (sem `category`) e verificar que carrega sem erro.

---

### P3: Aba de símbolos

**User Story**: Como usuário, quero uma aba com símbolos (matemática, ícones) que eu possa copiar rapidamente, pra não precisar procurar em outro lugar enquanto digito.

**Why P3**: Aditiva e isolada — catálogo estático embutido no binário, sem tocar protocolo/daemon. Não bloqueia nem é bloqueada pelas demais stories; pode ser cortada sem impacto se o tempo apertar.

**Acceptance Criteria**:

1. WHEN o usuário troca pra aba de símbolos THEN o sistema SHALL exibir um catálogo estático de símbolos (matemática, ícones) embutido no binário `copied-gui`, sem chamada IPC.
2. WHEN o usuário seleciona (clique ou Enter) um símbolo THEN o sistema SHALL copiar o símbolo pro clipboard (via `wl-clipboard-rs`, direto, sem passar pelo daemon) e fechar a janela — mesmo comportamento de copy-and-close das demais stories.
3. WHEN a aba de símbolos está ativa THEN a busca por texto (P1) SHALL também filtrar os símbolos exibidos, reaproveitando o mesmo campo de busca.

**Independent Test**: Abrir a aba de símbolos, filtrar por um termo, selecionar um símbolo, verificar que foi parar no clipboard e a janela fechou.

---

## Edge Cases

- WHEN o atalho é acionado repetidamente em sequência rápida (antes do primeiro popup terminar de abrir) THEN o sistema SHALL tratar como toggle único, sem condição de corrida abrindo múltiplas janelas.
- WHEN a conexão IPC cai no meio de uma ação (ex: daemon reiniciado enquanto popup está aberto) THEN a janela SHALL exibir erro e permitir fechar, sem travar aguardando resposta indefinidamente.
- WHEN o `Response::ImageBytes` retorna um payload muito grande (imagem grande) THEN o sistema SHALL exibir a miniatura sem bloquear a UI (decodificação fora da thread de render, se necessário).
- WHEN um item some da lista entre a seleção e a ação (deletado por outra sessão/conexão concorrente) THEN o daemon SHALL responder `Response::Error` (`item não encontrado`, já implementado) e a UI SHALL refletir isso sem crash.

---

## Requirement Traceability

| Requirement ID | Story | Phase | Status |
| --- | --- | --- | --- |
| GUI-01 | P1: Popup gráfico substitui o terminal | Design | Pending |
| GUI-02 | P1: Popup gráfico substitui o terminal (toggle) | Design | Pending |
| GUI-03 | P1: Paridade de ações (teclado + mouse) | Design | Pending |
| GUI-04 | P1: Busca/filtro por texto | Design | Pending |
| GUI-05 | P2: Preview de imagem inline (`Command::GetImageBytes`) | Design | Pending |
| GUI-06 | P2: Categorização automática e editável (`Command::SetCategory`) | Design | Pending |
| GUI-07 | P2: Migração de `stack.json` sem campo `category` | Design | Pending |
| GUI-08 | P3: Aba de símbolos | Design | Pending |

**Coverage:** 8 total, 0 mapped to tasks, 8 unmapped ⚠️

---

## Success Criteria

- [ ] `copied-gui` cobre list/copy/delete/pin/unpin com paridade total em relação a `copied-cli`, validado manualmente (sem framework E2E, conforme `TESTING.md`).
- [ ] `copied-cli`, `ratatui` e `crossterm` removidos do `Cargo.toml` do workspace.
- [ ] Busca funcional sem nova mensagem IPC.
- [ ] `cargo test` cobre: detecção automática de categoria (heurísticas), migração de `stack.json` sem `category`, roundtrip de `Command::GetImageBytes`/`Command::SetCategory` em `copied-core`.
- [ ] `cargo clippy -D warnings` e `cargo fmt --check` passam no workspace após a migração.
