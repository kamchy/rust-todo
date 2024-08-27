use crossterm::cursor::MoveTo;
use crossterm::style::{Color, SetForegroundColor};
use crossterm::terminal::{Clear, ClearType};
use crossterm::{style::Stylize, ExecutableCommand};
use inquire::list_option::ListOption;
use inquire::{Select, Text};
use serde::Deserialize;
use serde::Serialize;
use std::fs;
use std::{
    cell::RefCell,
    collections::HashMap,
    fmt::Display,
    io::{self, stdout},
};
use uuid::Uuid;

/// TaskRepository allows storing and retrieving tasks.
/// Tasks - after being added to [TaskRepository] are identified by [[Uuid]]
/// See [[KeyedTask]].
/// One implementation of this trait is [[MapTaskRepository]].
trait TaskRepository {
    /// adds a Task and returns its Uuid
    fn add_task(&self, t: Task) -> Uuid;
    /// retrieves a task from repository for given Uuid
    fn get_task(&self, id: Uuid) -> Option<Task>;
    /// gets the vec od Uuid references
    fn ids(&self) -> Vec<Uuid>;
    fn get_all(&self) -> Vec<KeyedTask>;
    /// remove task from repository; uses [KeyedTask] so that a task can be removed using its Uuid
    fn remove_task(&self, t: &KeyedTask);
}

/// (Uuid, Task) pair
#[derive(Debug)]
struct KeyedTask(Uuid, Task);
impl Display for KeyedTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} - {}", self.1, self.0)
    }
}
/// TaskRepository implementation struct that consists of and uses HashMap with [[Uuid]] as a key..
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
/// Implementation of TaskRepository for [[MapTaskRepository]]
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
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
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

/// A task struct has name and priority.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Task {
    name: String,
    priority: Priority,
}
impl Display for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}]{}", self.priority, self.name)
    }
}

/// Action represent user action in prompt-action loop.
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

/// Displays actions prompt and returns an action selected by user.
fn display_actions() -> Action {
    let action = Select::new("Select action: ", Action::VALUES.to_vec()).prompt();
    match action {
        Ok(a) => a,
        Err(err) => Action::Unknown(err.to_string()),
    }
}

/// Maps priority to a color used in terminal
fn priority_to_color(p: &Priority) -> Color {
    match p {
        Priority::Low => Color::Green,
        Priority::High => Color::Red,
        Priority::Medium => Color::Yellow,
    }
}

/// Returns String representation of the task with color-coded priority
fn format_task(t: &Task) -> String {
    format!(
        "[{:>10}] {}\n",
        t.priority,
        t.name.to_string().with(Color::Magenta)
    )
}

/// Prints a task to stdout
fn print_task(t: &Task) {
    let _ = stdout().execute(SetForegroundColor(priority_to_color(&t.priority)));
    print!("{}", format_task(t));
}

/// Lists all tasks in repository
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

/// Prompts for a task and its priority and adds it to repository.
/// **NOTE***: [todo] task creation and task adding should be separated.
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

/// Formats a ListOption of KeyedTask (to be used in [[select_task]].
fn task_selection_formatter(lo: ListOption<&KeyedTask>) -> String {
    format_task(&lo.value.1)
}

/// Promppts for a task.
fn select_task(tr: &dyn TaskRepository) -> Option<KeyedTask> {
    let task_repr: Vec<KeyedTask> = tr.get_all();
    let selected = Select::new("Select one of tasks: ", task_repr)
        .with_formatter(&task_selection_formatter)
        .prompt();

    selected.ok()
}

/// Keeps state between loop executions (currently, defers the removal action of a selected task to next itetation of the loop. Can be useful also to edit a selected task (not implemented yet).
#[derive(Default)]
struct State {
    should_continue: bool,
    task: Option<KeyedTask>,
    action: Option<Action>,
}

/// Executes provided action (unless state contains deferred action which has higher  priority).
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
/// Default path to read from and store tasks
const PATH: &str = "tasks.json";

/// loads tasks to repository from [[PATH]]
fn load_tasks(tr: &dyn TaskRepository) -> io::Result<()> {
    if let Ok(contents) = fs::read_to_string(PATH) {
        let tasks: Vec<Task> = serde_json::from_str(&contents)?;
        for t in tasks {
            tr.add_task(t.clone());
        }
    };
    if tr.get_all().is_empty() {
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
    Ok(())
}

/// Saves tasks to a file denoted by [PATH]
fn save_tasks(tr: &dyn TaskRepository) -> io::Result<()> {
    let v: Vec<Task> = tr.get_all().iter().map(|kt| kt.1.clone()).collect();
    fs::write(PATH, serde_json::to_string(&v)?)
}

/// Clears stdout
fn clear() -> io::Result<()> {
    stdout()
        .execute(Clear(ClearType::All))?
        .execute(MoveTo(0, 0))?;
    Ok(())
}

fn main() -> io::Result<()> {
    let tr = MapTaskRepository::new();
    load_tasks(&tr)?;
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
    save_tasks(&tr)?;
    Ok(())
}
