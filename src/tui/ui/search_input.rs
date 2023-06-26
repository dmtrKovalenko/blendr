use regex::Regex;

use crate::Ctx;

use crossterm::event::KeyCode;

use crossterm::event::KeyEvent;

pub fn handle_search_input(current_search: &mut Option<String>, key: &KeyEvent) {
    match (key.code, current_search.as_mut()) {
        (KeyCode::Char(c), Some(search)) => search.push(c),
        (KeyCode::Backspace, Some(search)) => {
            search.pop();
        }
        (KeyCode::Char(c), None) => *current_search = Some(c.to_string()),
        _ => {}
    }
}

pub enum ShouldUpdate<T> {
    Update(T),
    NoUpdate,
}

/// Do not parse and compile regex if nothing changes
pub fn maybe_update_search_regexp(
    search: Option<&str>,
    last_search: Option<String>,
    ctx: &Ctx,
) -> ShouldUpdate<Option<Regex>> {
    match search {
        None => ShouldUpdate::Update(None),
        Some(search) if search.is_empty() => ShouldUpdate::Update(None),
        Some(search) if Some(search) != last_search.as_deref() => {
            let regex = Regex::new(&format!("{}{search}", ctx.args.regex_flags))
                .map_err(|e| {
                    tracing::error!(?e, "Failed to create regex");
                    e
                })
                .ok();

            ShouldUpdate::Update(regex)
        }
        _ => ShouldUpdate::NoUpdate,
    }
}
