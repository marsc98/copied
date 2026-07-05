use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::SystemTime;

use copied_core::{Category, ItemId};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub const MAX_ITEMS: usize = 15;
pub const MAX_PINS: usize = 3;

pub fn content_hash(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher.finalize().into()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ItemKind {
    Text(String),
    Image {
        path: PathBuf,
        mime: String,
        size_bytes: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Item {
    pub id: ItemId,
    pub kind: ItemKind,
    pub content_hash: [u8; 32],
    pub copied_at: SystemTime,
    #[serde(default)]
    pub category: Category,
}

impl Item {
    pub fn new_text(content: String) -> Self {
        let hash = content_hash(content.as_bytes());
        Self {
            id: Uuid::new_v4(),
            kind: ItemKind::Text(content),
            content_hash: hash,
            copied_at: SystemTime::now(),
            category: Category::default(),
        }
    }

    pub fn new_image(path: PathBuf, mime: String, size_bytes: u64, content_hash: [u8; 32]) -> Self {
        Self {
            id: Uuid::new_v4(),
            kind: ItemKind::Image {
                path,
                mime,
                size_bytes,
            },
            content_hash,
            copied_at: SystemTime::now(),
            category: Category::default(),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum PinError {
    NotFound,
    LimitReached,
}

#[derive(Debug, PartialEq, Eq)]
pub enum CategoryError {
    NotFound,
}

#[derive(Debug, PartialEq)]
pub enum PushOutcome {
    Inserted,
    Touched,
}

#[derive(Debug, Default)]
pub struct Stack {
    items: VecDeque<Item>,
    pins: Vec<Item>,
}

impl Stack {
    /// Reconstrói um `Stack` a partir de itens já carregados (ex: do disco).
    /// Quem chama garante os invariantes (MAX_ITEMS/MAX_PINS) — usado só por
    /// `persistence::load`, que grava exatamente o que este mesmo módulo salvou.
    pub fn from_parts(items: VecDeque<Item>, pins: Vec<Item>) -> Self {
        Self { items, pins }
    }

    pub fn items(&self) -> impl Iterator<Item = &Item> {
        self.items.iter()
    }

    pub fn pins(&self) -> impl Iterator<Item = &Item> {
        self.pins.iter()
    }

    /// Busca um item (pinado ou não) por id, sem mutar a pilha — usado pra
    /// resolver o conteúdo na hora de copiar de volta pro clipboard.
    pub fn get(&self, id: ItemId) -> Option<&Item> {
        self.items
            .iter()
            .find(|i| i.id == id)
            .or_else(|| self.pins.iter().find(|i| i.id == id))
    }

    /// Se um item (pinado ou não) já tem esse hash, atualiza `copied_at` e,
    /// se não-pinado, move pro topo. Retorna true se encontrou.
    pub fn touch(&mut self, hash: &[u8; 32]) -> bool {
        if let Some(pos) = self.items.iter().position(|i| &i.content_hash == hash) {
            let mut item = self.items.remove(pos).expect("position just found");
            item.copied_at = SystemTime::now();
            self.items.push_front(item);
            return true;
        }
        if let Some(item) = self.pins.iter_mut().find(|i| &i.content_hash == hash) {
            item.copied_at = SystemTime::now();
            return true;
        }
        false
    }

    /// Insere um item novo no topo dos não-pinados. Retorna o item evictado
    /// (LRU) se a pilha excedeu MAX_ITEMS — quem chama é responsável por
    /// limpar o arquivo de cache do evictado, se for imagem.
    pub fn insert(&mut self, item: Item) -> Option<Item> {
        self.items.push_front(item);
        if self.items.len() > MAX_ITEMS {
            self.items.pop_back()
        } else {
            None
        }
    }

    /// Conveniência pra texto: dedup via touch, senão insere. Texto não tem
    /// I/O de cache, então pode ser uma operação só, ao contrário de imagem
    /// (que precisa ser salva em disco antes de virar Item — orquestrado
    /// fora do Stack).
    pub fn push_text(&mut self, content: String) -> PushOutcome {
        let hash = content_hash(content.as_bytes());
        if self.touch(&hash) {
            PushOutcome::Touched
        } else {
            self.insert(Item::new_text(content));
            PushOutcome::Inserted
        }
    }

    pub fn pin(&mut self, id: ItemId) -> Result<(), PinError> {
        let pos = self
            .items
            .iter()
            .position(|i| i.id == id)
            .ok_or(PinError::NotFound)?;
        if self.pins.len() >= MAX_PINS {
            return Err(PinError::LimitReached);
        }
        let item = self.items.remove(pos).expect("position just found");
        self.pins.push(item);
        Ok(())
    }

    /// Despina o item, devolvendo-o ao topo dos não-pinados. Retorna
    /// `Some(evicted)` se isso causou eviction do item mais antigo, e
    /// `None` se o item não estava pinado (nada aconteceu) ou não houve
    /// eviction. Diferencia os dois casos via `find`.
    ///
    /// SPEC_DEVIATION: design.md especificava `unpin(&mut self, id) -> bool`.
    /// Trocado pra devolver o item evictado (se houver) porque a limpeza do
    /// arquivo de cache de imagem (T4/T8) precisa saber qual item saiu —
    /// um `bool` de sucesso não carrega essa informação.
    pub fn unpin(&mut self, id: ItemId) -> UnpinOutcome {
        let Some(pos) = self.pins.iter().position(|i| i.id == id) else {
            return UnpinOutcome::NotPinned;
        };
        let item = self.pins.remove(pos);
        let evicted = self.insert(item);
        UnpinOutcome::Unpinned { evicted }
    }

    /// Atualiza a categoria de um item, pinado ou não.
    pub fn set_category(&mut self, id: ItemId, category: Category) -> Result<(), CategoryError> {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            item.category = category;
            return Ok(());
        }
        if let Some(item) = self.pins.iter_mut().find(|i| i.id == id) {
            item.category = category;
            return Ok(());
        }
        Err(CategoryError::NotFound)
    }

    /// Remove o item de onde ele estiver (não-pinado ou pinado).
    pub fn delete(&mut self, id: ItemId) -> Option<Item> {
        if let Some(pos) = self.items.iter().position(|i| i.id == id) {
            return self.items.remove(pos);
        }
        if let Some(pos) = self.pins.iter().position(|i| i.id == id) {
            return Some(self.pins.remove(pos));
        }
        None
    }
}

#[derive(Debug, PartialEq)]
pub enum UnpinOutcome {
    NotPinned,
    Unpinned { evicted: Option<Item> },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_text_inserts_new_item_at_top() {
        let mut stack = Stack::default();
        stack.push_text("first".into());
        stack.push_text("second".into());

        let items: Vec<_> = stack.items().collect();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].kind, ItemKind::Text("second".into()));
        assert_eq!(items[1].kind, ItemKind::Text("first".into()));
    }

    #[test]
    fn push_text_dedup_moves_existing_to_top_without_duplicating() {
        let mut stack = Stack::default();
        stack.push_text("a".into());
        stack.push_text("b".into());
        let outcome = stack.push_text("a".into());

        assert_eq!(outcome, PushOutcome::Touched);
        let items: Vec<_> = stack.items().collect();
        assert_eq!(items.len(), 2, "não deve duplicar");
        assert_eq!(items[0].kind, ItemKind::Text("a".into()));
    }

    #[test]
    fn push_text_evicts_oldest_when_over_capacity() {
        let mut stack = Stack::default();
        for i in 0..MAX_ITEMS {
            stack.push_text(format!("item-{i}"));
        }
        assert_eq!(stack.items().count(), MAX_ITEMS);

        stack.push_text("item-overflow".into());

        let items: Vec<_> = stack.items().collect();
        assert_eq!(items.len(), MAX_ITEMS, "não deve crescer além de 15");
        assert_eq!(items[0].kind, ItemKind::Text("item-overflow".into()));
        assert!(
            items
                .iter()
                .all(|i| i.kind != ItemKind::Text("item-0".into())),
            "o mais antigo (item-0) deveria ter sido evictado"
        );
    }

    #[test]
    fn pin_moves_item_from_stack_to_pins() {
        let mut stack = Stack::default();
        stack.push_text("a".into());
        let id = stack.items().next().unwrap().id;

        stack.pin(id).expect("deveria pinar com sucesso");

        assert_eq!(stack.items().count(), 0);
        assert_eq!(stack.pins().count(), 1);
        assert_eq!(stack.pins().next().unwrap().id, id);
    }

    #[test]
    fn pin_rejects_pin_past_limit_with_limit_reached() {
        let mut stack = Stack::default();
        let mut ids = vec![];
        for i in 0..MAX_PINS {
            stack.push_text(format!("pin-{i}"));
            let id = stack.items().next().unwrap().id;
            stack.pin(id).unwrap();
            ids.push(id);
        }
        assert_eq!(stack.pins().count(), MAX_PINS);

        stack.push_text("one-past-limit".into());
        let extra_id = stack.items().next().unwrap().id;
        let result = stack.pin(extra_id);

        assert_eq!(result, Err(PinError::LimitReached));
        assert_eq!(stack.pins().count(), MAX_PINS, "não deve alterar os pins");
        assert_eq!(stack.items().count(), 1, "item continua não-pinado");
    }

    #[test]
    fn pin_unknown_id_returns_not_found() {
        let mut stack = Stack::default();
        let result = stack.pin(Uuid::new_v4());
        assert_eq!(result, Err(PinError::NotFound));
    }

    #[test]
    fn unpin_returns_item_to_top_of_stack() {
        let mut stack = Stack::default();
        stack.push_text("a".into());
        let id = stack.items().next().unwrap().id;
        stack.pin(id).unwrap();

        let outcome = stack.unpin(id);

        assert_eq!(outcome, UnpinOutcome::Unpinned { evicted: None });
        assert_eq!(stack.pins().count(), 0);
        assert_eq!(stack.items().count(), 1);
        assert_eq!(stack.items().next().unwrap().id, id);
    }

    #[test]
    fn unpin_not_pinned_id_is_noop() {
        let mut stack = Stack::default();
        let outcome = stack.unpin(Uuid::new_v4());
        assert_eq!(outcome, UnpinOutcome::NotPinned);
    }

    #[test]
    fn unpin_evicts_oldest_when_stack_is_full() {
        let mut stack = Stack::default();
        stack.push_text("will-be-pinned".into());
        let pin_id = stack.items().next().unwrap().id;
        stack.pin(pin_id).unwrap();

        for i in 0..MAX_ITEMS {
            stack.push_text(format!("item-{i}"));
        }
        assert_eq!(stack.items().count(), MAX_ITEMS);

        let outcome = stack.unpin(pin_id);

        match outcome {
            UnpinOutcome::Unpinned {
                evicted: Some(evicted),
            } => {
                assert_eq!(evicted.kind, ItemKind::Text("item-0".into()));
            }
            other => panic!("esperava eviction do item mais antigo, veio {other:?}"),
        }
        assert_eq!(stack.items().count(), MAX_ITEMS);
    }

    #[test]
    fn delete_removes_non_pinned_item() {
        let mut stack = Stack::default();
        stack.push_text("a".into());
        let id = stack.items().next().unwrap().id;

        let removed = stack.delete(id);

        assert!(removed.is_some());
        assert_eq!(stack.items().count(), 0);
    }

    #[test]
    fn delete_removes_pinned_item_and_frees_slot() {
        let mut stack = Stack::default();
        stack.push_text("a".into());
        let id = stack.items().next().unwrap().id;
        stack.pin(id).unwrap();

        let removed = stack.delete(id);

        assert!(removed.is_some());
        assert_eq!(stack.pins().count(), 0);
    }

    #[test]
    fn delete_unknown_id_returns_none() {
        let mut stack = Stack::default();
        assert_eq!(stack.delete(Uuid::new_v4()), None);
    }

    #[test]
    fn set_category_updates_existing_item() {
        let mut stack = Stack::default();
        stack.push_text("a".into());
        let id = stack.items().next().unwrap().id;

        stack
            .set_category(id, Category::Url)
            .expect("deveria atualizar");

        assert_eq!(stack.items().next().unwrap().category, Category::Url);
    }

    #[test]
    fn set_category_unknown_id_returns_not_found() {
        let mut stack = Stack::default();
        let result = stack.set_category(Uuid::new_v4(), Category::Url);
        assert_eq!(result, Err(CategoryError::NotFound));
    }

    #[test]
    fn set_category_works_on_pinned_item() {
        let mut stack = Stack::default();
        stack.push_text("a".into());
        let id = stack.items().next().unwrap().id;
        stack.pin(id).unwrap();

        stack
            .set_category(id, Category::Codigo)
            .expect("deveria atualizar item pinado");

        assert_eq!(stack.pins().next().unwrap().category, Category::Codigo);
    }

    #[test]
    fn insert_image_item_computed_hash_used_for_dedup_via_touch() {
        let mut stack = Stack::default();
        let bytes = b"fake-png-bytes";
        let hash = content_hash(bytes);
        let item = Item::new_image(
            PathBuf::from("/cache/a.png"),
            "image/png".into(),
            bytes.len() as u64,
            hash,
        );
        stack.insert(item);

        assert!(stack.touch(&hash), "deveria encontrar a imagem pelo hash");
        assert_eq!(stack.items().count(), 1);
    }
}
