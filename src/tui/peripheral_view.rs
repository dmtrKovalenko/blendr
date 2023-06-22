use btleplug::api::{Characteristic, Peripheral};
use crossterm::event::KeyCode;
use regex::Regex;
use tui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{List, ListItem, ListState, Paragraph},
};

use crate::{
    bluetooth::{
        ble_default_services::{SPECIAL_CHARACTERISTICS_NAMES, SPECIAL_SERVICES_NAMES},
        ConnectedCharacteristic,
    },
    route::Route,
    tui::{
        block::{self, BlendrBlock},
        list,
        search_input::{self, ShouldUpdate},
        AppRoute,
    },
    Ctx,
};
use std::{collections::BTreeSet, ops::Deref, sync::Arc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Focus {
    Search,
    List,
}

#[derive(Debug)]
pub struct PeripheralView {
    ctx: Arc<Ctx>,
    list_state: ListState,
    pub focus: Focus,
    pub search: Option<String>,
    pub search_regex: Option<Regex>,
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
        PeripheralView {
            search: ctx.args.characteristic.clone(),
            search_regex: None,
            focus: Focus::List,
            list_state: ListState::default(),
            ctx,
        }
    }

    fn handle_input(&mut self, key: &crossterm::event::KeyEvent) {
        let last_search = self.search.clone();
        let active_route = self.ctx.get_active_route();

        match active_route.deref() {
            Route::PeripheralWaitingView { .. } => {
                if matches!(key.code, KeyCode::Left | KeyCode::Char('d')) {
                    drop(active_route);
                    Route::PeripheralList.navigate(&self.ctx);
                }
            }

            Route::PeripheralConnectedView(peripheral) => {
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
                        let filtered_peripherals = peripheral
                            .characteristics
                            .iter()
                            .filter(|peripheral| self.filter_characteristic(peripheral))
                            .collect::<Vec<_>>();

                        list::handle_key_input(
                            &filtered_peripherals,
                            &key.code,
                            &mut self.list_state,
                            |peripheral| {
                                // self.ctx.navigate(Route::PeripheralView(peripheral.clone()));
                            },
                        );

                        match key.code {
                            KeyCode::Char('/') => {
                                self.focus = Focus::Search;
                                list::list_unselect(&mut self.list_state)
                            }
                            KeyCode::Left | KeyCode::Char('d') => {
                                drop(active_route);
                                Route::PeripheralList.navigate(&self.ctx)
                            }
                            _ => {}
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
            Route::PeripheralWaitingView { peripheral, .. } => {
                let loading_placeholder = Paragraph::new(Spans::from("In progress..."))
                    .style(Style::default())
                    .block(tui::widgets::Block::from(BlendrBlock {
                        focused: false,
                        title: format!("Connecting to {}", peripheral.name),
                        route_active,
                    }));

                f.render_widget(loading_placeholder, area);
                return Ok(());
            }

            _ => {
                return Err(crate::error::Error::client("Invalid route"));
            }
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
            title: "Filter services or characteristics",
        }));

        f.render_widget(input, chunks[0]);

        let filtered_chars = connection
            .characteristics
            .iter()
            .filter(|characteristic| self.filter_characteristic(characteristic))
            .collect::<Vec<_>>();

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
                    if let Some(name) = char.standard_gatt_service_name {
                        spans.push(Spans::from(name));
                    } else {
                        spans.push(Spans::from(format!("Service {}", char.service_uuid)));
                    }
                }

                let is_highlighted = Some(i) == self.list_state.selected();
                let style = is_highlighted
                    .then(|| {
                        Style::default()
                            .bg(Color::LightBlue)
                            .add_modifier(Modifier::BOLD)
                    })
                    .unwrap_or_default();

                let char_uuid = char.uuid.to_string();
                let char_name = if let Some(name) = char.standard_gatt_char_name {
                    name
                } else {
                    char_uuid.as_str()
                };

                // need to fill out the whole line to make highlight work as on general list items
                let mut spacer =
                    String::with_capacity(chunks[1].width as usize - 4 - char_name.len());
                while spacer.len() < spacer.capacity() {
                    spacer.push(' ');
                }

                spans.push(Spans::from(Span::styled(
                    format!(
                        "{}  {}{spacer}",
                        if is_highlighted { ">" } else { "•" },
                        char_name
                    ),
                    style,
                )));

                ListItem::new(spans).style(Style::default().fg(Color::Gray))
            })
            .collect();

        // Create a List from all list items and highlight the currently selected one
        let items = List::new(items).block(tui::widgets::Block::from(BlendrBlock {
            route_active,
            focused: matches!(self.focus, Focus::List),
            title: format!(
                " Device {} ({}) ",
                connection.peripheral.name,
                connection.peripheral.ble_peripheral.address()
            ),
        }));

        // We can now render the item list
        f.render_stateful_widget(items, chunks[1], &mut self.list_state);
        if chunks[2].height > 0 {
            f.render_widget(
                block::render_help([
                    ("/", "Search", false),
                    ("<- | d", "Disconnect from device", false),
                    ("->", "View characteristic", false),
                    ("r", "Reconnect to device scan", false),
                ]),
                chunks[2],
            );
        }

        Ok(())
    }
}
