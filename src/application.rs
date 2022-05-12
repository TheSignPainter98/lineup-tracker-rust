use crate::model::{Ability, Map, ProgressStore, Usage, Zone};
use crate::render::Renderable;
use crate::selection::{Selection, Selector};
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use serde::{Deserialize as Deserialise, Serialize as Serialise};
use std::io::{self, Error, ErrorKind, Read, Write};
use tui::{
    backend::Backend,
    layout::{Constraint, Layout},
    widgets::{Block, Borders, Paragraph, TableState},
    Frame, Terminal,
};
use unicode_width::UnicodeWidthStr;

enum InputSubject {
    MapName,
    ZoneName,
    AbilityName,
    UsageName,
}

enum InputOp {
    New,
    Select,
    Remove,
}

type InputType = (InputOp, InputSubject);

enum InputState {
    Normal,
    Edit(InputType, String),
}

impl InputState {
    fn edit(i: InputOp, s: InputSubject) -> Self {
        InputState::Edit((i, s), "".to_string())
    }
}

pub enum FinalAction {
    None,
    Save,
}

pub struct App {
    state: TableState,
    pub progress: ProgressStore,
    input_state: InputState,
    selection: Selection,
}

#[derive(Serialise, Deserialise)]
struct SaveState {
    progress: ProgressStore,
    selection: Selection,
}

impl App {
    pub fn new() -> App {
        App {
            state: TableState::default(),
            progress: ProgressStore::new("Progress".into()),
            input_state: InputState::Normal,
            selection: Selection::new(),
        }
    }

    pub fn load<R>(r: R) -> io::Result<Self>
    where
        R: Read,
    {
        let save_state: SaveState = serde_json::from_reader(r)?;
        Ok(save_state.into())
    }

    pub fn save<W>(self, w: W) -> Result<(), serde_json::Error>
    where
        W: Write,
    {
        let save_state: SaveState = self.into();
        serde_json::to_writer(w, &save_state)
    }

    pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> io::Result<FinalAction> {
        loop {
            terminal.draw(|f| self.draw(f))?;

            match event::read()? {
                Event::Key(key) => match self.input_state {
                    InputState::Normal => match key.code {
                        KeyCode::Char('Q') => return Ok(FinalAction::Save),
                        KeyCode::Char('!') => return Ok(FinalAction::None),
                        KeyCode::Char('y') => {
                            self.progress
                                .get_target_mut(&self.selection)
                                .map(|t| t.change_progress(1));
                        }
                        KeyCode::Char('Y') => {
                            self.progress
                                .get_target_mut(&self.selection)
                                .map(|t| t.change_progress(-1));
                        }
                        KeyCode::Char('u') => {
                            self.progress
                                .get_target_mut(&self.selection)
                                .map(|t| t.change_target(1));
                        }
                        KeyCode::Char('U') => {
                            self.progress
                                .get_target_mut(&self.selection)
                                .map(|t| t.change_target(-1));
                        }
                        KeyCode::Char('I') => {
                            self.progress
                                .get_target_mut(&self.selection)
                                .map(|t| t.match_target_to_progress());
                        }
                        KeyCode::Char('i') => {
                            self.progress
                                .get_target_mut(&self.selection)
                                .map(|t| t.match_progress_to_target());
                        }
                        KeyCode::Char('o') => {
                            self.progress
                                .get_target_mut(&self.selection)
                                .map(|t| t.zero_target());
                        }
                        KeyCode::Char('O') => {
                            self.progress
                                .get_target_mut(&self.selection)
                                .map(|t| t.zero_progress());
                        }
                        KeyCode::Char('q') => {
                            self.input_state = InputState::edit(InputOp::New, InputSubject::MapName)
                        }
                        KeyCode::Char('w') => {
                            self.input_state =
                                InputState::edit(InputOp::New, InputSubject::ZoneName)
                        }
                        KeyCode::Char('e') => {
                            self.input_state =
                                InputState::edit(InputOp::New, InputSubject::AbilityName)
                        }
                        KeyCode::Char('r') => {
                            self.input_state =
                                InputState::edit(InputOp::New, InputSubject::UsageName)
                        }
                        KeyCode::Char('a') => {
                            self.input_state =
                                InputState::edit(InputOp::Select, InputSubject::MapName)
                        }
                        KeyCode::Char('s') => {
                            self.input_state =
                                InputState::edit(InputOp::Select, InputSubject::ZoneName)
                        }
                        KeyCode::Char('d') => {
                            self.input_state =
                                InputState::edit(InputOp::Select, InputSubject::AbilityName)
                        }
                        KeyCode::Char('f') => {
                            self.input_state =
                                InputState::edit(InputOp::Select, InputSubject::UsageName)
                        }
                        KeyCode::Char('z') => {
                            self.input_state =
                                InputState::edit(InputOp::Remove, InputSubject::MapName)
                        }
                        KeyCode::Char('x') => {
                            self.input_state =
                                InputState::edit(InputOp::Remove, InputSubject::ZoneName)
                        }
                        KeyCode::Char('c') => {
                            if key.modifiers.contains(KeyModifiers::CONTROL) {
                                return Err(Error::new(ErrorKind::Other, "SIGINT Caught!"));
                            }

                            self.input_state =
                                InputState::edit(InputOp::Remove, InputSubject::AbilityName)

                        }
                        KeyCode::Char('v') => {
                            self.input_state =
                                InputState::edit(InputOp::Remove, InputSubject::UsageName)
                        }
                        KeyCode::Left | KeyCode::Char('h') => {
                            self.selection.prev_usage(&self.progress.abilities)
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            self.selection.next_zone(&self.progress.maps)
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            self.selection.prev_zone(&self.progress.maps)
                        }
                        KeyCode::Right | KeyCode::Char('l') => {
                            self.selection.next_usage(&self.progress.abilities)
                        }
                        _ => {}
                    },
                    InputState::Edit(ref op, ref mut buf) => match key.code {
                        KeyCode::Char(c) => buf.push(c),
                        KeyCode::Backspace => {
                            buf.pop();
                        }
                        KeyCode::Enter => {
                            match op {
                                (InputOp::New, InputSubject::MapName) => {
                                    self.progress.add_map(Map::new(buf.clone()));
                                    self.selection.map = Some(Selector::Name(buf.clone()));
                                    self.selection.zone = None;
                                }
                                (InputOp::New, InputSubject::ZoneName) => {
                                    if let Some(mi) = &self.selection.map {
                                        let zone_selector = Selector::Name(buf.clone());
                                        let zone = Zone::new(buf.clone());

                                        if let Some(map) =
                                            mi.get_selected_mut::<Map>(&mut self.progress.maps)
                                        {
                                            map.add_zone(zone);
                                        }

                                        self.progress.add_zone(&mi, &zone_selector);
                                        self.selection.zone = Some(zone_selector);
                                    }
                                }
                                (InputOp::New, InputSubject::AbilityName) => {
                                    self.progress.add_ability(Ability::new(buf.clone()));
                                    self.selection.ability = Some(Selector::Name(buf.clone()));
                                    self.selection.usage = None;
                                }
                                (InputOp::New, InputSubject::UsageName) => {
                                    if let Some(asel) = &self.selection.ability {
                                        let usage_selector = Selector::Name(buf.clone());
                                        let usage = Usage::new(buf.clone());

                                        if let Some(ability) =
                                            asel.get_selected_mut(&mut self.progress.abilities)
                                        {
                                            ability.add_usage(usage);
                                        }

                                        self.progress.add_usage(&asel, &usage_selector);
                                        self.selection.usage = Some(usage_selector);
                                    }
                                }
                                (InputOp::Select, subject) => {
                                    match subject {
                                        InputSubject::MapName => {
                                            self.selection.map = Some(buf.clone().into())
                                        }
                                        InputSubject::ZoneName => {
                                            self.selection.zone = Some(buf.clone().into())
                                        }
                                        InputSubject::AbilityName => {
                                            self.selection.ability = Some(buf.clone().into())
                                        }
                                        InputSubject::UsageName => {
                                            self.selection.usage = Some(buf.clone().into())
                                        }
                                    };
                                    self.selection = self
                                        .selection
                                        .relative(&self.progress.maps, &self.progress.abilities);
                                }
                                (InputOp::Remove, _) => {} // TODO: implement removal
                            };
                            self.input_state = InputState::Normal;
                        }
                        KeyCode::Esc => self.input_state = InputState::Normal,
                        _ => {}
                    },
                },
                _ => {}
            }
        }
    }

