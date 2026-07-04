use std::sync::mpsc::Sender;

use copied_core::{Command, ItemId, ItemKindView, ItemView, Response};
use futures::stream::{self, Stream, StreamExt};
use iced::keyboard::key::Named;
use iced::keyboard::{Event as KeyboardEvent, Key};
use iced::widget::{button, column, container, mouse_area, row, scrollable, text, text_input};
use iced::{Element, Length, Subscription, Task};

use crate::ipc_worker;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PendingAction {
    List,
    Copy,
    Delete,
    TogglePin,
}

pub struct AppState {
    items: Vec<ItemView>,
    search: String,
    selected: Option<ItemId>,
    hovered: Option<ItemId>,
    status: Option<String>,
    should_exit: bool,
    ipc_tx: Option<Sender<Command>>,
    pending: Option<PendingAction>,
}

#[derive(Debug, Clone)]
pub enum Message {
    IpcConnected(Sender<Command>),
    IpcResponse(Response),
    ItemHovered(ItemId),
    ItemUnhovered,
    ItemClicked(ItemId),
    TogglePinClicked(ItemId),
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
            should_exit: false,
            ipc_tx: None,
            pending: None,
        }
    }

    pub fn should_exit(&self) -> bool {
        self.should_exit
    }

    fn send(&mut self, cmd: Command, action: PendingAction) {
        if let Some(tx) = &self.ipc_tx {
            self.pending = Some(action);
            let _ = tx.send(cmd);
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
            let _ = tx.send(Command::List);
            state.pending = Some(PendingAction::List);
            state.ipc_tx = Some(tx);
        }
        Message::IpcResponse(response) => {
            let action = state.pending.take();
            match response {
                Response::Items(items) => {
                    state.items = items;
                    state.clamp_selection();
                }
                Response::Ack => match action {
                    Some(PendingAction::Copy) => state.should_exit = true,
                    Some(PendingAction::Delete) | Some(PendingAction::TogglePin) => {
                        state.status = None;
                        state.send(Command::List, PendingAction::List);
                    }
                    _ => {}
                },
                Response::Error { message } => state.status = Some(message),
                Response::ImageBytes { .. } => {}
            }
        }
        Message::ItemHovered(id) => state.hovered = Some(id),
        Message::ItemUnhovered => state.hovered = None,
        Message::ItemClicked(id) => {
            state.selected = Some(id);
            state.send(Command::CopyToClipboard { id }, PendingAction::Copy);
        }
        Message::TogglePinClicked(id) => {
            state.selected = Some(id);
            toggle_pin(state, id);
        }
        Message::MoveSelection(delta) => state.move_selection(delta),
        Message::SearchChanged(value) => {
            state.search = value;
            state.clamp_selection();
        }
        Message::CopySelected => {
            if let Some(id) = state.selected {
                state.send(Command::CopyToClipboard { id }, PendingAction::Copy);
            }
        }
        Message::DeleteSelected => {
            if let Some(id) = state.selected {
                state.send(Command::Delete { id }, PendingAction::Delete);
            }
        }
        Message::TogglePinSelected => {
            if let Some(id) = state.selected {
                toggle_pin(state, id);
            }
        }
        Message::CloseRequested => state.should_exit = true,
    }
    Task::none()
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
    let label = text(format!("{marker}{pin_marker}{}", render_content(item)));

    let pin_button = button(if item.pinned { "unpin" } else { "pin" })
        .on_press(Message::TogglePinClicked(item.id));

    let content = row![label, pin_button].width(Length::Fill);

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
