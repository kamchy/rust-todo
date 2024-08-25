use inquire::{Select, Text};
use std::{collections::HashMap, fmt::Display};
use uuid::Uuid;

/// TaskRepository allows storing and retrieving tasks
trait TaskRepository {
    /// adds a Task and returns its Uuid
    fn add_task(&mut self, t: Task) -> Uuid;
    /// retrieves a task from repository for given Uuid
    fn get_task(&self, id: Uuid) -> Option<Task>;
    /// gets the vec od Uuid references
    fn ids(&self) -> Vec<&Uuid>;
    fn get_all(&self) -> Vec<KeyedTask>;
    fn remove_task(&mut self, t: &KeyedTask);
}

#[derive(Debug)]
struct KeyedTask<'a>(&'a Uuid, &'a Task);
impl<'a> Display for KeyedTask<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} - {}", self.1, self.0)
    }
}
impl TaskRepository for HashMap<Uuid, Task> {
    fn add_task(&mut self, t: Task) -> Uuid {
        let uuid = Uuid::new_v4();
        self.insert(uuid, t);
        uuid
    }

    fn get_task(&self, id: Uuid) -> Option<Task> {
        let opt_ref = self.get(&id);
        opt_ref.cloned()
    }

    fn ids(&self) -> Vec<&Uuid> {
        let mut v = Vec::new();
        for k in self.keys() {
            v.push(k);
        }
        v
    }

    fn get_all(&self) -> Vec<KeyedTask> {
        let mut vv = Vec::new();
        for (uid, task) in self.iter() {
            vv.push(KeyedTask(uid, task));
        }
        vv
    }

    fn remove_task(&mut self, t: &KeyedTask) {
        self.remove(t.0);
    }
}

/// Models a priority of the task
#[derive(Debug, Clone)]
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

fn list_tasks(tr: &dyn TaskRepository) {
    for uid in tr.ids() {
        if let Some(t) = tr.get_task(*uid) {
            print!(" -> {:?} --- {}\n", t, uid);
        } else {
            print!("No tasks");
        };
    }
}

fn add_task(tr: &mut dyn TaskRepository) {
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
fn select_task(tr: &mut dyn TaskRepository) -> Option<KeyedTask<'_>> {
    let ids: Vec<KeyedTask> = tr.get_all();
    let selected = Select::new("Select one of tasks: ", ids).prompt();
    match selected {
        Ok(t) => Some(t),
        Err(_) => None,
    }
}

#[derive(Default)]
struct State<'a> {
    should_continue: bool,
    task: Option<KeyedTask<'a>>,
    action: Option<Action>,
}

fn execute_action<'a>(a: Action, tr: &'a mut dyn TaskRepository, state: State<'a>) -> State<'a> {
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
        Action::Quit => should_continue = false,
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

fn load_tasks(tr: &mut dyn TaskRepository) {
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

fn main() {
    let mut tr = HashMap::new();
    load_tasks(&mut tr);

    let mut curr_state = State::default();
    while curr_state.should_continue {
        let a = display_actions();
        curr_state = execute_action(a, &mut tr, curr_state);
    }
}