    pub fn draw<B: Backend>(&mut self, f: &mut Frame<B>) {
        let rect_constraints;
        if let InputState::Edit(_, _) = self.input_state {
            rect_constraints = [Constraint::Percentage(95), Constraint::Min(3)].as_ref();
        } else {
            rect_constraints = [Constraint::Percentage(100)].as_ref();
        }
        let rects = Layout::default()
            .margin(1)
            .constraints(rect_constraints)
            .split(f.size());

        let (ncols, mut table) = self.progress.render(&self.selection);
        let widths;
        if ncols != 0 {
            widths = [Constraint::Percentage(100 / ncols as u16)].repeat(ncols);
            table = table.widths(&widths);
        }
        f.render_stateful_widget(table, rects[0], &mut self.state);

        if let InputState::Edit(t, s) = &self.input_state {
            let mut box_name = match t {
                (InputOp::New, _) => "New ",
                (InputOp::Select, _) => "Select ",
                (InputOp::Remove, _) => "Remove ",
            }
            .to_string();
            box_name.push_str(match t {
                (_, InputSubject::MapName) => "Map",
                (_, InputSubject::ZoneName) => "Zone",
                (_, InputSubject::AbilityName) => "Ability",
                (_, InputSubject::UsageName) => "Usage",
            });
            let input_box = Paragraph::new(s.as_ref())
                .block(Block::default().borders(Borders::ALL).title(box_name));
            f.render_widget(input_box, rects[1]);
            f.set_cursor(rects[1].x + s.width() as u16 + 1, rects[1].y + 1)
        }
    }
}

impl From<SaveState> for App {
    fn from(s: SaveState) -> Self {
        App {
            state: TableState::default(),
            progress: s.progress,
            input_state: InputState::Normal,
            selection: s.selection,
        }
    }
}

impl Into<SaveState> for App {
    fn into(self) -> SaveState {
        SaveState {
            progress: self.progress,
            selection: self.selection,
        }
    }
}
