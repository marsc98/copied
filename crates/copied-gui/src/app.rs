use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::mpsc::Sender;

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;
use copied_core::{Category, Command, ItemId, ItemKindView, ItemView, Response};
use futures::stream::{self, Stream, StreamExt};
use iced::keyboard::key::Named;
use iced::keyboard::{Event as KeyboardEvent, Key};
use iced::widget::image::Handle as ImageHandle;
use iced::widget::{button, column, container, image, mouse_area, row, scrollable, text, text_input};
use iced::{Element, Length, Subscription, Task};
use iced_layershell::to_layer_message;

use crate::ipc_worker;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PendingAction {
    List,
    Copy,
    Delete,
    TogglePin,
    SetCategory,
    FetchImage(ItemId),
}

pub struct AppState {
    items: Vec<ItemView>,
    search: String,
    selected: Option<ItemId>,
    hovered: Option<ItemId>,
    status: Option<String>,
    ipc_tx: Option<Sender<Command>>,
    pending: VecDeque<PendingAction>,
    image_cache: HashMap<ItemId, ImageHandle>,
    image_requested: HashSet<ItemId>,
}

#[to_layer_message]
#[derive(Debug, Clone)]
pub enum Message {
    IpcConnected(Sender<Command>),
    IpcResponse(Response),
    ItemHovered(ItemId),
    ItemUnhovered,
    ItemClicked(ItemId),
    TogglePinClicked(ItemId),
    CategoryClicked(ItemId),
    MoveSelection(isize),
    SearchChanged(String),
    CopySelected,
    DeleteSelected,
    TogglePinSelected,
    CloseRequested,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            search: String::new(),
            selected: None,
            hovered: None,
            status: None,
            ipc_tx: None,
            pending: VecDeque::new(),
            image_cache: HashMap::new(),
            image_requested: HashSet::new(),
        }
    }

    fn send(&mut self, cmd: Command, action: PendingAction) {
        if let Some(tx) = &self.ipc_tx {
            self.pending.push_back(action);
            let _ = tx.send(cmd);
        }
    }

    /// Dispara `GetImageBytes` uma vez por item de imagem ainda não
    /// cacheado/pedido — chamado sempre que a lista é atualizada.
    fn queue_missing_image_fetches(&mut self) {
        let ids: Vec<ItemId> = self
            .items
            .iter()
            .filter(|item| matches!(item.kind, ItemKindView::Image { .. }))
            .map(|item| item.id)
            .filter(|id| !self.image_requested.contains(id))
            .collect();
        for id in ids {
            self.image_requested.insert(id);
            self.send(Command::GetImageBytes { id }, PendingAction::FetchImage(id));
        }
    }

    fn filtered_items(&self) -> Vec<&ItemView> {
        if self.search.trim().is_empty() {
            return self.items.iter().collect();
        }
        let needle = self.search.to_lowercase();
        self.items
            .iter()
            .filter(|item| render_content(item).to_lowercase().contains(&needle))
            .collect()
    }

    fn clamp_selection(&mut self) {
        let visible = self.filtered_items();
        if visible.is_empty() {
            self.selected = None;
            return;
        }
        if self
            .selected
            .is_none_or(|id| !visible.iter().any(|item| item.id == id))
        {
            self.selected = Some(visible[0].id);
        }
    }

    fn move_selection(&mut self, delta: isize) {
        let visible = self.filtered_items();
        if visible.is_empty() {
            return;
        }
        let len = visible.len() as isize;
        let current = self
            .selected
            .and_then(|id| visible.iter().position(|item| item.id == id))
            .unwrap_or(0) as isize;
        let next = (current + delta).rem_euclid(len) as usize;
        self.selected = Some(visible[next].id);
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

pub fn update(state: &mut AppState, message: Message) -> Task<Message> {
    match message {
        Message::IpcConnected(tx) => {
            state.ipc_tx = Some(tx);
            state.send(Command::List, PendingAction::List);
            Task::none()
        }
        Message::IpcResponse(response) => {
            let action = state.pending.pop_front();
            match response {
                Response::Items(items) => {
                    state.items = items;
                    state.clamp_selection();
                    state.queue_missing_image_fetches();
                    Task::none()
                }
                Response::Ack => match action {
                    Some(PendingAction::Copy) => iced::exit(),
                    Some(PendingAction::Delete)
                    | Some(PendingAction::TogglePin)
                    | Some(PendingAction::SetCategory) => {
                        state.status = None;
                        state.send(Command::List, PendingAction::List);
                        Task::none()
                    }
                    _ => Task::none(),
                },
                Response::Error { message } => {
                    if !matches!(action, Some(PendingAction::FetchImage(_))) {
                        state.status = Some(message);
                    }
                    Task::none()
                }
                Response::ImageBytes { data_base64, .. } => {
                    if let Some(PendingAction::FetchImage(id)) = action {
                        if let Ok(bytes) = BASE64.decode(data_base64) {
                            state.image_cache.insert(id, ImageHandle::from_bytes(bytes));
                        }
                    }
                    Task::none()
                }
            }
        }
        Message::ItemHovered(id) => {
            state.hovered = Some(id);
            Task::none()
        }
        Message::ItemUnhovered => {
            state.hovered = None;
            Task::none()
        }
        Message::ItemClicked(id) => {
            state.selected = Some(id);
            state.send(Command::CopyToClipboard { id }, PendingAction::Copy);
            Task::none()
        }
        Message::TogglePinClicked(id) => {
            state.selected = Some(id);
            toggle_pin(state, id);
            Task::none()
        }
        Message::CategoryClicked(id) => {
            state.selected = Some(id);
            if let Some(item) = state.items.iter().find(|item| item.id == id) {
                let category = next_category(item.category);
                state.send(Command::SetCategory { id, category }, PendingAction::SetCategory);
            }
            Task::none()
        }
        Message::MoveSelection(delta) => {
            state.move_selection(delta);
            Task::none()
        }
        Message::SearchChanged(value) => {
            state.search = value;
            state.clamp_selection();
            Task::none()
        }
        Message::CopySelected => {
            if let Some(id) = state.selected {
                state.send(Command::CopyToClipboard { id }, PendingAction::Copy);
            }
            Task::none()
        }
        Message::DeleteSelected => {
            if let Some(id) = state.selected {
                state.send(Command::Delete { id }, PendingAction::Delete);
            }
            Task::none()
        }
        Message::TogglePinSelected => {
            if let Some(id) = state.selected {
                toggle_pin(state, id);
            }
            Task::none()
        }
        Message::CloseRequested => iced::exit(),
        _ => Task::none(),
    }
}

fn next_category(current: Category) -> Category {
    match current {
        Category::Texto => Category::Url,
        Category::Url => Category::Codigo,
        Category::Codigo => Category::Imagem,
        Category::Imagem => Category::Outro,
        Category::Outro => Category::Texto,
    }
}

fn toggle_pin(state: &mut AppState, id: ItemId) {
    let Some(item) = state.items.iter().find(|item| item.id == id) else {
        return;
    };
    let cmd = if item.pinned {
        Command::Unpin { id }
    } else {
        Command::Pin { id }
    };
    state.send(cmd, PendingAction::TogglePin);
}

pub fn view(state: &AppState) -> Element<'_, Message> {
    let search_box = text_input("Buscar…", &state.search)
        .on_input(Message::SearchChanged)
        .width(Length::Fill);

    let filtered = state.filtered_items();

    let body: Element<'_, Message> = if state.items.is_empty() {
        text("Pilha vazia — copie algo pra começar.").into()
    } else if filtered.is_empty() {
        text("Nenhum resultado pra essa busca.").into()
    } else {
        let rows = filtered.into_iter().map(|item| render_item(state, item));
        scrollable(column(rows).width(Length::Fill)).into()
    };

    let status = text(state.status.clone().unwrap_or_default());

    column![search_box, body, status].width(Length::Fill).into()
}

