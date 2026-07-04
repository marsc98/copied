use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type ItemId = Uuid;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "value")]
pub enum Category {
    Texto,
    Url,
    Codigo,
    Imagem,
    #[default]
    Outro,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "action")]
pub enum Command {
    List,
    CopyToClipboard { id: ItemId },
    Delete { id: ItemId },
    Pin { id: ItemId },
    Unpin { id: ItemId },
    GetImageBytes { id: ItemId },
    SetCategory { id: ItemId, category: Category },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "result", content = "data")]
pub enum Response {
    Items(Vec<ItemView>),
    Ack,
    Error { message: String },
    ImageBytes { mime: String, data_base64: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ItemView {
    pub id: ItemId,
    pub kind: ItemKindView,
    pub pinned: bool,
    pub copied_at: SystemTime,
    pub category: Category,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum ItemKindView {
    Text { preview: String },
    Image { mime: String, size_bytes: u64 },
}

/// `$XDG_RUNTIME_DIR/copied.sock` — convenção compartilhada entre daemon e clientes.
pub fn socket_path() -> io::Result<PathBuf> {
    let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR").ok_or_else(|| {
        io::Error::new(
            ErrorKind::NotFound,
            "XDG_RUNTIME_DIR não está setada; não é possível localizar o socket do daemon",
        )
    })?;
    Ok(Path::new(&runtime_dir).join("copied.sock"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip<T: Serialize + for<'de> Deserialize<'de> + PartialEq + std::fmt::Debug>(value: T) {
        let json = serde_json::to_string(&value).expect("serialize");
        let back: T = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(value, back);
    }

    #[test]
    fn command_list_roundtrips() {
        roundtrip(Command::List);
    }

    #[test]
    fn command_copy_to_clipboard_roundtrips() {
        roundtrip(Command::CopyToClipboard { id: Uuid::new_v4() });
    }

    #[test]
    fn command_delete_roundtrips() {
        roundtrip(Command::Delete { id: Uuid::new_v4() });
    }

    #[test]
    fn command_pin_roundtrips() {
        roundtrip(Command::Pin { id: Uuid::new_v4() });
    }

    #[test]
    fn command_unpin_roundtrips() {
        roundtrip(Command::Unpin { id: Uuid::new_v4() });
    }

    #[test]
    fn response_items_roundtrips() {
        let item = ItemView {
            id: Uuid::new_v4(),
            kind: ItemKindView::Text {
                preview: "hello".into(),
            },
            pinned: false,
            copied_at: SystemTime::now(),
            category: Category::Texto,
        };
        roundtrip(Response::Items(vec![item]));
    }

    #[test]
    fn response_ack_roundtrips() {
        roundtrip(Response::Ack);
    }

    #[test]
    fn response_error_roundtrips() {
        roundtrip(Response::Error {
            message: "Máximo de 5 pins atingido".into(),
        });
    }

    #[test]
    fn category_default_roundtrips() {
        roundtrip(Category::default());
        assert_eq!(Category::default(), Category::Outro);
    }

    #[test]
    fn command_get_image_bytes_roundtrips() {
        roundtrip(Command::GetImageBytes { id: Uuid::new_v4() });
    }

    #[test]
    fn command_set_category_roundtrips() {
        roundtrip(Command::SetCategory {
            id: Uuid::new_v4(),
            category: Category::Url,
        });
    }

    #[test]
    fn response_image_bytes_roundtrips() {
        roundtrip(Response::ImageBytes {
            mime: "image/png".into(),
            data_base64: "iVBORw0KGgo=".into(),
        });
    }

    #[test]
    fn item_view_image_roundtrips() {
        let item = ItemView {
            id: Uuid::new_v4(),
            kind: ItemKindView::Image {
                mime: "image/png".into(),
                size_bytes: 12345,
            },
            pinned: true,
            copied_at: SystemTime::now(),
            category: Category::Imagem,
        };
        roundtrip(item);
    }
}
