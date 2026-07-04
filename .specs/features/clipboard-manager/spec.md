# Clipboard Manager (copied) Specification

## Problem Statement

O clipboard do sistema operacional guarda só o último item copiado — copiar algo novo destrói o anterior. Isso força re-trabalho constante (copiar de novo um valor perdido) durante tarefas que exigem colar vários trechos em sequência (código, comandos, textos). O usuário quer um histórico curto e acessível via teclado, sem sair do fluxo do terminal, rodando nativamente no Pop!_OS 24.04 (COSMIC/Wayland).

## Proposed Solution

Um daemon Rust roda em background (systemd user service), monitorando o clipboard do sistema via protocolo Wayland `ext-data-control`/`wlr-data-control`, e mantém uma pilha de até 15 itens (texto ou imagem) mais 5 pins separados, persistida em disco. Um atalho de teclado configurado no COSMIC abre um terminal com uma TUI (ratatui) que lista a pilha; o usuário navega com setas, copia um item de volta ao clipboard com Enter, exclui com Delete, ou pina/despina com uma tecla dedicada.

## Goals

- [ ] Nenhum conteúdo copiado nos últimos 15 usos (ou pinado) é perdido, mesmo entre reinicializações do sistema
- [ ] Acessar e reutilizar um item da pilha leva menos de 5 segundos (atalho → navegar → Enter)
- [ ] Daemon roda continuamente sem intervenção manual após o login

## Out of Scope

| Feature                                   | Reason                                                        |
| ------------------------------------------ | -------------------------------------------------------------- |
| Sincronização em nuvem entre máquinas      | Fora do escopo definido na entrevista (deferred idea)          |
| GUI gráfica (fora do terminal)             | Produto é deliberadamente uma TUI, não uma app gráfica          |
| Suporte a arquivos (URIs) e HTML rico      | Só texto e imagem estão no escopo capturado                     |
| Suporte a X11 puro (sem COSMIC/Wayland)    | Ambiente alvo é COSMIC no Pop!_OS 24.04, Wayland-only            |

---

## User Stories

### P1: Captura contínua de clipboard (texto) ⭐ MVP

**User Story**: Como usuário, quero que o daemon capture automaticamente todo texto que eu copiar, para que nada copiado se perca.

**Why P1**: Sem captura confiável em background, não há pilha pra visualizar — é a base de tudo.

**Acceptance Criteria**:

1. WHEN o usuário copia texto (Ctrl+C ou seleção) em qualquer aplicação THEN o daemon SHALL adicionar o conteúdo ao topo da pilha em até 1 segundo
2. WHEN o conteúdo copiado é idêntico a um item já existente na pilha (pinado ou não) THEN o daemon SHALL mover o item existente para o topo, sem criar duplicata
3. WHEN a pilha de não-pinados atinge 15 itens e um novo item é adicionado THEN o daemon SHALL remover o item mais antigo não-pinado (LRU)
4. WHEN o daemon inicia sem a variável de ambiente `COSMIC_DATA_CONTROL_ENABLED=1` setada THEN o daemon SHALL logar erro claro explicando a necessidade da flag e encerrar

**Independent Test**: Copiar 16 textos diferentes em sequência com o daemon rodando; verificar via socket/log que a pilha tem 15 itens e o primeiro copiado foi descartado.

---

### P1: Persistência da pilha em disco ⭐ MVP

**User Story**: Como usuário, quero que a pilha sobreviva a reinicializações do sistema ou crash do daemon, para não perder o histórico.

**Why P1**: Sem persistência, um reboot ou crash apaga tudo — inaceitável pra um histórico de clipboard.

**Acceptance Criteria**:

1. WHEN o daemon adiciona, remove, ou modifica (pin/unpin) um item THEN o daemon SHALL gravar o estado completo em `~/.local/share/copied/stack.json` antes de confirmar a operação ao cliente
2. WHEN o daemon reinicia THEN o daemon SHALL carregar a pilha e os pins salvos em `stack.json`, restaurando o estado anterior
3. WHEN `stack.json` está corrompido ou ilegível na inicialização THEN o daemon SHALL logar o erro e iniciar com pilha vazia, sem crashar

**Independent Test**: Adicionar itens, matar o processo do daemon (`kill`), reiniciar o daemon, confirmar via TUI que os itens persistiram.

---

### P1: Visualizar a pilha via TUI acionada por atalho ⭐ MVP

**User Story**: Como usuário, quero abrir a pilha com um atalho de teclado, para visualizar o histórico sem sair do fluxo de trabalho.

