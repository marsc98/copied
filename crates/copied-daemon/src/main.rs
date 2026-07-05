mod categorize;
mod clipboard_write;
mod ipc;
mod persistence;
mod stack;
mod watcher;

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

use copied_core::Category;
use ipc::DaemonState;
use watcher::ClipboardChange;

fn main() {
    let stack_path = stack_path();
    let cache_dir = image_cache_dir();

    let stack = persistence::load(&stack_path);
    let state = Arc::new(Mutex::new(DaemonState::new(stack, stack_path)));

    let (tx, rx) = mpsc::channel::<ClipboardChange>();

    thread::spawn(move || watcher::run(tx));

    let ipc_state = Arc::clone(&state);
    let socket_path = match ipc::socket_path() {
        Ok(path) => path,
        Err(err) => {
            eprintln!("copied-daemon: {err}");
            std::process::exit(1);
        }
    };
    thread::spawn(move || {
        if let Err(err) = ipc::serve(&socket_path, ipc_state) {
            eprintln!("copied-daemon: erro fatal no servidor IPC: {err}");
            std::process::exit(1);
        }
    });

    // Thread principal: dona do estado, consome as mudanças de clipboard
    // detectadas pelo watcher e persiste cada mutação.
    for change in rx {
        handle_clipboard_change(&state, &cache_dir, change);
    }
}

fn handle_clipboard_change(
    state: &Arc<Mutex<DaemonState>>,
    cache_dir: &Path,
    change: ClipboardChange,
) {
    let mut guard = state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    match change {
        ClipboardChange::Text(text) => {
            let category = categorize::detect(&text);
            if guard.stack.push_text(text) == stack::PushOutcome::Inserted {
                let id = guard.stack.items().next().map(|item| item.id);
                if let Some(id) = id {
                    let _ = guard.stack.set_category(id, category);
                }
            }
        }
        ClipboardChange::Image { bytes, mime } => {
            let hash = stack::content_hash(&bytes);
            if !guard.stack.touch(&hash) {
                match persistence::save_image(cache_dir, &hash, &bytes, &mime) {
                    Ok(path) => {
                        let mut item = stack::Item::new_image(path, mime, bytes.len() as u64, hash);
                        item.category = Category::Imagem;
                        if let Some(evicted) = guard.stack.insert(item) {
                            if let stack::ItemKind::Image { path, .. } = &evicted.kind {
                                if let Err(err) = persistence::delete_image(path) {
                                    eprintln!(
                                        "copied-daemon: falha removendo imagem evictada {}: {err}",
                                        path.display()
                                    );
                                }
                            }
                        }
                    }
                    Err(err) => {
                        eprintln!("copied-daemon: falha salvando imagem no cache: {err}");
                    }
                }
            }
        }
    }

    guard.persist();
}

fn stack_path() -> PathBuf {
    xdg_dir("XDG_DATA_HOME", ".local/share")
        .join("copied")
        .join("stack.json")
}

fn image_cache_dir() -> PathBuf {
    xdg_dir("XDG_CACHE_HOME", ".cache")
        .join("copied")
        .join("images")
}

fn xdg_dir(env_var: &str, fallback_from_home: &str) -> PathBuf {
    if let Some(dir) = std::env::var_os(env_var) {
        return PathBuf::from(dir);
    }
    let home = std::env::var_os("HOME").expect("HOME não está setada");
    PathBuf::from(home).join(fallback_from_home)
}
