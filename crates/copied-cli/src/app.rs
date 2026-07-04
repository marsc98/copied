use std::io;

use copied_core::{Command, ItemId, ItemKindView, ItemView, Response};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::{DefaultTerminal, Frame};

use crate::ipc_client::IpcClient;

pub struct App {
    items: Vec<ItemView>,
    state: ListState,
    exit: bool,
    status: Option<String>,
}

impl App {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            state: ListState::default(),
            exit: false,
            status: None,
        }
    }

    pub fn run(
        &mut self,
        terminal: &mut DefaultTerminal,
        client: &mut IpcClient,
    ) -> io::Result<()> {
        self.refresh(client)?;
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events(client)?;
        }
        Ok(())
    }

    fn refresh(&mut self, client: &mut IpcClient) -> io::Result<()> {
        match client.send(&Command::List)? {
            Response::Items(items) => {
                self.items = items;
                if self.items.is_empty() {
                    self.state.select(None);
                } else {
                    let selected = self.state.selected().unwrap_or(0).min(self.items.len() - 1);
                    self.state.select(Some(selected));
                }
            }
            Response::Error { message } => self.status = Some(message),
            Response::Ack => {}
        }
        Ok(())
    }

    fn selected_id(&self) -> Option<ItemId> {
        self.state
            .selected()
            .and_then(|i| self.items.get(i))
            .map(|item| item.id)
    }

    fn handle_events(&mut self, client: &mut IpcClient) -> io::Result<()> {
        let Event::Key(key) = event::read()? else {
            return Ok(());
        };
        if key.kind != KeyEventKind::Press {
            return Ok(());
        }

        match key.code {
            KeyCode::Esc => self.exit = true,
            KeyCode::Up => self.move_selection(-1),
            KeyCode::Down => self.move_selection(1),
            KeyCode::Enter => self.copy_selected(client)?,
            KeyCode::Delete => self.delete_selected(client)?,
            KeyCode::F(2) => self.toggle_pin_selected(client)?,
            _ => {}
        }
        Ok(())
    }

    fn move_selection(&mut self, delta: isize) {
        if self.items.is_empty() {
            return;
        }
        let len = self.items.len() as isize;
        let current = self.state.selected().unwrap_or(0) as isize;
        let next = (current + delta).rem_euclid(len);
        self.state.select(Some(next as usize));
    }

    fn copy_selected(&mut self, client: &mut IpcClient) -> io::Result<()> {
        let Some(id) = self.selected_id() else {
            return Ok(());
        };
        match client.send(&Command::CopyToClipboard { id })? {
            Response::Ack => self.exit = true,
            Response::Error { message } => self.status = Some(message),
            Response::Items(_) => {}
        }
        Ok(())
    }

    fn delete_selected(&mut self, client: &mut IpcClient) -> io::Result<()> {
        let Some(id) = self.selected_id() else {
            return Ok(());
        };
        match client.send(&Command::Delete { id })? {
            Response::Ack => {
                self.status = None;
                self.refresh(client)?;
            }
            Response::Error { message } => self.status = Some(message),
            Response::Items(_) => {}
        }
        Ok(())
    }

    fn toggle_pin_selected(&mut self, client: &mut IpcClient) -> io::Result<()> {
        let Some(item) = self.state.selected().and_then(|i| self.items.get(i)) else {
            return Ok(());
        };
        let id = item.id;
        let cmd = if item.pinned {
            Command::Unpin { id }
        } else {
            Command::Pin { id }
        };
        match client.send(&cmd)? {
            Response::Ack => {
                self.status = None;
                self.refresh(client)?;
            }
            Response::Error { message } => self.status = Some(message),
            Response::Items(_) => {}
        }
        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();
        let chunks = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(area);

        if self.items.is_empty() {
            let empty = Paragraph::new("Pilha vazia — copie algo pra começar.")
                .block(Block::default().borders(Borders::ALL).title("copied"));
            frame.render_widget(empty, chunks[0]);
        } else {
            let list_items: Vec<ListItem> = self.items.iter().map(render_item).collect();
            let list = List::new(list_items)
                .block(Block::default().borders(Borders::ALL).title(
                    "copied — \u{2191}/\u{2193} nav · Enter copia · Delete exclui · F2 pin/unpin · Esc sai",
                ))
                .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
            frame.render_stateful_widget(list, chunks[0], &mut self.state);
        }

        let status_text = self.status.clone().unwrap_or_default();
        let status = Paragraph::new(status_text).style(Style::default().fg(Color::Red));
        frame.render_widget(status, chunks[1]);
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

fn render_item(item: &ItemView) -> ListItem<'static> {
    let prefix = if item.pinned { "[pin] " } else { "" };
    let content = match &item.kind {
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
    };
    ListItem::new(Line::from(Span::raw(format!("{prefix}{content}"))))
}