**Why P1**: É a interface principal — sem ela, a pilha capturada é inacessível.

**Acceptance Criteria**:

1. WHEN o usuário aciona o atalho configurado no COSMIC THEN o sistema SHALL abrir uma janela de terminal nova executando o cliente TUI
2. WHEN o cliente TUI conecta ao daemon THEN o cliente SHALL exibir a lista de itens pinados (topo, até 5) seguida dos não-pinados (até 15), mais recente primeiro
3. WHEN a pilha está vazia (sem itens e sem pins) THEN o cliente SHALL exibir uma mensagem de estado vazio, sem erro
4. WHEN o daemon não está rodando ou o socket é inacessível THEN o cliente SHALL exibir mensagem de erro clara ("daemon não encontrado — inicie o serviço") e encerrar sem travar
5. WHEN o usuário pressiona Esc THEN o cliente SHALL fechar a TUI sem executar nenhuma ação sobre a pilha

**Independent Test**: Com o daemon rodando e itens na pilha, disparar o atalho e verificar visualmente que a TUI abre, lista os itens corretos, e Esc fecha sem side-effects.

---

### P1: Copiar item da pilha de volta ao clipboard ⭐ MVP

**User Story**: Como usuário, quero selecionar um item da pilha e colocá-lo de volta no clipboard do sistema, para reutilizá-lo.

**Why P1**: É a ação central que resolve o problema original (reuso de conteúdo perdido).

**Acceptance Criteria**:

1. WHEN o usuário navega até um item com as setas e pressiona Enter THEN o cliente SHALL escrever o conteúdo daquele item no clipboard do sistema e fechar a TUI
2. WHEN o item copiado de volta é texto THEN o clipboard do sistema SHALL conter exatamente o texto original, sem alteração de encoding
3. WHEN o item copiado de volta é imagem THEN o clipboard do sistema SHALL conter os bytes da imagem original com o MIME type correto

**Independent Test**: Selecionar um item de texto conhecido, pressionar Enter, colar (Ctrl+V) em outro app, confirmar conteúdo idêntico.

---

### P1: Excluir item da pilha ⭐ MVP

**User Story**: Como usuário, quero remover um item específico da pilha, para limpar entradas que não preciso mais.

**Why P1**: Gerenciamento básico da pilha — sem exclusão, itens indesejados (ex: senha copiada por engano) ficam presos até serem empurrados pra fora pelo LRU.

**Acceptance Criteria**:

1. WHEN o usuário navega até um item e pressiona Delete THEN o cliente SHALL remover o item da pilha (via daemon) e atualizar a lista exibida imediatamente, sem fechar a TUI
2. WHEN o item excluído é uma imagem THEN o daemon SHALL também remover o arquivo correspondente do cache dir (`~/.cache/copied/images/`)
3. WHEN o item excluído está pinado THEN o daemon SHALL removê-lo tanto dos pins quanto liberar aquele slot de pin

**Independent Test**: Excluir um item pinado e um não-pinado; confirmar que ambos somem da TUI e que arquivos de imagem associados são removidos do cache.

---

### P1: Pinar e despinar item (até 5 pins) ⭐ MVP

**User Story**: Como usuário, quero pinar itens importantes, para que não sejam removidos pelo LRU da pilha de 15.

**Why P1**: Decisão explícita da entrevista — pins fazem parte do MVP junto com texto e imagem.

**Acceptance Criteria**:

1. WHEN o usuário pressiona a tecla dedicada de pin sobre um item não-pinado E existem menos de 5 pins THEN o daemon SHALL mover o item para a seção de pins, removendo-o da contagem dos 15 slots
2. WHEN o usuário pressiona a tecla dedicada de pin sobre um item não-pinado E já existem 5 pins THEN o cliente SHALL exibir mensagem "Máximo de 5 pins atingido — despine algo primeiro" e SHALL NOT alterar a pilha
3. WHEN o usuário pressiona a mesma tecla sobre um item já pinado THEN o daemon SHALL despiná-lo, devolvendo-o ao topo da pilha de não-pinados
4. WHEN a pilha de não-pinados está cheia (15 itens) e um pin é desfeito THEN o daemon SHALL aplicar a regra de LRU normalmente ao reinseri-lo (pode evictar o item mais antigo)

**Independent Test**: Pinar 5 itens, tentar pinar um 6º (deve rejeitar com mensagem), despinar um, pinar outro (deve funcionar).

---

### P1: Suporte a imagens no clipboard ⭐ MVP

