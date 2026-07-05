use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;

pub use copied_core::socket_path;
use copied_core::{Command, ItemKindView, ItemView, Response};

use crate::clipboard_write;
use crate::persistence;
use crate::stack::{CategoryError, Item, ItemKind, PinError, Stack, UnpinOutcome};

/// Estado compartilhado do daemon: a pilha em memória e onde persisti-la.
pub struct DaemonState {
    pub stack: Stack,
    pub stack_path: PathBuf,
}

impl DaemonState {
    pub fn new(stack: Stack, stack_path: PathBuf) -> Self {
        Self { stack, stack_path }
    }

    pub(crate) fn persist(&self) {
        if let Err(err) = persistence::save(&self.stack_path, &self.stack) {
            eprintln!(
                "copied-daemon: falha ao salvar {}: {err} (continuando em memória)",
                self.stack_path.display()
            );
        }
    }
}

/// Sobe o servidor: aceita conexões em loop, uma thread por conexão.
pub fn serve(socket_path: &Path, state: Arc<Mutex<DaemonState>>) -> io::Result<()> {
    if socket_path.exists() {
        std::fs::remove_file(socket_path)?;
    }
    let listener = UnixListener::bind(socket_path)?;
    for stream in listener.incoming() {
        let stream = match stream {
            Ok(s) => s,
            Err(err) => {
                eprintln!("copied-daemon: erro aceitando conexão: {err}");
                continue;
            }
        };
        let state = Arc::clone(&state);
        thread::spawn(move || handle_connection(stream, state));
    }
    Ok(())
}

fn handle_connection(stream: UnixStream, state: Arc<Mutex<DaemonState>>) {
    let reader_stream = match stream.try_clone() {
        Ok(s) => s,
        Err(err) => {
            eprintln!("copied-daemon: erro clonando conexão: {err}");
            return;
        }
    };
    let mut writer = stream;
    let reader = BufReader::new(reader_stream);

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(err) => {
                eprintln!("copied-daemon: erro lendo do socket: {err}");
                return;
            }
        };
        if line.trim().is_empty() {
            continue;
        }

        let response = match serde_json::from_str::<Command>(&line) {
            Ok(cmd) => {
                let mut guard = state
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                handle_command(&mut guard, cmd)
            }
            Err(err) => Response::Error {
                message: format!("comando inválido: {err}"),
            },
        };

        let Ok(json) = serde_json::to_string(&response) else {
            eprintln!("copied-daemon: falha serializando resposta");
            return;
        };
        if writer.write_all(json.as_bytes()).is_err() || writer.write_all(b"\n").is_err() {
            return;
        }
    }
}

fn handle_command(state: &mut DaemonState, cmd: Command) -> Response {
    match cmd {
        Command::List => Response::Items(list_views(&state.stack)),

        Command::Delete { id } => match state.stack.delete(id) {
            Some(removed) => {
                cleanup_image_if_any(&removed);
                state.persist();
                Response::Ack
            }
            None => not_found(),
        },

        Command::Pin { id } => match state.stack.pin(id) {
            Ok(()) => {
                state.persist();
                Response::Ack
            }
            Err(PinError::LimitReached) => Response::Error {
                message: "Máximo de 3 pins atingido — despine algo primeiro".into(),
            },
            Err(PinError::NotFound) => not_found(),
        },

        Command::Unpin { id } => match state.stack.unpin(id) {
            UnpinOutcome::Unpinned { evicted } => {
                if let Some(evicted) = &evicted {
                    cleanup_image_if_any(evicted);
                }
                state.persist();
                Response::Ack
            }
            UnpinOutcome::NotPinned => Response::Error {
                message: "item não está pinado".into(),
            },
        },

        Command::CopyToClipboard { id } => copy_to_clipboard(state, id),

        Command::GetImageBytes { id } => get_image_bytes(state, id),

        Command::SetCategory { id, category } => match state.stack.set_category(id, category) {
            Ok(()) => {
                state.persist();
                Response::Ack
            }
            Err(CategoryError::NotFound) => not_found(),
        },
    }
}

fn get_image_bytes(state: &DaemonState, id: copied_core::ItemId) -> Response {
    use base64::{engine::general_purpose::STANDARD, Engine as _};

    let Some(item) = state.stack.get(id) else {
        return not_found();
    };
    let ItemKind::Image { path, mime, .. } = &item.kind else {
        return Response::Error {
            message: "item não é uma imagem".into(),
        };
    };
    match std::fs::read(path) {
        Ok(bytes) => Response::ImageBytes {
            mime: mime.clone(),
            data_base64: STANDARD.encode(bytes),
        },
        Err(err) => Response::Error {
            message: format!("falha lendo arquivo de imagem: {err}"),
        },
    }
}

