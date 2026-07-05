use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::mpsc::Sender;

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;
use copied_core::{Command, ItemId, ItemKindView, ItemView, Response};
use futures::stream::{self, Stream, StreamExt};
use iced::keyboard::key::Named;
use iced::keyboard::{Event as KeyboardEvent, Key};
use iced::widget::image::Handle as ImageHandle;
use iced::widget::{
    button, column, container, image, mouse_area, row, rule, scrollable, text, text_input,
};
use iced::{Element, Length, Subscription, Task};
use iced_layershell::to_layer_message;

use crate::ipc_worker;
use crate::symbols;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Stack,
    Symbols,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Focus {
    TabStack,
    TabSymbols,
    Search,
    List,
}

impl Focus {
    fn next(self) -> Self {
        match self {
            Focus::TabStack => Focus::TabSymbols,
            Focus::TabSymbols => Focus::Search,
            Focus::Search => Focus::List,
            Focus::List => Focus::TabStack,
        }
    }
}

const SEARCH_ID: &str = "copied-gui-search";
const LIST_ID: &str = "copied-gui-list";

const BOLD_FONT: iced::Font = iced::Font {
    weight: iced::font::Weight::Bold,
    ..iced::Font::DEFAULT
};

const BUTTON_RADIUS: f32 = 8.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PendingAction {
    List,
    Copy,
    Delete,
    TogglePin,
    FetchImage(ItemId),
}

pub struct AppState {
    items: Vec<ItemView>,
    search: String,
    active_tab: Tab,
    selected: Option<ItemId>,
    hovered: Option<ItemId>,
    status: Option<String>,
    ipc_tx: Option<Sender<Command>>,
    pending: VecDeque<PendingAction>,
    image_cache: HashMap<ItemId, ImageHandle>,
    image_requested: HashSet<ItemId>,
    focus: Focus,
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
    DeleteClicked(ItemId),
    MoveSelection(isize),
    SearchChanged(String),
    EnterPressed,
    FocusNext,
    DeleteSelected,
    TogglePinSelected,
    TabSelected(Tab),
    SymbolClicked(&'static str),
    CloseRequested,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            search: String::new(),
            active_tab: Tab::Stack,
            selected: None,
            hovered: None,
            status: None,
            ipc_tx: None,
            pending: VecDeque::new(),
            image_cache: HashMap::new(),
            image_requested: HashSet::new(),
            focus: Focus::Search,
        }
    }

    pub fn boot() -> (Self, Task<Message>) {
        (Self::new(), iced::widget::operation::focus(SEARCH_ID))
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
            .filter(|item| render_content(item, false).to_lowercase().contains(&needle))
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
                    Some(PendingAction::Delete) | Some(PendingAction::TogglePin) => {
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
            state.focus = Focus::List;
            state.send(Command::CopyToClipboard { id }, PendingAction::Copy);
            Task::none()
        }
        Message::TogglePinClicked(id) => {
            state.selected = Some(id);
            toggle_pin(state, id);
            Task::none()
        }
        Message::DeleteClicked(id) => {
            state.selected = Some(id);
            state.send(Command::Delete { id }, PendingAction::Delete);
            Task::none()
        }
        Message::MoveSelection(delta) => {
            state.move_selection(delta);
            scroll_to_selection(state)
        }
        Message::SearchChanged(value) => {
            state.search = value;
            state.clamp_selection();
            Task::none()
        }
        Message::EnterPressed => match state.focus {
            Focus::TabStack => {
                state.active_tab = Tab::Stack;
                Task::none()
            }
            Focus::TabSymbols => {
                state.active_tab = Tab::Symbols;
                Task::none()
            }
            Focus::Search | Focus::List => {
                if let Some(id) = state.selected {
                    state.send(Command::CopyToClipboard { id }, PendingAction::Copy);
                }
                Task::none()
            }
        },
        Message::FocusNext => {
            state.focus = state.focus.next();
            match state.focus {
                Focus::Search => iced::widget::operation::focus(SEARCH_ID),
                _ => iced::widget::operation::focus(iced::widget::Id::unique()),
            }
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
        Message::TabSelected(tab) => {
            state.active_tab = tab;
            state.focus = match tab {
                Tab::Stack => Focus::TabStack,
                Tab::Symbols => Focus::TabSymbols,
            };
            Task::none()
        }
        Message::SymbolClicked(symbol) => {
            let _ = copy_symbol_to_clipboard(symbol);
            iced::exit()
        }
        Message::CloseRequested => iced::exit(),
        _ => Task::none(),
    }
}

fn copy_symbol_to_clipboard(symbol: &str) -> Result<(), wl_clipboard_rs::copy::Error> {
    use wl_clipboard_rs::copy::{self, MimeType, Options, Source};
    let options = Options::default();
    let source = Source::Bytes(symbol.as_bytes().to_vec().into_boxed_slice());
    copy::copy(options, source, MimeType::Text)
}

