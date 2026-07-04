use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::stack::{Item, Stack};

#[derive(Serialize, Deserialize)]
struct StackSnapshot {
    items: Vec<Item>,
    pins: Vec<Item>,
}

/// Carrega a pilha do disco. Se o arquivo não existir ou estiver corrompido,
/// loga o problema e retorna uma pilha vazia — nunca entra em pânico
/// (spec CLIP-02, edge case de stack.json corrompido).
pub fn load(path: &Path) -> Stack {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Stack::default(),
        Err(err) => {
            eprintln!(
                "copied-daemon: erro lendo {}: {err}, iniciando pilha vazia",
                path.display()
            );
            return Stack::default();
        }
    };

    match serde_json::from_str::<StackSnapshot>(&contents) {
        Ok(snapshot) => Stack::from_parts(snapshot.items.into(), snapshot.pins),
        Err(err) => {
            eprintln!(
                "copied-daemon: {} corrompido ({err}), iniciando pilha vazia",
                path.display()
            );
            Stack::default()
        }
    }
}

/// Grava a pilha inteira em disco, sobrescrevendo o arquivo.
pub fn save(path: &Path, stack: &Stack) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let snapshot = StackSnapshot {
        items: stack.items().cloned().collect(),
        pins: stack.pins().cloned().collect(),
    };
    let json = serde_json::to_string_pretty(&snapshot).map_err(io::Error::other)?;
    fs::write(path, json)
}

/// Salva os bytes de uma imagem no cache dir, nomeando o arquivo pelo hash
/// do conteúdo (mesmo hash usado pelo `stack` pra dedup — evita nomes
/// colidirem e permite localizar o arquivo por hash se necessário).
pub fn save_image(
    cache_dir: &Path,
    hash: &[u8; 32],
    bytes: &[u8],
    mime: &str,
) -> io::Result<PathBuf> {
    fs::create_dir_all(cache_dir)?;
    let ext = extension_for_mime(mime);
    let path = cache_dir.join(format!("{}.{ext}", hex_encode(hash)));
    fs::write(&path, bytes)?;
    Ok(path)
}

/// Remove o arquivo de imagem do cache. Arquivo já ausente não é erro
/// (idempotente — pode já ter sido removido por uma operação concorrente).
pub fn delete_image(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err),
    }
}

fn extension_for_mime(mime: &str) -> &'static str {
    match mime {
        "image/png" => "png",
        "image/jpeg" => "jpg",
        _ => "bin",
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stack::content_hash;

    #[test]
    fn load_returns_empty_stack_when_file_missing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("does-not-exist.json");

        let stack = load(&path);

        assert_eq!(stack.items().count(), 0);
        assert_eq!(stack.pins().count(), 0);
    }

    #[test]
    fn load_returns_empty_stack_when_file_corrupted() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("stack.json");
        fs::write(&path, b"{ isto nao e json valido").unwrap();

        let stack = load(&path);

        assert_eq!(stack.items().count(), 0);
    }

    #[test]
    fn save_then_load_roundtrips_text_and_image_items() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("stack.json");

        let mut stack = Stack::default();
        stack.push_text("hello".into());
        let hash = content_hash(b"fake-image-bytes");
        stack.insert(crate::stack::Item::new_image(
            PathBuf::from("/cache/img.png"),
            "image/png".into(),
            17,
            hash,
        ));
        let pinned_id = stack.items().last().unwrap().id;
        stack.pin(pinned_id).unwrap();

        save(&path, &stack).expect("save deveria funcionar");
        let loaded = load(&path);

        assert_eq!(loaded.items().count(), stack.items().count());
        assert_eq!(loaded.pins().count(), 1);
        assert_eq!(loaded.pins().next().unwrap().id, pinned_id);
    }

    #[test]
    fn save_image_names_file_by_hash_and_extension() {
        let dir = tempfile::tempdir().unwrap();
        let bytes = b"png-bytes";
        let hash = content_hash(bytes);

        let path = save_image(dir.path(), &hash, bytes, "image/png").unwrap();

        assert!(path.exists());
        assert!(path.to_string_lossy().ends_with(".png"));
        assert_eq!(fs::read(&path).unwrap(), bytes);
    }

    #[test]
    fn delete_image_removes_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let hash = content_hash(b"x");
        let path = save_image(dir.path(), &hash, b"x", "image/jpeg").unwrap();
        assert!(path.exists());

        delete_image(&path).expect("delete deveria funcionar");

        assert!(!path.exists());
    }

    #[test]
    fn delete_image_on_missing_file_is_ok() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nao-existe.png");

        delete_image(&path).expect("deveria ser idempotente, sem erro");
    }
}
