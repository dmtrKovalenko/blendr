use crossterm::event::KeyCode;
use std::{collections::HashMap, hash::Hash, ops::Index};
use tui::widgets::ListState;

pub trait StableListItem<T: Eq + Hash> {
    fn id(&self) -> T;
}

pub struct StableIndexList<'a, TId: Eq + Hash, TItem: StableListItem<TId>> {
    id_to_idx: HashMap<TId, usize>,
    vec: Vec<&'a TItem>,
}

impl<'a, TId: Eq + Hash, TItem: StableListItem<TId>> Index<usize>
    for StableIndexList<'a, TId, TItem>
{
    type Output = TItem;

    fn index(&self, index: usize) -> &Self::Output {
        self.vec[index]
    }
}

impl<'a, TId: Eq + Hash, TItem: StableListItem<TId>> StableIndexList<'a, TId, TItem> {
    pub fn new() -> Self {
        Self {
            vec: Vec::new(),
            id_to_idx: HashMap::new(),
        }
    }

    pub fn get(&self, index: usize) -> Option<&'a TItem> {
        self.vec.get(index).copied()
    }

    pub fn len(&self) -> usize {
        self.vec.len()
    }

    pub fn is_empty(&self) -> bool {
        self.vec.is_empty()
    }

    pub fn push(&mut self, item: &'a TItem) {
        self.id_to_idx.insert(item.id(), self.vec.len());
        self.vec.push(item);
    }

    pub fn iter(&self) -> core::slice::Iter<'_, &'a TItem> {
        self.vec.iter()
    }
}

impl<'a, TId: Eq + Hash, TItem: StableListItem<TId>> std::iter::IntoIterator
    for StableIndexList<'a, TId, TItem>
{
    type Item = &'a TItem;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.vec.into_iter()
    }
}

impl<'a, TId: Eq + Hash, TItem: StableListItem<TId>> std::iter::FromIterator<&'a TItem>
    for StableIndexList<'a, TId, TItem>
{
    fn from_iter<I: IntoIterator<Item = &'a TItem>>(iter: I) -> Self {
        let mut collection = StableIndexList::new();

        for item in iter {
            collection.push(item);
        }

        collection
    }
}

#[derive(Debug)]
pub struct StableListState<T>
where
    T: Eq + Hash,
{
    selected_id: Option<T>,
    pub unstable_state: ListState,
}

impl<T: Eq + Hash> Default for StableListState<T> {
    fn default() -> Self {
        Self {
            selected_id: None,
            unstable_state: ListState::default(),
        }
    }
}

impl<TId: Eq + Hash> StableListState<TId> {
    pub fn stabilize_selected_index<TItem: StableListItem<TId>>(
        &mut self,
        list: &StableIndexList<'_, TId, TItem>,
    ) {
        let relative_index = self
            .selected_id
            .as_ref()
            .and_then(|id| list.id_to_idx.get(id))
            .copied();

        self.unstable_state.select(relative_index);
    }

    pub fn select<TItem: StableListItem<TId>>(
        &mut self,
        list: &StableIndexList<TId, TItem>,
        index: Option<usize>,
    ) {
        self.unstable_state.select(index);

        if let Some(index) = index {
            let selected_item = list.vec.get(index);
            self.selected_id = selected_item.map(|item| item.id());
        }
    }

    pub fn selected(&mut self) -> Option<usize> {
        self.unstable_state.selected()
    }

    pub fn list_select_next<TItem: StableListItem<TId>>(
        &mut self,
        list: &StableIndexList<TId, TItem>,
    ) {
        if list.vec.is_empty() {
            return;
        }

        let i = self.selected().unwrap_or(0);
        let next = if i >= list.vec.len() - 1 { 0 } else { i + 1 };
        self.select(list, Some(next));
    }

    pub fn list_select_previous<TItem: StableListItem<TId>>(
        &mut self,
        list: &StableIndexList<TId, TItem>,
    ) {
        if list.vec.is_empty() {
            return;
        }

        let i = self.selected().unwrap_or(0);
        let next = if i == 0 { list.vec.len() - 1 } else { i - 1 };
        self.select(list, Some(next));
    }

    pub fn list_unselect(&mut self, list: &StableIndexList<TId, impl StableListItem<TId>>) {
        self.select(list, None);
    }

    pub fn get_ratatui_state(&mut self) -> &mut ListState {
        &mut self.unstable_state
    }

    pub fn handle_key_input<'a, 'b: 'a, TItem: StableListItem<TId> + Clone>(
        &mut self,
        list: &'a StableIndexList<'b, TId, TItem>,
        keycode: &KeyCode,
    ) -> HandleInputResult<&'b TItem> {
        match keycode {
            KeyCode::Left | KeyCode::Char('h') => self.list_unselect(list),
            KeyCode::Right | KeyCode::Char('l') | KeyCode::Enter => {
                if let Some(selected_index) = self.selected() {
                    if selected_index >= list.vec.len() {
                        return HandleInputResult::None;
                    }

                    return HandleInputResult::Selected(list.vec[selected_index]);
                }
            }
            KeyCode::Down | KeyCode::Char('j') => self.list_select_next(list),
            KeyCode::Up | KeyCode::Char('k') => self.list_select_previous(list),
            _ => (),
        };

        HandleInputResult::None
    }
}

#[derive(Debug)]
pub enum HandleInputResult<T> {
    Selected(T),
    None,
}
