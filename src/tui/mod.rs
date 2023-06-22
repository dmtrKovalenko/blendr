mod peripheral_list;
mod peripheral_view;
mod welcome;
mod ui;
use crate::{error::Result, route::Route, tui::peripheral_view::PeripheralView};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{
    io::{self, Stdout},
    ops::Deref,
    sync::Arc,
    time::{Duration, Instant},
};
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    Frame, Terminal,
};

use crate::{tui::peripheral_list::PeripheralList, Ctx};

struct App {
    ctx: Arc<Ctx>,
    peripheral_list: peripheral_list::PeripheralList,
    peripheral_view: peripheral_view::PeripheralView,
    welcome_screen: welcome::WelcomeBlock,
}

enum BlockVariant<T> {
    Primary(T),
    Secondary(T),
}

impl<T> BlockVariant<T> {
    fn into_inner(self) -> T {
        match self {
            BlockVariant::Primary(inner) => inner,
            BlockVariant::Secondary(inner) => inner,
        }
    }
}

impl App {
    fn get_active_blocks(&mut self) -> Vec<BlockVariant<&mut dyn AppRoute>> {
        match self.ctx.get_active_route().deref() {
            Route::PeripheralList => vec![
                BlockVariant::Primary(&mut self.peripheral_list),
                BlockVariant::Secondary(&mut self.welcome_screen),
            ],
            // When peripheral is not yet connected we share controls for both blocks to be able at the same time navigate and disconnect
            Route::PeripheralWaitingView { .. } => {
                vec![
                    BlockVariant::Primary(&mut self.peripheral_list),
                    BlockVariant::Primary(&mut self.peripheral_view),
                ]
            }
            Route::PeripheralConnectedView(_) => {
                vec![
                    BlockVariant::Secondary(&mut self.peripheral_list),
                    BlockVariant::Primary(&mut self.peripheral_view),
                ]
            }
            Route::CharacteristicView(_) => {
                vec![
                    BlockVariant::Secondary(&mut self.peripheral_list),
                    BlockVariant::Primary(&mut self.peripheral_view),
                ]
            }
        }
    }
}

pub type TerminalBackend = CrosstermBackend<Stdout>;

trait AppRoute {
    fn new(ctx: Arc<Ctx>) -> Self
    where
        Self: Sized;
    fn handle_input(&mut self, key: &KeyEvent);
    fn render(&mut self, area: Rect, is_active: bool, f: &mut Frame<TerminalBackend>)
        -> Result<()>;
}

pub fn run_tui_app(ctx: Arc<Ctx>) -> Result<()> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    // create app and run it
    let tick_rate = Duration::from_millis(250);

    let app = App {
        ctx: Arc::clone(&ctx),
        peripheral_list: PeripheralList::new(Arc::clone(&ctx)),
        peripheral_view: PeripheralView::new(Arc::clone(&ctx)),
        welcome_screen: welcome::WelcomeBlock::new(ctx),
    };

    let res = tui_loop(&mut terminal, app, tick_rate);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{}", err)
    }

    Ok(())
}

fn tui_loop(
    terminal: &mut Terminal<TerminalBackend>,
    mut app: App,
    tick_rate: Duration,
) -> Result<()> {
    let mut last_tick = Instant::now();
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    _ => {}
                }

                app.get_active_blocks()
                    .into_iter()
                    .filter(|block| matches!(block, BlockVariant::Primary(_)))
                    .for_each(|block| {
                        block.into_inner().handle_input(&key);
                    });
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }
}

fn ui(f: &mut Frame<TerminalBackend>, app: &mut App) {
    // Create two chunks with equal horizontal screen space
    let active_blocks = app.get_active_blocks();
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(f.size());

    for (i, block) in active_blocks.into_iter().enumerate() {
        let is_active = matches!(block, BlockVariant::Primary(_));
        block.into_inner().render(chunks[i], is_active, f);
    }
}
