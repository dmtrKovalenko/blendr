use crate::bluetooth::{BleScan, HandledPeripheral};
use crate::error::Result;
use crate::tui::ui::{block, list::StableListState, search_input, BlendrBlock, ShouldUpdate};
use crate::tui::ui::{HandleInputResult, StableIndexList};
use crate::tui::{AppRoute, HandleKeydownResult};
use crate::{route::Route, Ctx};
use btleplug::platform::PeripheralId;
use crossterm::event::{KeyCode, KeyEvent};
use regex::Regex;
use std::sync::atomic::AtomicU16;
use std::sync::Arc;
use tui::layout::Rect;
use tui::text::Line;
use tui::widgets::Paragraph;
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
    pub list_state: StableListState<PeripheralId>,
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
            return regex.is_match(&peripheral.name)
                || peripheral
                    .services_names
                    .iter()
                    .any(|name| regex.is_match(name));
        }

        if let Some(search) = self.search.as_ref() {
            return peripheral.name.contains(search)
                || peripheral
                    .services_names
                    .iter()
                    .any(|name| name.contains(search));
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
            list_state: StableListState::default(),
            to_remove_unknowns: false,
            ctx,
        }
    }

    fn handle_input(&mut self, key: &KeyEvent) -> HandleKeydownResult {
        let last_search = self.search.clone();

        if let Ok(Some(BleScan { peripherals, .. })) = &self.ctx.latest_scan.read().as_deref() {
            let filtered_peripherals = peripherals
                .iter()
                .filter(|peripheral| self.filter_peripherals(peripheral))
                .collect::<StableIndexList<PeripheralId, HandledPeripheral>>();

            self.list_state
                .stabilize_selected_index(&filtered_peripherals);

            match self.focus {
                Focus::Search => {
                    search_input::handle_search_input(&mut self.search, key);

                    match key.code {
                        KeyCode::Enter | KeyCode::Down => {
                            self.list_state.select(&filtered_peripherals, Some(0));
                            self.focus = Focus::List
                        }
                        KeyCode::Esc | KeyCode::Tab => {
                            self.list_state.list_unselect(&filtered_peripherals);
                            self.focus = Focus::List;
                        }
                        _ => (),
                    }
                }
                Focus::List => {
                    if let HandleInputResult::Selected(peripheral) =
                        StableListState::handle_key_input(
                            &mut self.list_state,
                            &filtered_peripherals,
                            &key.code,
                        )
                    {
                        Route::PeripheralWaitingView {
                            retry: Arc::new(AtomicU16::new(0)),
                            peripheral: peripheral.clone(),
                        }
                        .navigate(&self.ctx)
                    }

                    match key.code {
                        KeyCode::Char('/') | KeyCode::Tab => {
                            self.focus = Focus::Search;
                            self.list_state.list_unselect(&filtered_peripherals)
                        }
                        KeyCode::Char('r') => {
                            if let Ok(request_restart) =
                                self.ctx.request_scan_restart.lock().as_deref_mut()
                            {
                                *request_restart = true;
                            }
                        }
                        KeyCode::Char('u') => self.to_remove_unknowns = !self.to_remove_unknowns,
                        KeyCode::Char('s') => {
                            let mut sort_by_name = self.ctx.sort_by_name.lock().unwrap();
                            *sort_by_name = !*sort_by_name;
                        }
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

        HandleKeydownResult::Continue
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
            let loading_placeholder = Paragraph::new(Line::from("In progress..."))
                .style(Style::default())
                .block(tui::widgets::Block::from(BlendrBlock {
                    focused: false,
                    title: "Connecting to BLE devices",
                    route_active,
                    ..Default::default()
                }));

            f.render_widget(loading_placeholder, area);
            return Ok(());
        };

        let filtered_peripherals: StableIndexList<PeripheralId, HandledPeripheral> = peripherals
            .iter()
            .filter(|peripheral| self.filter_peripherals(peripheral))
            .collect();

        self.list_state
            .stabilize_selected_index(&filtered_peripherals);

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

        let input = Paragraph::new(Line::from(vec![
            Span::styled(" /", Style::default().fg(Color::DarkGray)),
            Span::from(self.search.as_deref().unwrap_or("")),
        ]))
        .style(Style::default())
        .block(tui::widgets::Block::from(BlendrBlock {
            route_active,
            focused: matches!(self.focus, Focus::Search),
            title: "Filter with regex (press \"/\" to focus)",
            ..Default::default()
        }));

        f.render_widget(input, chunks[0]);

        if !self.first_match_done && filtered_peripherals.len() == 1 && self.search.is_some() {
            self.list_state.select(&filtered_peripherals, Some(0));
            Route::PeripheralWaitingView {
                peripheral: filtered_peripherals[0].clone(),
                retry: Arc::new(AtomicU16::new(0)),
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
        let title = format!("Latest Scan on {}", sync_time.format("%H:%M:%S"));
        let items = List::new(items)
            .block(tui::widgets::Block::from(BlendrBlock {
                route_active,
                focused: matches!(self.focus, Focus::List),
                title: title.as_str(),
                ..Default::default()
            }))
            .highlight_style(
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            );

        // We can now render the item list
        f.render_stateful_widget(items, chunks[1], self.list_state.get_ratatui_state());
        if chunks[2].height > 0 {
            f.render_widget(
                block::render_help([
                    Some(("q", "Quit", false)),
                    Some(("u", "Hide unknown devices", self.to_remove_unknowns)),
                    Some(("->", "Connect to device", false)),
                    Some(("r", "Restart scan", false)),
                    Some(("s", "Sort by name", *self.ctx.sort_by_name.lock().unwrap())),
                    Some(("h/j or arrows", "Navigate", false)),
                ]),
                chunks[2],
            );
        }

        Ok(())
    }
}
