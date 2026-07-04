# Code Conventions

**Analyzed:** 2026-07-04 — extraído de 10 arquivos `.rs` reais (1630 linhas totais).

## Naming Conventions

**Files:** `snake_case`, um módulo por arquivo, nome = responsabilidade técnica (`stack.rs`, `persistence.rs`, `ipc.rs`, `watcher.rs`, `clipboard_write.rs`, `ipc_client.rs`, `app.rs`).

**Structs/Enums:** `PascalCase`. Sufixo `View` para tipos de transporte IPC que espelham um tipo de domínio interno (`Item` → `ItemView`, `ItemKind` → `ItemKindView`) — separa o modelo interno (com `PathBuf`, hash bruto) do modelo exposto ao cliente (preview truncado, sem paths de filesystem).
Sufixo `Outcome`/`Error` para tipos de retorno que carregam mais que sucesso/falha (`PushOutcome`, `UnpinOutcome`, `PinError`).

**Functions/Methods:** `snake_case`, verbo no imperativo (`push_text`, `content_hash`, `delete_image`, `handle_clipboard_change`). Construtores alternativos como `new_text`/`new_image` em vez de builder genérico.

**Constants:** `SCREAMING_SNAKE_CASE` (`MAX_ITEMS`, `MAX_PINS`, `TEXT_MIME_TYPES`, `RECONNECT_BACKOFFS_SECS`), sempre com `pub const` quando cruzam módulo (`stack::MAX_ITEMS` usado em `ipc.rs`).

**Variables:** nomes descritivos completos, sem abreviação além de convenções aceitas no ecossistema (`tx`/`rx` para mpsc, `qh` para `QueueHandle`, `guard` para `MutexGuard`).

## Code Organization

**Import ordering:** externo (`std`) → crates externas → crates do workspace (`copied_core`) → `crate::` (módulos locais). Exemplo em `ipc.rs:1-11`.

**File structure interna:** tipos públicos e construtores no topo → funções de orquestração → helpers privados no fim → `#[cfg(test)] mod tests` sempre por último, no mesmo arquivo (nunca `*_test.rs` separado).

**Enums serializáveis:** `#[serde(tag = "...")]` explícito em todo enum que cruza o socket IPC (`Command`, `Response`, `ItemKindView`) — nunca serialização default sem tag.

## Type Safety/Documentation

**Approach:** Tipagem forte com newtypes leves (`ItemId = Uuid`) em vez de `String`/`u64` cru para identificadores. Sem `unsafe` em nenhum arquivo. Zero uso de `dyn Any`/`Box<dyn Error>` genérico — erros são enums explícitos por módulo (`WatchError`, `PinError`) com `impl fmt::Display` manual quando o erro cruza um limite de módulo público.

**Comentários doc (`///`, `//!`):** presentes só em itens públicos de crate/módulo com comportamento não-óbvio (ex: `watcher.rs:1-14` explica por que `wl-clipboard-rs` não é usado ali; `stack.rs:166-169` documenta um `SPEC_DEVIATION` explícito). Funções privadas triviais não têm doc comment.

## Error Handling

**Pattern:** Nunca `panic!`/`unwrap()` em caminho de execução normal do daemon — logar via `eprintln!` com prefixo `"copied-daemon: "` e degradar (pilha vazia, memória sem persistência, `continue` no loop) em vez de crashar. Ver `persistence::load` (arquivo corrompido → pilha vazia) e `ipc::serve` (erro numa conexão → `continue`, não derruba o listener).
Erros de biblioteca externa são envolvidos em enum próprio do módulo (`WatchError` em `watcher.rs`) em vez de propagados crus, pra permitir `Display` customizado.
`.expect()` só aparece (3 ocorrências fora de teste) sobre invariante garantido linhas antes por `position()`/`find()`, ou falha de setup irrecuperável (`HOME` não setada).

## Comments/Documentation

**Style:** Comentários de linha (`//`) explicam o **porquê**, nunca o **o quê** — ex: `main.rs:41-42` explica por que a thread principal é dona do estado, não repete o código. Nenhum comentário do tipo "seta x para 1". Idioma dos comentários e mensagens de erro/log: **português** (mensagens de usuário e `eprintln!`), enquanto identificadores e doc comments de itens públicos de biblioteca (`watcher.rs`) usam **inglês** — a mistura segue a audiência (usuário final vs. leitor de API).