**User Story**: Como usuário, quero que imagens copiadas (screenshots, PNGs) também entrem na pilha, não só texto.

**Why P1**: Decisão explícita da entrevista — imagem é MVP junto com texto e pins.

**Acceptance Criteria**:

1. WHEN o usuário copia uma imagem (PNG ou JPEG) THEN o daemon SHALL salvar os bytes em `~/.cache/copied/images/<hash>.<ext>` e adicionar uma entrada na pilha referenciando o path
2. WHEN a TUI lista um item de imagem THEN o cliente SHALL exibir um placeholder textual (ex: "[Imagem PNG, 240KB]") em vez do conteúdo bruto
3. WHEN o conteúdo copiado não é nem texto UTF-8 nem imagem PNG/JPEG reconhecida THEN o daemon SHALL ignorar o conteúdo (não adiciona à pilha), sem crashar

**Independent Test**: Copiar um screenshot (ex: via ferramenta de captura do COSMIC), confirmar que aparece na TUI como imagem, copiar de volta e colar num editor de imagem, confirmar bytes idênticos.

---

### P1: Daemon inicia automaticamente no login ⭐ MVP

**User Story**: Como usuário, quero que o daemon suba sozinho ao fazer login, para não precisar lembrar de iniciá-lo manualmente.

**Why P1**: Sem autostart, o produto não é utilizável no dia a dia — é justamente o cenário que a entrevista definiu como objetivo (fluxo sem fricção).

**Acceptance Criteria**:

1. WHEN o usuário faz login na sessão COSMIC THEN o systemd user service do daemon SHALL iniciar automaticamente (unit habilitada com `systemctl --user enable`)
2. WHEN o daemon crasha THEN o systemd SHALL reiniciá-lo automaticamente (política de restart configurada na unit)

**Independent Test**: Habilitar a unit, fazer logout/login (ou `systemctl --user restart` simulando boot), confirmar via `systemctl --user status` que o daemon está ativo.

---

## Edge Cases

- WHEN o daemon não consegue escrever em `~/.local/share/copied/` (permissão/disco cheio) THEN o daemon SHALL logar erro claro e continuar operando em memória (sem persistência) em vez de crashar
- WHEN dois clientes TUI tentam conectar ao mesmo tempo THEN o daemon SHALL aceitar ambas conexões e serializar as operações (sem corromper a pilha)
- WHEN o conteúdo de texto copiado excede um tamanho razoável (ex: >1MB, colagem de arquivo gigante) THEN o daemon SHALL truncar ou rejeitar a entrada, evitando inflar a pilha com lixo
- WHEN uma imagem copiada excede um tamanho razoável (ex: >20MB) THEN o daemon SHALL rejeitar a captura, logando aviso
- WHEN o item mais recente no topo da pilha é o mesmo que acabou de ser copiado (usuário copiou 2x seguidas) THEN o daemon SHALL tratar como no-op (dedup já cobre isso)

---

## Requirement Traceability

| Requirement ID | Story                                       | Phase  | Status  |
| --------------- | -------------------------------------------- | ------ | ------- |
| CLIP-01         | P1: Captura contínua de clipboard (texto)     | Design | Pending |
| CLIP-02         | P1: Persistência da pilha em disco            | Design | Pending |
| CLIP-03         | P1: Visualizar a pilha via TUI                | Design | Pending |
| CLIP-04         | P1: Copiar item da pilha ao clipboard         | Design | Pending |
| CLIP-05         | P1: Excluir item da pilha                     | Design | Pending |
| CLIP-06         | P1: Pinar e despinar item (até 5 pins)        | Design | Pending |
| CLIP-07         | P1: Suporte a imagens no clipboard            | Design | Pending |
| CLIP-08         | P1: Daemon inicia automaticamente no login    | Design | Pending |

**Coverage:** 8 total, 0 mapped to tasks, 8 unmapped ⚠️

---

## Success Criteria

- [ ] Copiar 16 textos em sequência resulta em pilha de 15 itens (mais antigo descartado), sem crash
- [ ] Reiniciar o daemon (ou o sistema) preserva pilha e pins intactos
- [ ] Atalho de teclado abre a TUI em menos de 1 segundo perceptível
- [ ] Copiar, excluir e pinar/despinar funcionam corretamente via TUI, incluindo rejeição do 6º pin
- [ ] Imagem copiada é capturada, listada, e recuperável byte-a-byte
- [ ] Daemon sobrevive a logout/login sem intervenção manual
