use crate::bluetooth::{BleScan, HandledPeripheral};
use crate::error::Result;
use crate::tui::ui::{block, list, search_input, BlendrBlock, ShouldUpdate};
use crate::tui::AppRoute;
use crate::{route::Route, Ctx};
use btleplug::api::Peripheral;
use crossterm::event::{KeyCode, KeyEvent};
use regex::Regex;
use std::sync::Arc;
use tui::layout::Rect;
use tui::text::Spans;
use tui::widgets::{ListState, Paragraph, Widget, Wrap};
use tui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{List, ListItem},
    Frame,
};

pub enum Focus {
    Search,
    List,
}

pub(crate) struct PeripheralList {
    pub ctx: Arc<Ctx>,
    pub list_state: ListState,
    pub search: Option<String>,
    pub search_regex: Option<Regex>,
    pub focus: Focus,
    pub to_remove_unknowns: bool,
    pub first_match_done: bool,
}

impl PeripheralList {
    fn filter_peripherals(&self, peripheral: &HandledPeripheral) -> bool {
        if self.to_remove_unknowns && peripheral.name_unset {
            return false;
        }

        if let Some(regex) = self.search_regex.as_ref() {
            return regex.is_match(&peripheral.name);
        }

        if let Some(search) = self.search.as_ref() {
            return peripheral.name.contains(search);
        }

        true
    }
}

impl AppRoute for PeripheralList {
    fn new(ctx: Arc<Ctx>) -> Self {
        let initial_search = ctx.args.device.clone();

        PeripheralList {
            first_match_done: false,
            search_regex: match search_input::maybe_update_search_regexp(
                initial_search.as_deref(),
                None,
                &ctx,
            ) {
                ShouldUpdate::NoUpdate => None,
                ShouldUpdate::Update(regex) => regex,
            },
            search: initial_search,
            focus: Focus::List,
            list_state: ListState::default(),
            to_remove_unknowns: false,
            ctx,
        }
    }

    fn handle_input(&mut self, key: &KeyEvent) {
        let last_search = self.search.clone();

        if let Ok(Some(BleScan { peripherals, .. })) = &self.ctx.latest_scan.read().as_deref() {
            match self.focus {
                Focus::Search => {
                    search_input::handle_search_input(&mut self.search, key);

                    match key.code {
                        KeyCode::Enter | KeyCode::Down => {
                            self.list_state.select(Some(0));
                            self.focus = Focus::List
                        }
                        KeyCode::Esc => {
                            list::list_unselect(&mut self.list_state);
                            self.focus = Focus::List;
                        }
                        _ => (),
                    }
                }
                Focus::List => {
                    let filtered_peripherals = peripherals
                        .iter()
                        .filter(|peripheral| self.filter_peripherals(peripheral))
                        .collect::<Vec<_>>();

                    list::handle_key_input(
                        &filtered_peripherals,
                        &key.code,
                        &mut self.list_state,
                        |peripheral| {
                            Route::PeripheralWaitingView {
                                peripheral: peripheral.clone(),
                            }
                            .navigate(&self.ctx)
                        },
                    );

                    match key.code {
                        KeyCode::Char('/') => {
                            self.focus = Focus::Search;
                            list::list_unselect(&mut self.list_state)
                        }
                        KeyCode::Char('r') => {
                            if let Ok(request_restart) =
                                self.ctx.request_scan_restart.lock().as_deref_mut()
                            {
                                *request_restart = true;
                            }
                        }
                        KeyCode::Char('u') => self.to_remove_unknowns = !self.to_remove_unknowns,
                        _ => {}
                    }
                }
            }
        }

        if let ShouldUpdate::Update(new_regex) =
            search_input::maybe_update_search_regexp(self.search.as_deref(), last_search, &self.ctx)
        {
            self.search_regex = new_regex;
        }
    }

    fn render(
        &mut self,
        area: Rect,
        route_active: bool,
        f: &mut Frame<super::TerminalBackend>,
    ) -> Result<()> {
        let scan = self.ctx.latest_scan.read();
        let BleScan {
            peripherals,
            sync_time,
        } = if let Ok(Some(scan)) = scan.as_deref() {
            scan
        } else {
            // todo add no peripherals handling
            return Err("no peripherals".into());
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(3),
                    Constraint::Min(10),
                    Constraint::Length(if route_active { 3 } else { 0 }),
                ]
                .as_ref(),
            )
            .split(area);

        let input = Paragraph::new(Spans(vec![
            Span::styled(" /", Style::default().fg(Color::DarkGray)),
            Span::from(self.search.as_deref().unwrap_or("")),
        ]))
        .style(Style::default())
        .block(tui::widgets::Block::from(BlendrBlock {
            route_active,
            focused: matches!(self.focus, Focus::Search),
            title: "Filter with regex (press \"/\" to focus)",
        }));

        f.render_widget(input, chunks[0]);

        let filtered_peripherals: Vec<_> = peripherals
            .iter()
            .filter(|peripheral| self.filter_peripherals(peripheral))
            .collect();

        if !self.first_match_done && filtered_peripherals.len() == 1 && self.search.is_some() {
            self.list_state.select(Some(0));
            Route::PeripheralWaitingView {
                peripheral: filtered_peripherals[0].clone(),
            }
            .navigate(&self.ctx);
        }

        if !self.first_match_done && !filtered_peripherals.is_empty() {
            self.first_match_done = true
        }

        let items: Vec<ListItem> = filtered_peripherals
            .into_iter()
            .enumerate()
            .map(|(i, peripheral)| {
                let is_highlighted = Some(i) == self.list_state.selected();
                ListItem::new(Span::from(format!(
                    "{}{}{}",
                    if is_highlighted { "> " } else { "  " },
                    peripheral.name,
                    match peripheral.rssi {
                        Some(rssi) => format!(" (rssi {rssi})"),
                        None => String::from(""),
                    }
                )))
                .style(Style::default().fg(Color::Gray))
            })
            .collect();

        // Create a List from all list items and highlight the currently selected one
        let items = List::new(items)
            .block(tui::widgets::Block::from(BlendrBlock {
                route_active,
                focused: matches!(self.focus, Focus::List),
                title: format!("Latest Scan on {}", sync_time.format("%H:%M:%S")).as_str(),
            }))
            .highlight_style(
                Style::default()
                    .bg(Color::LightBlue)
                    .add_modifier(Modifier::BOLD),
            );

        // We can now render the item list
        f.render_stateful_widget(items, chunks[1], &mut self.list_state);
        if chunks[2].height > 0 {
            f.render_widget(
                block::render_help([
                    Some(("q", "Quit", false)),
                    Some(("u", "Hide unknown devices", self.to_remove_unknowns)),
                    Some(("->", "Connect to device", false)),
                    Some(("r", "Restart scan", false)),
                    Some(("h/j or arrows", "Navigate", false)),
                ]),
                chunks[2],
            );
        }

        Ok(())
    }
}
