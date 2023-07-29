use crossterm::event::KeyCode;
use regex::Regex;
use tui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, Paragraph},
};
use uuid::Uuid;

use crate::{
    bluetooth::{display_properties, ConnectedCharacteristic},
    route::Route,
    tui::AppRoute,
    tui::{
        ui::{
            block::{self, BlendrBlock},
            list::{HandleInputResult, StableListState},
            search_input::{self, ShouldUpdate},
        },
        HandleKeydownResult,
    },
    Ctx,
};
use std::{
    ops::Deref,
    sync::{Arc, RwLock},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Focus {
    Search,
    List,
}

#[derive(Debug)]
pub struct PeripheralView {
    ctx: Arc<Ctx>,
    list_state: StableListState<Uuid>,
    focus: Focus,
    search: Option<String>,
    search_regex: Option<Regex>,
    first_match_done: bool,
}

impl PeripheralView {
    fn filter_characteristic(&self, characteristic: &ConnectedCharacteristic) -> bool {
        let characteristic_name = characteristic.uuid.to_string();
        let service_name = characteristic.service_uuid.to_string();

        if let Some(regex) = self.search_regex.as_ref() {
            let uuids_match = regex.is_match(&characteristic.uuid.to_string())
                || regex.is_match(&characteristic.service_uuid.to_string());

            let name_match = characteristic
                .standard_gatt_char_name
                .is_some_and(|name| regex.is_match(name));
            let service_match = characteristic
                .standard_gatt_service_name
                .is_some_and(|name| regex.is_match(name));

            return uuids_match || name_match || service_match;
        }

        if let Some(search) = self.search.as_ref() {
            let uuids_match = characteristic_name.contains(search) || service_name.contains(search);

            let name_match = characteristic
                .standard_gatt_char_name
                .is_some_and(|name| name.contains(search));

            let service_match = characteristic
                .standard_gatt_service_name
                .is_some_and(|name| name.contains(search));

            return uuids_match || name_match || service_match;
        }

        true
    }
}

impl AppRoute for PeripheralView {
    fn new(ctx: std::sync::Arc<crate::Ctx>) -> Self
    where
        Self: Sized,
    {
        let initial_search = ctx.args.characteristic.clone();

        PeripheralView {
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
            first_match_done: false,
            ctx,
        }
    }

    fn handle_input(&mut self, key: &crossterm::event::KeyEvent) -> HandleKeydownResult {
        let last_search = self.search.clone();
        let active_route = self.ctx.get_active_route();

        match active_route.deref() {
            Route::PeripheralWaitingView { .. } => {
                if matches!(key.code, KeyCode::Left | KeyCode::Char('d')) {
                    drop(active_route);
                    Route::PeripheralList.navigate(&self.ctx);
                }
            }
            Route::PeripheralConnectedView(peripheral)
            | Route::CharacteristicView { peripheral, .. } => {
                let filtered_chars = peripheral
                    .characteristics
                    .iter()
                    .filter(|peripheral| self.filter_characteristic(peripheral))
                    .collect();

                self.list_state.stabilize_selected_index(&filtered_chars);

                match self.focus {
                    Focus::Search => {
                        search_input::handle_search_input(&mut self.search, key);

                        match key.code {
                            KeyCode::Enter | KeyCode::Down => {
                                self.list_state.select(&filtered_chars, Some(0));
                                self.focus = Focus::List
                            }
                            KeyCode::Esc | KeyCode::Tab => {
                                self.list_state.list_unselect(&filtered_chars);
                                self.focus = Focus::List;
                            }
                            _ => (),
                        }
                    }
                    Focus::List => {
                        match key.code {
                            KeyCode::Char('/') | KeyCode::Tab => {
                                self.focus = Focus::Search;
                                self.list_state.list_unselect(&filtered_chars);
                            }
                            KeyCode::Left | KeyCode::Char('d') => {
                                drop(active_route);
                                Route::PeripheralList.navigate(&self.ctx);
                                return HandleKeydownResult::Handled;
                            }
                            _ => {}
                        }

                        if let HandleInputResult::Selected(selected_char) =
                            self.list_state.handle_key_input(&filtered_chars, &key.code)
                        {
                            let char_clone = selected_char.clone();
                            let peripheral_clone = peripheral.clone();

                            drop(active_route);

                            Route::CharacteristicView {
                                characteristic: char_clone,
                                peripheral: peripheral_clone,
                                history: Arc::new(RwLock::new(vec![])),
                                historical_view_index: Default::default(),
                            }
                            .navigate(&self.ctx);
                        }
                    }
                }
            }

            _ => (),
        };

        if let ShouldUpdate::Update(new_regex) =
            search_input::maybe_update_search_regexp(self.search.as_deref(), last_search, &self.ctx)
        {
            self.search_regex = new_regex;
        }

        HandleKeydownResult::Continue
    }

    fn render(
        &mut self,
        area: tui::layout::Rect,
        route_active: bool,
        f: &mut tui::Frame<super::TerminalBackend>,
    ) -> crate::error::Result<()> {
        let active_route = self.ctx.get_active_route();

        let connection = match active_route.deref() {
            Route::PeripheralConnectedView(peripheral) => peripheral,
            Route::CharacteristicView { peripheral, .. } => peripheral,
            Route::PeripheralWaitingView {
                peripheral, retry, ..
            } => {
                let retry = retry.load(std::sync::atomic::Ordering::SeqCst);
                let loading_placeholder = Paragraph::new(vec![
                    Line::from(""),
                    Line::from(vec![
                        Span::raw("Establishing Connection... "),
                        if retry > 0 {
                            Span::from(format!("Retry #{retry}"))
                        } else {
                            Span::from("")
                        },
                    ]),
                ])
                .style(Style::default())
                .block(tui::widgets::Block::from(BlendrBlock {
                    focused: false,
                    title: format!("Connecting to {}", peripheral.name),
                    route_active,
                    ..Default::default()
                }));

                f.render_widget(loading_placeholder, area);
                return Ok(());
            }

            _ => {
                return Err(crate::error::Error::client("Invalid route"));
            }
        };

        let filtered_chars = connection
            .characteristics
            .iter()
            .filter(|characteristic| self.filter_characteristic(characteristic))
            .collect();

        self.list_state.stabilize_selected_index(&filtered_chars);

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
            title: "Filter services or characteristics",
            ..Default::default()
        }));

        f.render_widget(input, chunks[0]);

        let items: Vec<ListItem> = filtered_chars
            .iter()
            .enumerate()
            .map(|(i, char)| {
                let mut spans = vec![];

                // render service as a part of list but highlight only notification on selection
                let previous_char = filtered_chars.get(i.wrapping_sub(1));
                if i == 0
                    || previous_char
                        .is_some_and(|prev_char| prev_char.service_uuid != char.service_uuid)
                {
                    spans.push(Line::from(vec![
                        Span::styled("Service ", Style::default().fg(Color::White)),
                        Span::styled(
                            char.service_name(),
                            Style::default()
                                .add_modifier(Modifier::BOLD)
                                .fg(Color::Rgb(251, 146, 60)),
                        ),
                    ]));
                }

                let is_highlighted = Some(i) == self.list_state.selected();
                let base_style = is_highlighted
                    .then(|| {
                        Style::default()
                            .bg(Color::LightBlue)
                            .add_modifier(Modifier::BOLD)
                    })
                    .unwrap_or_default();

                let char_name = char.char_name();
                let mut char_line = Line::from(vec![
                    Span::styled(if is_highlighted { ">" } else { "•" }, base_style),
                    Span::styled("  ", base_style),
                    Span::styled(
                        char_name,
                        base_style.add_modifier(Modifier::BOLD).fg(Color::White),
                    ),
                    Span::styled(
                        format!(
                            " [{}]",
                            display_properties(char.ble_characteristic.properties)
                        ),
                        base_style,
                    ),
                ]);

                let mut spacer = String::new();
                // need to fill out the whole line to make highlight work as on general list items
                for _ in 0..(chunks[1].width as usize).saturating_sub(char_line.width()) {
                    spacer.push(' ');
                }

                char_line.spans.push(Span::styled(spacer, base_style));
                spans.push(char_line);

                ListItem::new(spans).style(Style::default().fg(Color::Gray))
            })
            .collect();

        // Create a List from all list items and highlight the currently selected one
        let items = List::new(items).block(tui::widgets::Block::from(BlendrBlock {
            route_active,
            focused: matches!(self.focus, Focus::List),
            title: format!(
                " Device {} ({}) ",
                connection.peripheral.name, connection.peripheral.address
            ),
            ..Default::default()
        }));

        // We can now render the item list
        f.render_stateful_widget(items, chunks[1], self.list_state.get_ratatui_state());
        if chunks[2].height > 0 {
            f.render_widget(
                block::render_help([
                    Some(("/", "Search", false)),
                    Some(("<- | d", "Disconnect from device", false)),
                    Some(("->", "View characteristic", false)),
                    Some(("r", "Reconnect to device scan", false)),
                ]),
                chunks[2],
            );
        }

        if !self.first_match_done && filtered_chars.len() == 1 {
            let peripheral = connection.clone();
            let characteristic = filtered_chars[0].clone();

            self.list_state.select(&filtered_chars, Some(0));
            drop(active_route);

            Route::CharacteristicView {
                characteristic,
                peripheral,
                history: Arc::new(RwLock::new(vec![])),
                historical_view_index: Default::default(),
            }
            .navigate(&self.ctx);
            self.first_match_done = true
        }

        Ok(())
    }
}
