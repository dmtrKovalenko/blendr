use crate::{
    error,
    tui::{ui::BlendrBlock, AppRoute, HandleKeydownResult, TerminalBackend},
    Ctx,
};
use crossterm::event::KeyCode;
use lazy_static::__Deref;
use std::{ops::DerefMut, sync::Arc};
use tui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::Line,
    widgets::{Clear, Paragraph},
    Frame,
};

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}

pub struct ErrorView {
    ctx: Arc<Ctx>,
}

impl AppRoute for ErrorView {
    fn new(ctx: Arc<Ctx>) -> Self
    where
        Self: Sized,
    {
        ErrorView { ctx }
    }

    fn handle_input(&mut self, key: &crossterm::event::KeyEvent) -> HandleKeydownResult {
        // unwrap here because we are already in error state and if can not get out of it – it means a super serious race condition
        let mut global_error_lock = self.ctx.global_error.lock().unwrap();
        if global_error_lock.deref().is_none() {
            return HandleKeydownResult::Errored;
        }

        match key.code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Tab | KeyCode::Char(' ') => {
                *global_error_lock.deref_mut() = None;
                HandleKeydownResult::Handled
            }
            _ => HandleKeydownResult::Continue,
        }
    }

    fn render(
        &mut self,
        _area: Rect,
        _is_active: bool,
        f: &mut Frame<TerminalBackend>,
    ) -> error::Result<()> {
        let global_error_lock = self.ctx.global_error.lock().unwrap();
        let error = if let Some(error) = global_error_lock.deref() {
            error
        } else {
            return Ok(());
        };

        let area = centered_rect(60, 20, f.size());
        f.render_widget(Clear, area); //this clears out the background

        let paragraph = Paragraph::new(vec![Line::from(""), Line::from(format!("{error}"))]).block(
            tui::widgets::Block::from(BlendrBlock {
                focused: true,
                route_active: true,
                title: "Error",
                color: Some(tui::style::Color::Red),
                ..Default::default()
            }),
        );

        f.render_widget(paragraph, area);

        Ok(())
    }
}