fn copy_to_clipboard(state: &DaemonState, id: copied_core::ItemId) -> Response {
    let Some(item) = state.stack.get(id) else {
        return not_found();
    };

    let result = match &item.kind {
        ItemKind::Text(text) => clipboard_write::write_text(text),
        ItemKind::Image { path, mime, .. } => match std::fs::read(path) {
            Ok(bytes) => clipboard_write::write_image(bytes, mime),
            Err(err) => {
                return Response::Error {
                    message: format!("falha lendo arquivo de imagem: {err}"),
                }
            }
        },
    };

    match result {
        Ok(()) => Response::Ack,
        Err(err) => Response::Error {
            message: format!("falha escrevendo no clipboard: {err}"),
        },
    }
}

fn cleanup_image_if_any(item: &Item) {
    if let ItemKind::Image { path, .. } = &item.kind {
        if let Err(err) = persistence::delete_image(path) {
            eprintln!(
                "copied-daemon: falha removendo imagem {}: {err}",
                path.display()
            );
        }
    }
}

fn not_found() -> Response {
    Response::Error {
        message: "item não encontrado".into(),
    }
}

fn list_views(stack: &Stack) -> Vec<ItemView> {
    let pins = stack.pins().map(|item| item_view(item, true));
    let items = stack.items().map(|item| item_view(item, false));
    pins.chain(items).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use copied_core::Category;

    fn test_state() -> (DaemonState, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let stack_path = dir.path().join("stack.json");
        (DaemonState::new(Stack::default(), stack_path), dir)
    }

    #[test]
    fn handle_get_image_bytes_returns_bytes_for_existing_image() {
        let (mut state, dir) = test_state();
        let bytes = b"fake-png-bytes";
        let path = dir.path().join("img.png");
        std::fs::write(&path, bytes).unwrap();
        let hash = crate::stack::content_hash(bytes);
        let item = Item::new_image(path, "image/png".into(), bytes.len() as u64, hash);
        let id = item.id;
        state.stack.insert(item);

        let response = handle_command(&mut state, Command::GetImageBytes { id });

        match response {
            Response::ImageBytes { mime, data_base64 } => {
                assert_eq!(mime, "image/png");
                use base64::{engine::general_purpose::STANDARD, Engine as _};
                assert_eq!(STANDARD.decode(data_base64).unwrap(), bytes);
            }
            other => panic!("esperava Response::ImageBytes, veio {other:?}"),
        }
    }

    #[test]
    fn handle_get_image_bytes_errors_when_file_missing() {
        let (mut state, dir) = test_state();
        let path = dir.path().join("missing.png");
        let hash = crate::stack::content_hash(b"x");
        let item = Item::new_image(path, "image/png".into(), 1, hash);
        let id = item.id;
        state.stack.insert(item);

        let response = handle_command(&mut state, Command::GetImageBytes { id });

        assert!(matches!(response, Response::Error { .. }));
    }

    #[test]
    fn handle_set_category_updates_item() {
        let (mut state, _dir) = test_state();
        let item = Item::new_text("hello".into());
        let id = item.id;
        state.stack.insert(item);

        let response = handle_command(
            &mut state,
            Command::SetCategory {
                id,
                category: Category::Url,
            },
        );

        assert_eq!(response, Response::Ack);
        assert_eq!(state.stack.get(id).unwrap().category, Category::Url);
    }

    #[test]
    fn handle_set_category_errors_on_unknown_id() {
        let (mut state, _dir) = test_state();

        let response = handle_command(
            &mut state,
            Command::SetCategory {
                id: copied_core::ItemId::new_v4(),
                category: Category::Url,
            },
        );

        assert!(matches!(response, Response::Error { .. }));
    }
}

fn item_view(item: &Item, pinned: bool) -> ItemView {
    let kind = match &item.kind {
        ItemKind::Text(text) => ItemKindView::Text {
            preview: text.chars().take(200).collect(),
        },
        ItemKind::Image {
            mime, size_bytes, ..
        } => ItemKindView::Image {
            mime: mime.clone(),
            size_bytes: *size_bytes,
        },
    };
    ItemView {
        id: item.id,
        kind,
        pinned,
        copied_at: item.copied_at,
        category: item.category,
    }
}
