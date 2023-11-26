use crate::{cli_args, Ctx};
use std::sync::Arc;
use tui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget, Wrap},
};

#[derive(Debug, Default)]
pub struct Title<'a>(Vec<Span<'a>>);

impl<'a> Title<'a> {
    pub fn new(vec: Vec<impl Into<Span<'a>>>) -> Self {
        Self(vec.into_iter().map(Into::into).collect())
    }
}

impl<'a, T: Into<Span<'a>>> From<T> for Title<'a> {
    fn from(title: T) -> Self {
        Self(vec![Span::from(" "), title.into(), Span::from(" ")])
    }
}

#[derive(Debug)]
pub struct BlendrBlock<'a, T: Into<Title<'a>> + Default> {
    pub focused: bool,
    pub title: T,
    pub route_active: bool,
    pub color: Option<Color>,
    pub title_alignment: tui::layout::Alignment,
    pub phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, T: Into<Title<'a>> + Default> Default for BlendrBlock<'a, T> {
    fn default() -> Self {
        Self {
            focused: Default::default(),
            title: T::default(),
            route_active: Default::default(),
            color: Default::default(),
            title_alignment: tui::layout::Alignment::Left,
            phantom: Default::default(),
        }
    }
}

impl<'a, TTitle: Into<Title<'a>> + Default> From<BlendrBlock<'a, TTitle>>
    for tui::widgets::Block<'a>
{
    fn from(block: BlendrBlock<'a, TTitle>) -> Self {
        let title: Title = block.title.into();
        tui::widgets::Block::default()
            .title(Line::from(title.0))
            .title_alignment(block.title_alignment)
            .border_style(if block.route_active && block.focused {
                Style::default()
                    .fg(block.color.unwrap_or(tui::style::Color::LightBlue))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            })
            .border_type(tui::widgets::BorderType::Rounded)
            .borders(tui::widgets::Borders::ALL)
    }
}

pub fn render_help<const N: usize>(
    ctx: Arc<Ctx>,
    help: [Option<(&str, &str, bool)>; N],
) -> impl Widget {
    const SPACING: &str = "    ";
    let general_options_guard = &ctx.general_options.read();
    let general_options = general_options_guard.as_ref().unwrap();

    let general_options_spans = [
        Span::from("Sort by: "),
        Span::styled(
            "[n]ame",
            if general_options.sort == cli_args::GeneralSort::Name {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            },
        ),
        Span::from(" | "),
        Span::styled(
            "default",
            if general_options.sort == cli_args::GeneralSort::DefaultSort {
                Style::default().add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            },
        ),
        Span::from(SPACING),
    ];

    let spans: Vec<_> = help
        .into_iter()
        .flatten()
        .map(|(key, text, bold)| {
            let mut key_span = Span::from(format!("[{key}] {text}{SPACING}"));

            if bold {
                key_span.style = Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(Color::Cyan);
            }

            // 4 spaces is a good spacing between the two helpers
            key_span
        })
        .chain(general_options_spans.into_iter())
        .collect();

    Paragraph::new(Line::from(spans))
        .style(Style::default().fg(Color::Gray))
        .wrap(Wrap { trim: true })
}
