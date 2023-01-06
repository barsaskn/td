use std::{error::Error, io::Stdout, path::PathBuf, time::SystemTime};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use td_lib::{
    database::{Database, DatabaseInfo, Task},
    errors::DatabaseReadError,
};
use tui::{
    backend::CrosstermBackend,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState},
    Frame, Terminal,
};

use self::{modal::text_input::TextInputModal, tab_layout::TabLayout};

mod modal;
mod tab_layout;

pub struct AppState {
    pub database: Database,
    pub path: PathBuf,
}
impl AppState {
    pub fn create(path: PathBuf) -> Result<Self, DatabaseReadError> {
        let db_info = if !path.exists() {
            println!("The given database file ({path:?}) does not exist, creating a new one.");

            let db_info = DatabaseInfo::default();
            db_info.write(&path)?;
            db_info
        } else {
            DatabaseInfo::read(&path)?
        };

        let database = db_info.try_into()?;

        Ok(Self { database, path })
    }

    pub fn run_loop(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    ) -> Result<(), Box<dyn Error>> {
        let mut root_component = LayoutRoot::new();

        loop {
            terminal.draw(|f| root_component.render(f, f.size(), self))?;

            if let Event::Key(key) = event::read()? {
                let handled = root_component.update(key, self);
                if !handled {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            break
                        }
                        KeyCode::Char('s') => {
                            // todo: save
                        }
                        _ => (),
                    }
                }
            }
        }

        Ok(())
    }
}

pub trait Component {
    /// Render the component and its children to the given area.
    fn render(&self, frame: &mut Frame<CrosstermBackend<Stdout>>, area: Rect, state: &AppState);

    /// Update state based in a key event. Returns whether the key event is handled by this
    /// component or one of its children.
    fn update(&mut self, key: KeyEvent, state: &mut AppState) -> bool;

    // TODO: may need to split update into input+update
}

struct LayoutRoot {
    tabs: TabLayout,
}

impl LayoutRoot {
    fn new() -> Self {
        Self {
            tabs: TabLayout::new([
                (
                    "Tasks",
                    Box::new(BasicTaskList::new(false)) as Box<dyn Component>,
                ),
                (
                    "Tasks (rev)",
                    Box::new(BasicTaskList::new(true)) as Box<dyn Component>,
                ),
            ]),
        }
    }
}

impl Component for LayoutRoot {
    fn render(&self, frame: &mut Frame<CrosstermBackend<Stdout>>, area: Rect, state: &AppState) {
        self.tabs.render(frame, area, state);
    }

    fn update(&mut self, key: KeyEvent, state: &mut AppState) -> bool {
        self.tabs.update(key, state)
    }
}

struct BasicTaskList {
    index: usize,
    task_popup: TextInputModal,
    reverse: bool,
}

impl BasicTaskList {
    fn new(reverse: bool) -> Self {
        Self {
            index: 0,
            task_popup: TextInputModal::new("Enter new task".to_string()),
            reverse,
        }
    }
}

impl Component for BasicTaskList {
    fn render(&self, frame: &mut Frame<CrosstermBackend<Stdout>>, area: Rect, state: &AppState) {
        let mut tasks = state.database.tasks.node_weights().collect::<Vec<_>>();

        tasks.sort_by(|a, b| a.time_created.cmp(&b.time_created));
        if self.reverse {
            tasks.reverse();
        }

        // render the list
        let block = Block::default()
            .title(if !self.reverse {
                "Basic Task List"
            } else {
                "Basic Task List (reversed)"
            })
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::White))
            .border_type(BorderType::Rounded)
            .style(Style::default().bg(Color::Black));

        let list_items = tasks
            .iter()
            .map(|t| ListItem::new(t.title.clone()))
            .collect::<Vec<_>>();
        let list = List::new(list_items)
            .block(block)
            .highlight_style(
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            )
            .style(Style::default().fg(Color::DarkGray));
        let mut list_state = ListState::default();
        list_state.select(if tasks.is_empty() {
            None
        } else {
            Some(self.index)
        });
        frame.render_stateful_widget(list, area, &mut list_state);

        // if needed, render the popup
        self.task_popup.render(frame, area, state);
    }

    fn update(&mut self, key: KeyEvent, state: &mut AppState) -> bool {
        if self.task_popup.update(key, state) {
            return true;
        }

        let task_indices = state.database.tasks.node_indices().collect::<Vec<_>>();

        if !task_indices.is_empty() {
            self.index = self.index.clamp(0, task_indices.len() - 1);
        }

        if self.task_popup.is_open() {
            // popup is open
            match key.code {
                KeyCode::Enter => {
                    if let Some(text) = self.task_popup.close() {
                        let task = Task {
                            title: text,
                            time_created: SystemTime::now(),
                        };
                        state.database.tasks.add_node(task);

                        // TODO: error handling. show popup on failure to save?
                        let db_info: DatabaseInfo = (&state.database).into();
                        db_info.write(&state.path).unwrap();
                    }
                    true
                }
                _ => false,
            }
        } else {
            match key.code {
                KeyCode::Char('c') if key.modifiers.is_empty() => {
                    self.task_popup.open();
                    true
                }
                KeyCode::Char('d') if key.modifiers.is_empty() && !task_indices.is_empty() => {
                    state.database.tasks.remove_node(task_indices[self.index]);

                    // TODO: error handling. show popup on failure to save?
                    let db_info: DatabaseInfo = (&state.database).into();
                    db_info.write(&state.path).unwrap();

                    true
                }
                KeyCode::Up => {
                    if self.index != 0 {
                        self.index -= 1;
                    }
                    true
                }
                KeyCode::Down => {
                    if self.index != task_indices.len() - 1 {
                        self.index += 1;
                    }
                    true
                }
                _ => false,
            }
        }
    }
}
