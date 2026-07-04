use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type ItemId = Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "action")]
pub enum Command {
    List,
    CopyToClipboard { id: ItemId },
    Delete { id: ItemId },
    Pin { id: ItemId },
    Unpin { id: ItemId },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "result", content = "data")]
pub enum Response {
    Items(Vec<ItemView>),
    Ack,
    Error { message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ItemView {
    pub id: ItemId,
    pub kind: ItemKindView,
    pub pinned: bool,
    pub copied_at: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum ItemKindView {
    Text { preview: String },
    Image { mime: String, size_bytes: u64 },
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
    fn item_view_image_roundtrips() {
        let item = ItemView {
            id: Uuid::new_v4(),
            kind: ItemKindView::Image {
                mime: "image/png".into(),
                size_bytes: 12345,
            },
            pinned: true,
            copied_at: SystemTime::now(),
        };
        roundtrip(item);
    }
}
