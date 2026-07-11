<p align="center">
  <a href="README.md">🇧🇷 Português</a> ·
  <a href="README.en.md">🇺🇸 English</a>
</p>

# copied

Gerenciador de histórico de clipboard pra **Pop!_OS 24.04 (COSMIC) / Wayland**. Roda como daemon em background, guarda os últimos itens copiados (texto e imagem), deixa fixar favoritos, categoriza automaticamente e abre um popup rápido pra buscar/colar de volta — sem terminal, via atalho de teclado.

Criei essa aplicação para treinar meu fluxo de desenvolvimento assistido por IA e ao mesmo tempo começar a estudar Rust já que sempre gostei de aprender desvendando aplicações reais, logo pensei, vou exercitar arquitetura agnóstica a linguagem e Rust em algo funcional. Busquei algo que ajudaria no meu dia a dia e assim lembrei que sou chato e não havia encontrado um gerenciador que me permitisse ter mais funcionalidades e me dói admitir mas parecido com o do windows. Ele está em construção e tem alguns gaps mas vou tentar ir atualizando (CLT faz no tempo livre). 

## O que faz 

- **Histórico** dos últimos 15 itens copiados (texto ou imagem), com deduplicação por hash.
- **Pins**: até 5 itens fixados, que não saem por LRU.
- **Categorização automática** do conteúdo copiado (heurística sobre o texto).
- **Popup sem decoração** (layer-shell, `zwlr_layer_shell_v1`) — abre por cima de tudo, sem taskbar, sem terminal.
- **Instância única**: reabrir o atalho com o popup já aberto fecha ele (toggle), não empilha janelas.
- **Aba de símbolos**: catálogo de símbolos matemáticos/ícones pra colar direto, sem passar pelo histórico.
- **Busca** por texto em ambas as abas (histórico e símbolos).
- Tudo local — sem rede, sem telemetria, sem conta.

## Como funciona

Dois binários, um workspace Cargo:

- **`copied-daemon`** — serviço `systemd --user` que fica escutando o clipboard via protocolo Wayland `zwlr_data_control_manager_v1`, mantém a pilha em memória e persiste em `~/.local/share/copied/stack.json` (mais cache de imagens em `~/.cache/copied/images/`). Serve um socket Unix (`$XDG_RUNTIME_DIR/copied.sock`) pra o cliente conversar com ele.
- **`copied`** (binário do crate `copied-gui`) — o popup gráfico. Conecta no socket do daemon, lista/filtra/copia/apaga/pina itens. A aba de símbolos escreve direto no clipboard (`wl-clipboard-rs`), sem envolver o daemon.

Protocolo entre os dois: NDJSON (uma linha JSON por mensagem) sobre o socket Unix, definido em `crates/copied-core` (única fonte de verdade do contrato).

Detalhes de arquitetura, fluxo de dados e decisões de design: [`.specs/codebase/ARCHITECTURE.md`](.specs/codebase/ARCHITECTURE.md).

## Requisitos

- **Pop!_OS 24.04** com **COSMIC** (depende do compositor `cosmic-comp` anunciar o protocolo `zwlr_data_control_manager_v1` — outros compositores wlroots *podem* funcionar, mas não são testados).
- **Rust** (toolchain `stable`, ver `rust-toolchain.toml`) com `cargo` no `PATH`. Se não tiver: [rustup.rs](https://rustup.rs).
- `systemd --user` disponível (padrão em qualquer sessão de usuário Linux com systemd).
- `sudo` disponível pra um único passo do instalador (grava `/etc/profile.d/copied-clipboard.sh`).

Não há dependências de sistema pra compilar (bibliotecas Wayland são carregadas em runtime via `dlopen`, não em build-time).

## Instalação

```bash
git clone <url-do-repositorio>
cd copied
./deploy/install.sh
```

O script é idempotente (seguro rodar de novo) e faz, em ordem:

1. **Build + instala os binários** via `cargo install` (`~/.cargo/bin/copied-daemon` e `~/.cargo/bin/copied`).
2. **Habilita o protocolo de clipboard Wayland** escrevendo `/etc/profile.d/copied-clipboard.sh` com `COSMIC_DATA_CONTROL_ENABLED=1` (pede `sudo`, confirma antes de rodar).
3. **Instala e ativa** o serviço `copied-daemon.service` como unit `systemd --user`.
4. **Imprime instruções** pra configurar o atalho de teclado (passo manual, ver abaixo).

⚠️ **Depois do passo 2, é preciso REINICIAR a máquina.** A variável só é lida pelo compositor COSMIC na inicialização da sessão — logout/login não é suficiente.

### Configurar o atalho de teclado

Em **COSMIC Settings → Keyboard → Custom Shortcuts**, crie um atalho que execute:

```
copied
```

Isso abre o popup diretamente (sem terminal). Rodar de novo com o popup aberto fecha ele.

### Verificar se está rodando

```bash
systemctl --user status copied-daemon.service
```

## Uso

Abra o popup pelo atalho configurado. Navegação só por teclado:

| Tecla | Ação |
|---|---|
| `↑` / `↓` | Navega a lista do histórico |
| `←` / `→` | Navega os símbolos (aba Símbolos) |
| `Tab` | Alterna foco (busca / abas / lista) |
| `Enter` | Copia o item selecionado pro clipboard e fecha |
| `p` ou `F2` | Fixa/desfixa (pin/unpin) o item selecionado |
| `d` ou `Delete` | Apaga o item selecionado |
| `Esc` | Fecha o popup |

Digite pra filtrar a lista atual (histórico ou símbolos) por texto.

## Limitações conhecidas

**Da instalação (não é totalmente "plug and play"):**

- O instalador assume `cargo` já instalado — se não tiver Rust, o script avisa mas não instala o toolchain pra você.
- O atalho de teclado precisa ser configurado manualmente na UI do COSMIC (não dá pra automatizar via script de forma confiável).
- A ativação do protocolo Wayland exige **reboot**, não só logout/login.

**Do software:**

- **Socket IPC sem autenticação**: qualquer processo rodando com o seu usuário pode se conectar no socket e listar/apagar/copiar itens do histórico (mitigado apenas pela permissão `0700` padrão de `$XDG_RUNTIME_DIR`).
- **Persistência não é atômica**: `stack.json` é sobrescrito direto (`fs::write`), sem escrita em arquivo temporário + rename. Se o processo for morto (`kill -9`, queda de energia) durante a escrita, o histórico inteiro pode ser perdido no próximo boot (o daemon detecta JSON inválido e reinicia vazio, em vez de crashar).
- **Sem limite de tamanho de conteúdo**: colar um texto ou imagem muito grande no clipboard não é truncado/rejeitado — pode inflar `stack.json` e o cache de imagens sem limite.

Detalhes completos: [`.specs/codebase/CONCERNS.md`](.specs/codebase/CONCERNS.md).

## Licença

[PolyForm Noncommercial 1.0.0](LICENSE) — uso pessoal, educacional e não-comercial livre. Ver `LICENSE` pra termos completos.
