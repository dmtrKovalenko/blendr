use crossterm::event::KeyCode;
use tui::widgets::ListState;

pub fn list_select_next<T>(list: &[T], state: &mut ListState) {
    if list.is_empty() {
        return;
    }

    let i = state.selected().unwrap_or(0);
    let next = if i >= list.len() - 1 { 0 } else { i + 1 };
    state.select(Some(next));
}

pub fn list_select_previous<T>(list: &[T], state: &mut ListState) {
    if list.is_empty() {
        return;
    }

    let i = state.selected().unwrap_or(0);
    let next = if i == 0 { list.len() - 1 } else { i - 1 };
    state.select(Some(next));
}

pub fn list_unselect(state: &mut ListState) {
    state.select(None);
}

pub fn handle_key_input<T: Copy>(
    list: &[T],
    keycode: &KeyCode,
    state: &mut ListState,
    on_select: impl FnOnce(T),
) {
    match keycode {
        KeyCode::Left | KeyCode::Char('h') => list_unselect(state),
        KeyCode::Right | KeyCode::Char('l') | KeyCode::Enter => {
            if let Some(selected_index) = state.selected() {
                if selected_index >= list.len() {
                    return;
                }
                
                let a = list[selected_index];
                on_select(a);
            }
        }
        KeyCode::Down | KeyCode::Char('j') => list_select_next(list, state),
        KeyCode::Up | KeyCode::Char('k') => list_select_previous(list, state),
        _ => (),
    }
}
