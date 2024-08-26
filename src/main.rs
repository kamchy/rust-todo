use crossterm::cursor::MoveTo;
use crossterm::style::{Color, SetForegroundColor};
use crossterm::terminal::{Clear, ClearType};
use crossterm::{style::Stylize, ExecutableCommand};
use inquire::{Select, Text};
use std::borrow::BorrowMut;
use std::{
    cell::RefCell,
    collections::HashMap,
    fmt::Display,
    io::{self, stdout, Write},
};
use uuid::Uuid;

/// TaskRepository allows storing and retrieving tasks
trait TaskRepository {
    /// adds a Task and returns its Uuid
    fn add_task(&self, t: Task) -> Uuid;
    /// retrieves a task from repository for given Uuid
    fn get_task(&self, id: Uuid) -> Option<Task>;
    /// gets the vec od Uuid references
    fn ids(&self) -> Vec<Uuid>;
    fn get_all(&self) -> Vec<KeyedTask>;
    fn remove_task(&self, t: &KeyedTask);
}

#[derive(Debug)]
struct KeyedTask(Uuid, Task);
impl Display for KeyedTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} - {}", self.1, self.0)
    }
}
/// TaskRepository implementation.
struct MapTaskRepository {
    tm: RefCell<HashMap<Uuid, Task>>,
}
impl MapTaskRepository {
    fn new() -> Self {
        MapTaskRepository {
            tm: RefCell::new(HashMap::new()),
        }
    }
}
impl TaskRepository for MapTaskRepository {
    fn add_task(&self, t: Task) -> Uuid {
        let uuid = Uuid::new_v4();
        self.tm.borrow_mut().insert(uuid, t);
        uuid
    }

    fn get_task(&self, id: Uuid) -> Option<Task> {
        let binding = self.tm.borrow();
        let opt_ref = binding.get(&id);
        opt_ref.cloned()
    }

    fn ids(&self) -> Vec<Uuid> {
        let mut v = Vec::new();
        let binding = self.tm.borrow();
        for k in binding.keys() {
            v.push(k.to_owned());
        }
        v
    }

    fn get_all(&self) -> Vec<KeyedTask> {
        let binding = self.tm.borrow();
        let v: Vec<KeyedTask> = binding
            .iter()
            .map(move |(k, v)| KeyedTask(k.clone(), v.clone()))
            .collect::<Vec<KeyedTask>>();
        v
    }
    fn remove_task(&self, t: &KeyedTask) {
        self.tm.borrow_mut().remove(&t.0);
    }
}

/// Models a priority of the task
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Priority {
    High,
    Medium,
    Low,
}
impl Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl Priority {
    const VALUES: [Priority; 3] = [Priority::High, Priority::Medium, Priority::Low];
}

#[derive(Clone, Debug)]
struct Task {
    name: String,
    priority: Priority,
}
impl Display for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}]{}", self.priority, self.name)
    }
}
#[derive(Debug, Clone)]
enum Action {
    Quit,
    List,
    Add,
    Remove,
    Unknown(String),
}
impl Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let w = match self {
            Self::Quit => "Quit",
            Self::List => "List",
            Self::Add => "Add",
            Self::Remove => "Remove",
            Self::Unknown(_msg) => "unknown",
        };
        write!(f, "[{:?}]", w)
    }
}
impl Action {
    const VALUES: [Action; 4] = [Action::Quit, Action::List, Action::Add, Action::Remove];
}

fn display_actions() -> Action {
    let action = Select::new("Select action: ", Action::VALUES.to_vec()).prompt();
    match action {
        Ok(a) => a,
        Err(err) => Action::Unknown(err.to_string()),
    }
}

fn priority_to_color(p: &Priority) -> Color {
    match p {
        Priority::Low => Color::Green,
        Priority::High => Color::Red,
        Priority::Medium => Color::Yellow,
    }
}
fn print_task(t: &Task) {
    let _ = stdout().execute(SetForegroundColor(priority_to_color(&t.priority)));
    print!(
        "[{:>10}] {}\n",
        t.priority,
        t.name.to_string().with(Color::Magenta)
    );
}

fn list_tasks(tr: &dyn TaskRepository) {
    let _ = clear();
    let mut all = tr.get_all();
    all.sort_by_key(|kt| kt.1.priority.clone());
    for t in all.iter() {
        print_task(&t.1);
    }
    if all.is_empty() {
        println!("{}", "No tasks".green());
    }
}

fn add_task(tr: &dyn TaskRepository) {
    let t = Text::new("Task: ").prompt();
    match t {
        Ok(task) => {
            let p = Select::new("Priority: ", Priority::VALUES.to_vec()).prompt();
            match p {
                Ok(prio) => {
                    let _ = tr.add_task(Task {
                        name: task,
                        priority: prio,
                    });
                }
                Err(_) => println!("error reading prompt"),
            }
        }
        Err(_) => println!("Error reading task"),
    }
}
fn select_task(tr: &dyn TaskRepository) -> Option<KeyedTask> {
    let ids: Vec<KeyedTask> = tr.get_all();
    let selected = Select::new("Select one of tasks: ", ids).prompt();
    match selected {
        Ok(t) => Some(t),
        Err(_) => None,
    }
}

#[derive(Default)]
struct State {
    should_continue: bool,
    task: Option<KeyedTask>,
    action: Option<Action>,
}

fn execute_action(a: Action, tr: &dyn TaskRepository, state: State) -> State {
    let mut should_continue = state.should_continue;
    let mut action_opt = state.action;
    let mut task_opt = state.task;
    if let Some(ref action) = action_opt {
        if let Some(ref t) = task_opt {
            match action {
                Action::Remove => {
                    tr.remove_task(&t);
                    action_opt = None;
                    task_opt = None;
                }
                _ => {}
            }
        }
    }
    match a {
        Action::Quit => {
            let _ = clear();
            should_continue = false
        }
        Action::List => list_tasks(tr),
        Action::Add => add_task(tr),
        Action::Remove => {
            action_opt = Some(a);
            task_opt = select_task(tr);
        }
        Action::Unknown(s) => {
            println!("Action undefined: {}", s);
        }
    };
    State {
        should_continue,
        task: task_opt,
        action: action_opt,
    }
}

fn load_tasks(tr: &dyn TaskRepository) {
    let t = Task {
        name: "Learn Rust".to_string(),
        priority: Priority::High,
    };
    let o = Task {
        name: "Learn NeoVim".to_string(),
        priority: Priority::Medium,
    };
    tr.add_task(t);
    tr.add_task(o);
}

fn clear() -> io::Result<()> {
    stdout()
        .execute(Clear(ClearType::All))?
        .execute(MoveTo(0, 0))?;
    Ok(())
}
fn main() -> io::Result<()> {
    let tr = MapTaskRepository::new();
    load_tasks(&tr);
    let mut curr_state = State {
        should_continue: true,
        task: None,
        action: None,
    };

    clear()?;
    while curr_state.should_continue {
        let a = display_actions();
        curr_state = execute_action(a, &tr, curr_state);
    }
    Ok(())
}
