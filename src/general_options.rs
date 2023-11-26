use std::sync::Arc;

use cli_args::GeneralSort;
use crossterm::event::KeyCode;

use crate::{
    cli_args::{self, Args},
    Ctx,
};

#[derive(Default, Debug)]
pub struct GeneralOptions {
    pub sort: GeneralSort,
}

impl GeneralOptions {
    pub fn new(args: &Args) -> Self {
        Self {
            sort: args.sort.unwrap_or_default(),
        }
    }

    pub fn handle_keystroke(keycode: &KeyCode, ctx: &Arc<Ctx>) -> bool {
        match keycode {
            KeyCode::Char('n') => {
                let mut general_options = ctx.general_options.write().unwrap();
                general_options.sort = match general_options.sort {
                    GeneralSort::Name => GeneralSort::DefaultSort,
                    GeneralSort::DefaultSort => GeneralSort::Name,
                };

                true
            }
            _ => false,
        }
    }
}