fn scroll_to_selection(state: &AppState) -> Task<Message> {
    let visible = state.filtered_items();
    let Some(selected) = state.selected else {
        return Task::none();
    };
    let Some(index) = visible.iter().position(|item| item.id == selected) else {
        return Task::none();
    };
    if visible.len() <= 1 {
        return Task::none();
    }
    let fraction = index as f32 / (visible.len() - 1) as f32;
    iced::widget::operation::snap_to(
        LIST_ID,
        iced::widget::operation::RelativeOffset {
            x: 0.0,
            y: fraction,
        },
    )
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
    let tab_bar = row![
        button("Stack")
            .on_press(Message::TabSelected(Tab::Stack))
            .style(move |theme, status| tab_button_style(
                theme,
                status,
                state.active_tab == Tab::Stack,
                state.focus == Focus::TabStack
            )),
        button("Símbolos")
            .on_press(Message::TabSelected(Tab::Symbols))
            .style(move |theme, status| tab_button_style(
                theme,
                status,
                state.active_tab == Tab::Symbols,
                state.focus == Focus::TabSymbols
            )),
    ]
    .spacing(6);

    let search_box = text_input("Buscar…", &state.search)
        .on_input(Message::SearchChanged)
        .id(SEARCH_ID)
        .width(Length::Fill);

    let body = match state.active_tab {
        Tab::Stack => view_stack(state),
        Tab::Symbols => view_symbols(state),
    };

    let status = text(state.status.clone().unwrap_or_default());

    let content = column![tab_bar, search_box, body, status]
        .width(Length::Fill)
        .spacing(10)
        .padding(10);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(window_style)
        .into()
}

fn window_style(theme: &iced::Theme) -> container::Style {
    container::Style {
        border: iced::border::color(theme.extended_palette().background.strong.color).width(1.0),
        ..container::Style::default()
    }
}

fn tab_button_style(
    theme: &iced::Theme,
    status: button::Status,
    active: bool,
    focused: bool,
) -> button::Style {
    let mut style = if active {
        button::primary(theme, status)
    } else {
        button::secondary(theme, status)
    };
    if focused {
        style.border = iced::border::color(theme.extended_palette().primary.base.color).width(2.0);
    }
    style.border = style.border.rounded(BUTTON_RADIUS);
    style
}

fn rounded_primary(theme: &iced::Theme, status: button::Status) -> button::Style {
    let mut style = button::primary(theme, status);
    style.border = style.border.rounded(BUTTON_RADIUS);
    style
}

fn view_stack(state: &AppState) -> Element<'_, Message> {
    let filtered = state.filtered_items();

    if state.items.is_empty() {
        text("Pilha vazia — copie algo pra começar.").into()
    } else if filtered.is_empty() {
        text("Nenhum resultado pra essa busca.").into()
    } else {
        let (pinned, normal): (Vec<&ItemView>, Vec<&ItemView>) =
            filtered.into_iter().partition(|item| item.pinned);

        let mut sections: Vec<Element<'_, Message>> = Vec::new();
        if !pinned.is_empty() {
            let pin_rows = pinned
                .into_iter()
                .map(|item| render_item(state, item, None));
            sections.push(column(pin_rows).width(Length::Fill).spacing(4).into());
            sections.push(rule::horizontal(1).into());
        }
        let normal_rows = normal
            .into_iter()
            .enumerate()
            .map(|(index, item)| render_item(state, item, Some(index + 1)));
        sections.push(column(normal_rows).width(Length::Fill).spacing(4).into());

        scrollable(column(sections).width(Length::Fill).spacing(16))
            .id(LIST_ID)
            .into()
    }
}

fn view_symbols(state: &AppState) -> Element<'_, Message> {
    let needle = state.search.trim().to_lowercase();
    let groups: Vec<Element<'_, Message>> = symbols::CATALOG
        .iter()
        .filter_map(|group| {
            let matches: Vec<&'static str> =
                if needle.is_empty() || group.name.to_lowercase().contains(&needle) {
                    group.symbols.to_vec()
                } else {
                    group
                        .symbols
                        .iter()
                        .copied()
                        .filter(|symbol| symbol.to_lowercase().contains(&needle))
                        .collect()
                };
            if matches.is_empty() {
                return None;
            }
            let buttons = matches.into_iter().map(|symbol| {
                button(text(symbol))
                    .on_press(Message::SymbolClicked(symbol))
                    .style(rounded_primary)
                    .into()
            });
            Some(
                column![text(group.name), row(buttons).spacing(4)]
                    .spacing(4)
                    .into(),
            )
        })
        .collect();

    if groups.is_empty() {
        text("Nenhum resultado pra essa busca.").into()
    } else {
        scrollable(column(groups).spacing(10).width(Length::Fill)).into()
    }
}

