use crate::{
    bluetooth::ConnectedCharacteristic,
    route::{CharacteristicValue, Route},
    tui::{
        ui::{
            block::{self, Title},
            BlendrBlock,
        },
        AppRoute, HandleKeydownResult,
    },
    Ctx,
};
use byteorder::{ByteOrder, LittleEndian, ReadBytesExt};
use copypasta::{ClipboardContext, ClipboardProvider};
use crossterm::event::KeyCode;
use lazy_static::__Deref;
use std::{io::Cursor, sync::Arc};
use tui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
};

pub struct ConnectionView {
    ctx: Arc<Ctx>,
    float_numbers: bool,
    unsigned_numbers: bool,
    highlight_copy_char_renders_delay_stack: u8,
    highlight_copy_service_renders_delay_stack: u8,
    clipboard: Option<ClipboardContext>,
}

fn try_parse_numeric_value<T: ByteOrder>(
    buf: &mut Cursor<&[u8]>,
    float: bool,
    unsigned: bool,
) -> std::io::Result<(&'static str, String)> {
    Ok(match buf.get_ref().len() {
        // 1 byte
        1 if unsigned => ("u8", buf.read_u8()?.to_string()),
        1 => ("i8", buf.read_i8()?.to_string()),
        // 4 bytes
        4 if unsigned => ("u32", buf.read_u32::<T>()?.to_string()),
        4 if float => ("f32", buf.read_f32::<T>()?.to_string()),
        4 => ("i32", buf.read_i32::<T>()?.to_string()),
        // 8 bytes
        8 if unsigned => ("u64", buf.read_u64::<T>()?.to_string()),
        8 if float => ("f64", buf.read_f64::<T>()?.to_string()),
        8 => ("i64", buf.read_i64::<T>()?.to_string()),
        // 16 bytes
        16 if unsigned => ("u128", buf.read_u128::<T>()?.to_string()),
        16 => ("i128", buf.read_i128::<T>()?.to_string()),
        _ => return Err(std::io::ErrorKind::InvalidData.into()),
    })
}

fn render_title_with_navigation_controls(
    area: &tui::layout::Rect,
    char: &ConnectedCharacteristic,
    historical_view_index: Option<usize>,
    history: &[CharacteristicValue],
) -> Title<'static> {
    const PREVIOUS_BUTTON: &str = " [<- Previous]  ";
    const PREVIOUS_BUTTON_DENSE: &str = " [<-]  ";
    const NEXT_BUTTON: &str = "  [Next ->] ";
    const NEXT_BUTTON_DENSE: &str = "  [->] ";

    let mut spans = vec![];
    let available_width = area.width - 2; // 2 chars for borders on the left and right
    let base_title = format!(
        "Characteristic {} / Service {}",
        char.char_name(),
        char.service_name()
    );

    let previous_button_style = if history.len() < 2 || historical_view_index == Some(0) {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default()
    };

    let next_button_style = if history.len() < 2 || historical_view_index.is_none() {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default()
    };

    let not_enough_space = (available_width as i32).saturating_sub(
        PREVIOUS_BUTTON.len() as i32 + NEXT_BUTTON.len() as i32 + base_title.len() as i32,
    ) < 0;

    if not_enough_space {
        spans.push(Span::styled(PREVIOUS_BUTTON_DENSE, previous_button_style));
        spans.push(Span::raw(format!("Char. {}", char.char_name())));
        spans.push(Span::styled(NEXT_BUTTON_DENSE, next_button_style));
    } else {
        spans.push(Span::styled(PREVIOUS_BUTTON, previous_button_style));
        spans.push(Span::raw(base_title));
        spans.push(Span::styled(NEXT_BUTTON, next_button_style));
    }

    Title::new(spans)
}

impl AppRoute for ConnectionView {
    fn new(ctx: std::sync::Arc<crate::Ctx>) -> Self
    where
        Self: Sized,
    {
        ConnectionView {
            ctx,
            float_numbers: false,
            unsigned_numbers: false,
            highlight_copy_char_renders_delay_stack: 0,
            highlight_copy_service_renders_delay_stack: 0,
            clipboard: ClipboardContext::new().ok(),
        }
    }

    fn handle_input(&mut self, key: &crossterm::event::KeyEvent) -> HandleKeydownResult {
        match key.code {
            KeyCode::Char('f') => {
                self.float_numbers = !self.float_numbers;
                return HandleKeydownResult::Handled;
            }
            KeyCode::Char('u') => {
                self.unsigned_numbers = !self.unsigned_numbers;
                return HandleKeydownResult::Handled;
            }
            _ => (),
        }

        let active_route = self.ctx.get_active_route();

        if let Route::CharacteristicView {
            historical_view_index,
            history,
            ..
        } = active_route.deref()
        {
            let update_index = |new_index| {
                historical_view_index.write(new_index);
            };

            match (
                key.code,
                history.read().ok().as_ref(),
                historical_view_index.deref().read(),
            ) {
                (KeyCode::Left, _, Some(current_historical_index)) => {
                    if current_historical_index >= 1 {
                        update_index(current_historical_index - 1);
                    }
                }
                (KeyCode::Left, Some(history), None) => {
                    update_index(history.len() - 1);
                }
                (KeyCode::Char('l'), _, _) => historical_view_index.annulate(),
                (KeyCode::Right, Some(history), Some(current_historical_index))
                    if current_historical_index == history.len() - 2 =>
                {
                    historical_view_index.annulate();
                }
                (KeyCode::Right, Some(history), Some(current_historical_index)) => {
                    if history.len() > current_historical_index {
                        update_index(current_historical_index + 1);
                    }
                }
                _ => (),
            }

            if matches!(key.code, KeyCode::Left | KeyCode::Right) {
                // on this view we always handing arrows as history navigation and preventing other view's actions
                return HandleKeydownResult::Handled;
            }
        }

        match (active_route.deref(), self.clipboard.as_mut()) {
            (Route::CharacteristicView { characteristic, .. }, Some(clipboard)) => match key.code {
                KeyCode::Char('c') => {
                    let _ = clipboard.set_contents(characteristic.uuid.to_string());
                    self.highlight_copy_char_renders_delay_stack = 4;
                }
                KeyCode::Char('s') => {
                    let _ = clipboard.set_contents(characteristic.service_uuid.to_string());
                    self.highlight_copy_service_renders_delay_stack = 4;
                }
                _ => (),
            },
            _ => (),
        }

        HandleKeydownResult::Continue
    }

