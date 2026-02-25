use std::fs;
use std::io;
use std::time::Duration;

use clap::{Parser, Subcommand};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};
use serde::{Deserialize, Serialize};

const TASKS_FILE: &str = "tasks.json";

#[derive(Parser)]
#[command(name = "lazytask")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Add { message: String },
    Move { id: u32, state: String },
    List,
    /// Open the kanban board TUI (vim motions, Space to select/move tasks)
    Board,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
enum TaskState {
    Todo,
    InProgress,
    Done,
}

impl TaskState {
    fn from_column_index(i: usize) -> Self {
        match i {
            0 => TaskState::Todo,
            1 => TaskState::InProgress,
            _ => TaskState::Done,
        }
    }

    fn label(self) -> &'static str {
        match self {
            TaskState::Todo => "To do",
            TaskState::InProgress => "In progress",
            TaskState::Done => "Done",
        }
    }

    fn next(self) -> Option<Self> {
        match self {
            TaskState::Todo => Some(TaskState::InProgress),
            TaskState::InProgress => Some(TaskState::Done),
            TaskState::Done => None,
        }
    }

    fn prev(self) -> Option<Self> {
        match self {
            TaskState::Todo => None,
            TaskState::InProgress => Some(TaskState::Todo),
            TaskState::Done => Some(TaskState::InProgress),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct Task {
    id: u32,
    title: String,
    state: TaskState,
}

fn load_tasks() -> Result<Vec<Task>, Box<dyn std::error::Error>> {
    match fs::read_to_string(TASKS_FILE) {
        Ok(content) => Ok(serde_json::from_str(&content)?),
        Err(_) => Ok(Vec::new()),
    }
}

fn save_tasks(tasks: &[Task]) -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string_pretty(tasks)?;
    fs::write(TASKS_FILE, json)?;
    Ok(())
}

fn tasks_by_state(tasks: &[Task], state: TaskState) -> Vec<&Task> {
    tasks.iter().filter(|t| t.state == state).collect()
}

/// TUI state for the kanban board
struct App {
    tasks: Vec<Task>,
    col: usize,        // 0 = Todo, 1 = InProgress, 2 = Done
    row: usize,        // index within current column
    selected_id: Option<u32>,
    message: String,
    /// When Some, we're in insert mode; the string is the new task title being typed.
    input_buffer: Option<String>,
}

impl App {
    fn new(tasks: Vec<Task>) -> Self {
        App {
            tasks,
            col: 0,
            row: 0,
            selected_id: None,
            message: String::new(),
            input_buffer: None,
        }
    }

    fn column_tasks(&self, col: usize) -> Vec<&Task> {
        let state = TaskState::from_column_index(col);
        tasks_by_state(&self.tasks, state)
    }

    fn current_column_len(&self) -> usize {
        self.column_tasks(self.col).len()
    }

    fn clamp_row(&mut self) {
        let len = self.current_column_len();
        if len == 0 {
            self.row = 0;
        } else if self.row >= len {
            self.row = len.saturating_sub(1);
        }
    }

    fn current_task(&self) -> Option<&Task> {
        let col_tasks = self.column_tasks(self.col);
        col_tasks.get(self.row).copied()
    }

    fn move_cursor_down(&mut self) {
        let len = self.current_column_len();
        if len == 0 {
            return;
        }
        self.row = (self.row + 1).min(len - 1);
    }

    fn move_cursor_up(&mut self) {
        self.row = self.row.saturating_sub(1);
    }

    fn move_cursor_left(&mut self) {
        if self.col > 0 {
            self.col -= 1;
            self.clamp_row();
        }
    }

    fn move_cursor_right(&mut self) {
        if self.col < 2 {
            self.col += 1;
            self.clamp_row();
        }
    }

    fn toggle_select(&mut self) {
        if self.selected_id.take().is_some() {
            self.message = "Deselected".into();
            return;
        }
        if let Some(task_id) = self.current_task().map(|t| t.id) {
            self.selected_id = Some(task_id);
            self.message = format!("Selected #{} - press h/l to move, Space to deselect", task_id);
        }
    }

    fn move_selected_task(&mut self, direction: i32) {
        let id = match self.selected_id {
            Some(id) => id,
            None => {
                self.message = "Select a task with Space first".into();
                return;
            }
        };

        let task = match self.tasks.iter_mut().find(|t| t.id == id) {
            Some(t) => t,
            None => return,
        };

        let new_state = if direction > 0 {
            task.state.next()
        } else {
            task.state.prev()
        };

        if let Some(s) = new_state {
            task.state = s;
            if save_tasks(&self.tasks).is_ok() {
                self.message = format!("Moved task #{} to {}", id, s.label());
            } else {
                self.message = "Failed to save".into();
            }
        } else {
            self.message = "Already at first/last column".into();
        }
    }

    fn handle_key(&mut self, key: KeyCode) -> bool {
        if let Some(ref mut buf) = self.input_buffer {
            match key {
                KeyCode::Enter => {
                    let title = buf.trim().to_string();
                    self.input_buffer = None;
                    if title.is_empty() {
                        self.message = "Cancelled".into();
                        return false;
                    }
                    let next_id = self.tasks.iter().map(|t| t.id).max().unwrap_or(0) + 1;
                    self.tasks.push(Task {
                        id: next_id,
                        title,
                        state: TaskState::Todo,
                    });
                    if save_tasks(&self.tasks).is_ok() {
                        self.message = "Task added".into();
                    } else {
                        self.message = "Failed to save".into();
                    }
                }
                KeyCode::Esc => {
                    self.input_buffer = None;
                    self.message = "Cancelled".into();
                }
                KeyCode::Backspace => {
                    buf.pop();
                }
                KeyCode::Char(c) => {
                    buf.push(c);
                }
                _ => {}
            }
            return false;
        }

        match key {
            KeyCode::Char('q') => return true,
            KeyCode::Char('i') => {
                self.input_buffer = Some(String::new());
                self.message = "New task (Enter to add, Esc to cancel)".into();
            }
            KeyCode::Char('j') | KeyCode::Down => self.move_cursor_down(),
            KeyCode::Char('k') | KeyCode::Up => self.move_cursor_up(),
            KeyCode::Char('h') | KeyCode::Left => {
                if self.selected_id.is_some() {
                    self.move_selected_task(-1);
                } else {
                    self.move_cursor_left();
                }
            }
            KeyCode::Char('l') | KeyCode::Right => {
                if self.selected_id.is_some() {
                    self.move_selected_task(1);
                } else {
                    self.move_cursor_right();
                }
            }
            KeyCode::Char(' ') => self.toggle_select(),
            _ => {}
        }
        false
    }
}

fn run_board() -> Result<(), Box<dyn std::error::Error>> {
    let tasks = load_tasks()?;
    let mut app = App::new(tasks);

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    let mut quit = false;
    while !quit {
        terminal.draw(|f| ui(f, &app))?;
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    quit = app.handle_key(key.code);
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn ui(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(34),
            Constraint::Percentage(33),
        ])
        .split(frame.area());

    let states = [TaskState::Todo, TaskState::InProgress, TaskState::Done];
    for (i, &state) in states.iter().enumerate() {
        let tasks = app.column_tasks(i);
        let block = Block::default()
            .title(state.label())
            .borders(Borders::ALL)
            .border_style(if app.col == i {
                Style::default().fg(Color::Green)
            } else {
                Style::default()
            });

        let items: Vec<ListItem> = tasks
            .iter()
            .enumerate()
            .map(|(idx, task)| {
                let selected = app.selected_id == Some(task.id);
                let cursor = app.col == i && app.row == idx;
                let style = if cursor && selected {
                    Style::default()
                        .fg(Color::Yellow)
                        .bg(Color::Blue)
                        .add_modifier(Modifier::BOLD)
                } else if cursor {
                    Style::default().fg(Color::White).bg(Color::Blue)
                } else if selected {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                };
                let line = format!(" #{} {}", task.id, task.title);
                ListItem::new(line).style(style)
            })
            .collect();

        let list = List::new(items).block(block);
        frame.render_widget(list, chunks[i]);
    }

    let help_text = if app.input_buffer.is_some() {
        " Enter: add  Esc: cancel "
    } else {
        " j/k: move  h/l: column  Space: select  i: insert  q: quit "
    };
    let help = Paragraph::new(help_text)
    .style(Style::default().fg(Color::DarkGray))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    )
    .wrap(Wrap { trim: true });
    let area = Rect {
        x: 0,
        y: frame.area().height.saturating_sub(3),
        width: frame.area().width,
        height: 3,
    };
    frame.render_widget(help, area);

    if let Some(ref buf) = app.input_buffer {
        let prompt = format!(" New task: {buf}_ ");
        let input_line = Paragraph::new(prompt)
            .style(Style::default().fg(Color::Yellow))
            .block(
                Block::default()
                    .title(" Insert mode ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow)),
            );
        let input_area = Rect {
            x: 0,
            y: frame.area().height.saturating_sub(6),
            width: frame.area().width,
            height: 3,
        };
        frame.render_widget(input_line, input_area);
    } else if !app.message.is_empty() {
        let msg = Paragraph::new(app.message.as_str())
            .style(Style::default().fg(Color::Green))
            .block(Block::default().borders(Borders::ALL));
        let msg_area = Rect {
            x: 0,
            y: frame.area().height.saturating_sub(6),
            width: frame.area().width,
            height: 3,
        };
        frame.render_widget(msg, msg_area);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let command = cli.command.unwrap_or(Commands::Board);

    match command {
        Commands::Add { message } => {
            let mut tasks = load_tasks()?;
            let next_id = tasks.iter().map(|t| t.id).max().unwrap_or(0) + 1;
            tasks.push(Task {
                id: next_id,
                title: message,
                state: TaskState::Todo,
            });
            save_tasks(&tasks)?;
            println!("Task saved.");
        }
        Commands::Move { id, state } => {
            let mut tasks = load_tasks()?;
            let mut found = false;
            if let Some(task) = tasks.iter_mut().find(|t| t.id == id) {
                task.state = match state.to_lowercase().as_str() {
                    "todo" => TaskState::Todo,
                    "inprogress" | "in_progress" => TaskState::InProgress,
                    "done" => TaskState::Done,
                    _ => {
                        println!("Invalid state. Use todo, in_progress, or done.");
                        return Ok(());
                    }
                };
                found = true;
            }
            if found {
                save_tasks(&tasks)?;
                println!("Task moved.");
            } else {
                println!("Task with id {} not found.", id);
            }
        }
        Commands::List => {
            let tasks = load_tasks()?;
            for task in &tasks {
                println!("id: {}  task: {}", task.id, task.title);
            }
        }
        Commands::Board => {
            run_board()?;
        }
    }
    Ok(())
}