fn render_item<'a>(state: &AppState, item: &'a ItemView) -> Element<'a, Message> {
    let marker = if state.selected == Some(item.id) {
        "> "
    } else {
        "  "
    };
    let pin_marker = if item.pinned { "[pin] " } else { "" };
    let prefix = format!("{marker}{pin_marker}");

    let preview: Element<'_, Message> =
        match (&item.kind, state.image_cache.get(&item.id)) {
            (ItemKindView::Image { .. }, Some(handle)) => row![
                text(prefix),
                image(handle.clone())
                    .width(Length::Fixed(48.0))
                    .height(Length::Fixed(48.0)),
            ]
            .spacing(6)
            .into(),
            _ => text(format!("{prefix}{}", render_content(item))).into(),
        };

    let category_button = button(text(format!("{:?}", item.category)))
        .on_press(Message::CategoryClicked(item.id));

    let pin_button = button(if item.pinned { "unpin" } else { "pin" })
        .on_press(Message::TogglePinClicked(item.id));

    let content = row![preview, category_button, pin_button].width(Length::Fill);

    mouse_area(container(content).width(Length::Fill))
        .on_press(Message::ItemClicked(item.id))
        .on_enter(Message::ItemHovered(item.id))
        .on_exit(Message::ItemUnhovered)
        .into()
}

fn render_content(item: &ItemView) -> String {
    match &item.kind {
        ItemKindView::Text { preview } => {
            let truncated: String = preview.chars().take(60).collect();
            if preview.chars().count() > 60 {
                format!("{truncated}…")
            } else {
                truncated
            }
        }
        ItemKindView::Image { mime, size_bytes } => {
            let kb = size_bytes / 1024;
            let format = mime.rsplit('/').next().unwrap_or(mime).to_uppercase();
            format!("[Imagem {format}, {kb}KB]")
        }
    }
}

pub fn subscription(_state: &AppState) -> Subscription<Message> {
    Subscription::batch([Subscription::run(ipc_stream), keyboard_subscription()])
}

/// Conecta ao `ipc_worker` uma vez que a subscription começa a rodar; o
/// `Sender<Command>` volta como a primeira mensagem do stream (padrão
/// recomendado pelo próprio iced pra workers com canal bidirecional), já que
/// `Subscription::run` só aceita `fn() -> S` sem captura de estado externo.
fn ipc_stream() -> impl Stream<Item = Message> {
    let (cmd_tx, resp_rx) = ipc_worker::spawn();
    stream::once(async move { Message::IpcConnected(cmd_tx) }).chain(resp_rx.map(Message::IpcResponse))
}

fn keyboard_subscription() -> Subscription<Message> {
    iced::keyboard::listen().filter_map(|event| {
        let KeyboardEvent::KeyPressed { key, .. } = event else {
            return None;
        };
        match key {
            Key::Named(Named::ArrowUp) => Some(Message::MoveSelection(-1)),
            Key::Named(Named::ArrowDown) => Some(Message::MoveSelection(1)),
            Key::Named(Named::Enter) => Some(Message::CopySelected),
            Key::Named(Named::Delete) => Some(Message::DeleteSelected),
            Key::Named(Named::F2) => Some(Message::TogglePinSelected),
            Key::Named(Named::Escape) => Some(Message::CloseRequested),
            _ => None,
        }
    })
}