fn render_item<'a>(
    state: &AppState,
    item: &'a ItemView,
    position: Option<usize>,
) -> Element<'a, Message> {
    let is_selected = state.selected == Some(item.id);
    let is_hovered = state.hovered == Some(item.id);
    let prefix = position.map(|n| text(format!("{n} | ")).font(BOLD_FONT));

    let preview: Element<'_, Message> = match (&item.kind, state.image_cache.get(&item.id)) {
        (ItemKindView::Image { .. }, Some(handle)) => {
            let mut contents = row![].spacing(6);
            if let Some(prefix) = prefix {
                contents = contents.push(prefix);
            }
            contents
                .push(
                    image(handle.clone())
                        .width(Length::Fixed(48.0))
                        .height(Length::Fixed(48.0)),
                )
                .into()
        }
        _ => {
            let mut contents = row![].width(Length::Fill);
            if let Some(prefix) = prefix {
                contents = contents.push(prefix);
            }
            contents
                .push(text(render_content(item, is_selected)))
                .into()
        }
    };

    let pin_button = button(if item.pinned { "Unpin (p)" } else { "Pin (p)" })
        .on_press(Message::TogglePinClicked(item.id))
        .style(rounded_primary);

    let delete_button = button("Delete (d)")
        .on_press(Message::DeleteClicked(item.id))
        .style(rounded_primary);

    let actions = row![pin_button, delete_button]
        .spacing(6)
        .width(Length::Fixed(220.0));

    let content = row![preview, actions]
        .spacing(8)
        .width(Length::Fill)
        .align_y(iced::Alignment::Center);

    let is_focused = is_selected && state.focus == Focus::List;
    let item_container = container(content)
        .width(Length::Fill)
        .padding(8)
        .style(move |theme: &iced::Theme| item_style(theme, is_selected, is_hovered, is_focused));

    mouse_area(item_container)
        .on_press(Message::ItemClicked(item.id))
        .on_enter(Message::ItemHovered(item.id))
        .on_exit(Message::ItemUnhovered)
        .into()
}

fn item_style(
    theme: &iced::Theme,
    selected: bool,
    hovered: bool,
    focused: bool,
) -> container::Style {
    let palette = theme.extended_palette();
    let background = if selected {
        Some(palette.primary.weak.color.into())
    } else if hovered {
        Some(palette.background.weak.color.into())
    } else {
        None
    };
    let border = if focused {
        iced::border::color(palette.primary.base.color).width(2.0)
    } else {
        iced::Border::default()
    };
    container::Style {
        background,
        border,
        ..container::Style::default()
    }
}

fn render_content(item: &ItemView, full: bool) -> String {
    match &item.kind {
        ItemKindView::Text { preview } => {
            if full {
                return preview.clone();
            }
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

pub fn subscription(state: &AppState) -> Subscription<Message> {
    Subscription::batch([
        Subscription::run(ipc_stream),
        keyboard_subscription(state.focus),
    ])
}

/// Conecta ao `ipc_worker` uma vez que a subscription começa a rodar; o
/// `Sender<Command>` volta como a primeira mensagem do stream (padrão
/// recomendado pelo próprio iced pra workers com canal bidirecional), já que
/// `Subscription::run` só aceita `fn() -> S` sem captura de estado externo.
fn ipc_stream() -> impl Stream<Item = Message> {
    let (cmd_tx, resp_rx) = ipc_worker::spawn();
    stream::once(async move { Message::IpcConnected(cmd_tx) })
        .chain(resp_rx.map(Message::IpcResponse))
}

fn keyboard_subscription(focus: Focus) -> Subscription<Message> {
    iced::keyboard::listen()
        .with(focus)
        .filter_map(|(focus, event)| {
            let KeyboardEvent::KeyPressed { key, .. } = event else {
                return None;
            };
            match key {
                Key::Named(Named::ArrowUp) => Some(Message::MoveSelection(-1)),
                Key::Named(Named::ArrowDown) => Some(Message::MoveSelection(1)),
                Key::Named(Named::Enter) => Some(Message::EnterPressed),
                Key::Named(Named::Tab) => Some(Message::FocusNext),
                Key::Named(Named::Delete) => Some(Message::DeleteSelected),
                Key::Named(Named::F2) => Some(Message::TogglePinSelected),
                Key::Named(Named::Escape) => Some(Message::CloseRequested),
                Key::Character(c)
                    if focus == Focus::List && c.as_str().eq_ignore_ascii_case("p") =>
                {
                    Some(Message::TogglePinSelected)
                }
                Key::Character(c)
                    if focus == Focus::List && c.as_str().eq_ignore_ascii_case("d") =>
                {
                    Some(Message::DeleteSelected)
                }
                _ => None,
            }
        })
}
