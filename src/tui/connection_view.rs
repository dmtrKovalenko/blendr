use crate::{
    bluetooth::display_properties,
    route::Route,
    tui::{
        ui::{block, BlendrBlock},
        AppRoute,
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
    highlight_copy_char_stack: u8,
    highlight_copy_service_stack: u8,
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

impl AppRoute for ConnectionView {
    fn new(ctx: std::sync::Arc<crate::Ctx>) -> Self
    where
        Self: Sized,
    {
        ConnectionView {
            ctx,
            float_numbers: false,
            unsigned_numbers: false,
            highlight_copy_char_stack: 0,
            highlight_copy_service_stack: 0,
            clipboard: ClipboardContext::new().ok(),
        }
    }

    fn handle_input(&mut self, key: &crossterm::event::KeyEvent) {
        match key.code {
            KeyCode::Char('f') => {
                self.float_numbers = !self.float_numbers;
                return;
            }
            KeyCode::Char('u') => {
                self.unsigned_numbers = !self.unsigned_numbers;
                return;
            }
            _ => (),
        }

        let active_route = self.ctx.get_active_route();

        match (active_route.deref(), self.clipboard.as_mut()) {
            (Route::CharacteristicView { characteristic, .. }, Some(clipboard)) => match key.code {
                KeyCode::Char('c') => {
                    let _ = clipboard.set_contents(characteristic.uuid.to_string());
                    self.highlight_copy_char_stack = 4;
                }
                KeyCode::Char('s') => {
                    let _ = clipboard.set_contents(characteristic.service_uuid.to_string());
                    self.highlight_copy_service_stack = 4;
                }
                _ => (),
            },
            _ => (),
        }
    }

    fn render(
        &mut self,
        area: tui::layout::Rect,
        route_active: bool,
        f: &mut tui::Frame<super::TerminalBackend>,
    ) -> crate::error::Result<()> {
        let active_route = self.ctx.active_route.read()?;
        let (_, characteristic, value) = if let Route::CharacteristicView {
            peripheral,
            characteristic,
            value,
        } = active_route.deref()
        {
            (peripheral, characteristic, value)
        } else {
            tracing::error!(
                "ConnectionView::render called when active route is not CharacteristicView"
            );

            return Ok(());
        };

        let mut text = vec![];
        text.push(Line::from(""));

        text.push(Line::from(format!(
            "Properties: {}",
            display_properties(characteristic.ble_characteristic.properties)
        )));

        if let Some(value) = value.read().unwrap().as_ref() {
            text.push(Line::from(""));

            text.push(Line::from(format!(
                "Last updated: {}",
                value.time.format("%Y-%m-%d %H:%M:%S")
            )));

            text.push(Line::from(""));

            if let Ok(string_value) = String::from_utf8(value.data.clone()) {
                text.push(Line::from("UTF-8 text"));
                text.push(Line::from(Span::styled(
                    string_value,
                    Style::default().fg(Color::Cyan),
                )));
                text.push(Line::from(""));
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
                    .num_panels(if area.width > 60 { 2 } else { 1 })
                    .with_border_style(hexyl::BorderStyle::Unicode)
                    .build();

                printer.print_all(&value.data[..]).unwrap();
            }

            use ansi_to_tui::IntoText;
            let a = hexyl_output_buf.into_text().unwrap().into_iter();
            text.extend(a);
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
                    title: format!(
                        "Service {} / Characteristic {}",
                        characteristic.service_name(),
                        characteristic.char_name()
                    ),
                    ..Default::default()
                }));

        f.render_widget(paragraph, chunks[0]);
        if chunks[1].height > 0 {
            f.render_widget(
                block::render_help([
                    Some(("d", "Disconnect from device", false)),
                    Some(("u", "Parse numeric as unsigned", self.unsigned_numbers)),
                    Some(("f", "Parse numeric as floats", self.float_numbers)),
                    self.clipboard.as_ref().map(|_| {
                        (
                            "c",
                            "Copy characteristic UUID",
                            self.highlight_copy_char_stack > 0,
                        )
                    }),
                    self.clipboard.as_ref().map(|_| {
                        (
                            "s",
                            "Copy services UUID",
                            self.highlight_copy_service_stack > 0,
                        )
                    }),
                ]),
                chunks[1],
            );
        }

        if self.highlight_copy_char_stack > 0 {
            self.highlight_copy_char_stack -= 1;
        }

        if self.highlight_copy_service_stack > 0 {
            self.highlight_copy_service_stack -= 1;
        }

        Ok(())
    }
}