    fn render(
        &mut self,
        area: tui::layout::Rect,
        route_active: bool,
        f: &mut tui::Frame<super::TerminalBackend>,
    ) -> crate::error::Result<()> {
        let active_route = self.ctx.active_route.read()?;
        let (_, characteristic, history, historical_view_index) =
            if let Route::CharacteristicView {
                peripheral,
                characteristic,
                history,
                historical_view_index,
            } = active_route.deref()
            {
                (peripheral, characteristic, history, historical_view_index)
            } else {
                tracing::error!(
                    "ConnectionView::render called when active route is not CharacteristicView"
                );

                return Ok(());
            };

        let history = history.read()?;
        let historical_index = historical_view_index.deref().read();

        let active_value = match historical_index {
            Some(index) => history.get(index),
            None => history.last(),
        };

        let mut text = vec![];
        if let Some(value) = active_value.as_ref() {
            text.push(Line::from(""));

            text.push(Line::from(format!(
                "{label}: {}",
                value.time.format("%Y-%m-%d %H:%M:%S"),
                label = if let Some(index) = historical_index {
                    format!(
                        "Historical data ({} of {}) viewing data of\n",
                        index + 1,
                        history.len()
                    )
                } else {
                    "Latest value received".to_owned()
                },
            )));

            text.push(Line::from(""));

            if let Ok(string_value) = String::from_utf8(value.data.clone()) {
                if !string_value.is_empty() {
                    text.push(Line::from("UTF-8 text"));
                    text.push(Line::from(Span::styled(
                        string_value,
                        Style::default().fg(Color::Cyan),
                    )));
                    text.push(Line::from(""));
                }
            }

            let mut cursor = std::io::Cursor::new(&value.data[..]);
            if let Ok((type_label, value)) = try_parse_numeric_value::<LittleEndian>(
                &mut cursor,
                self.float_numbers,
                self.unsigned_numbers,
            ) {
                text.push(Line::from(vec![
                    Span::raw("inferred as "),
                    Span::styled(type_label, Style::default().add_modifier(Modifier::BOLD)),
                ]));

                text.push(Line::from(Span::styled(
                    value,
                    Style::default().fg(Color::Green),
                )));

                text.push(Line::from(""));
            }

            let mut hexyl_output_buf = Vec::new();
            {
                let mut writer = std::io::Cursor::new(&mut hexyl_output_buf);
                let mut printer = hexyl::PrinterBuilder::new(&mut writer)
                    .show_color(true)
                    .num_panels(if area.width > 70 { 2 } else { 1 })
                    .with_border_style(hexyl::BorderStyle::Unicode)
                    .build();

                printer.print_all(&value.data[..]).unwrap();
            }

            use ansi_to_tui::IntoText;
            if let Ok(output) = hexyl_output_buf.into_text() {
                text.extend(output);
            } else {
                tracing::error!(
                    ?hexyl_output_buf,
                    "Failed to parse and display hexyl output"
                );
            }
        } else {
            text.push(Line::from("No value received yet"));
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Min(10),
                    Constraint::Length(if route_active { 3 } else { 0 }),
                ]
                .as_ref(),
            )
            .split(area);

        let paragraph =
            Paragraph::new(text)
                .wrap(Wrap { trim: true })
                .block(tui::widgets::Block::from(BlendrBlock {
                    route_active,
                    focused: route_active,
                    title_alignment: tui::layout::Alignment::Center,
                    title: render_title_with_navigation_controls(
                        &area,
                        characteristic,
                        historical_index,
                        &history,
                    ),
                    ..Default::default()
                }));

        f.render_widget(paragraph, chunks[0]);
        if chunks[1].height > 0 {
            f.render_widget(
                block::render_help(
                    Arc::clone(&self.ctx),
                    [
                        Some(("<-", "Previous value", false)),
                        Some(("->", "Next value", false)),
                        Some(("d", "[D]isconnect from device", false)),
                        Some(("u", "Parse numeric as [u]nsigned", self.unsigned_numbers)),
                        Some(("f", "Parse numeric as [f]loats", self.float_numbers)),
                        historical_index.map(|_| {
                            (
                                "l",
                                "Go to the [l]atest values",
                                self.highlight_copy_char_renders_delay_stack > 0,
                            )
                        }),
                        self.clipboard.as_ref().map(|_| {
                            (
                                "c",
                                "Copy [c]haracteristic UUID",
                                self.highlight_copy_char_renders_delay_stack > 0,
                            )
                        }),
                        self.clipboard.as_ref().map(|_| {
                            (
                                "s",
                                "Copy [s]ervice UUID",
                                self.highlight_copy_service_renders_delay_stack > 0,
                            )
                        }),
                    ],
                ),
                chunks[1],
            );
        }

        if self.highlight_copy_char_renders_delay_stack > 0 {
            self.highlight_copy_char_renders_delay_stack -= 1;
        }

        if self.highlight_copy_service_renders_delay_stack > 0 {
            self.highlight_copy_service_renders_delay_stack -= 1;
        }

        Ok(())
    }
}
