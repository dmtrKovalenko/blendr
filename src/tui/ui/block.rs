use tui::{
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Paragraph, Widget, Wrap},
};

#[derive(Debug, Default)]
pub struct BlendrBlock<T: std::fmt::Display> {
    pub focused: bool,
    pub title: T,
    pub route_active: bool,
}

impl<'a, Title: std::fmt::Display> From<BlendrBlock<Title>> for tui::widgets::Block<'a> {
    fn from(block: BlendrBlock<Title>) -> Self {
        tui::widgets::Block::default()
            .title(format!(" {} ", block.title))
            .border_style(if block.route_active && block.focused {
                Style::default().fg(tui::style::Color::LightBlue)
            } else {
                Style::default()
            })
            .border_type(tui::widgets::BorderType::Rounded)
            .borders(tui::widgets::Borders::ALL)
    }
}

pub fn render_help<const N: usize>(help: [(&str, &str, bool); N]) -> impl Widget {
    let spans: Vec<_> = help
        .into_iter()
        .map(|(key, text, bold)| {
            const SPACING: &str = "    ";
            let mut key_span = Span::from(format!("[{key}] {text}{SPACING}"));

            if bold {
                key_span.style = Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(Color::Cyan);
            }

            // 4 spaces is a good spacing between the two helpers
            key_span
        })
        .collect();

    Paragraph::new(Spans(spans))
        .style(Style::default().fg(Color::DarkGray))
        .wrap(Wrap { trim: true })
}
