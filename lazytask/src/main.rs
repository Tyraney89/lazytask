use std::fs;
use clap::{Parser, Subcommand};
use serde::{Serialize, Deserialize};

#[derive(Parser)]
#[command(name = "lazytask")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}


#[derive(Subcommand)]
enum Commands {
    Add {
        message: String,
    },
    Move {
        id: u32,
        state: String,
    },

    List,
}

#[derive(Serialize, Deserialize, Debug)]
enum TaskState {
    Todo,
    InProgress,
    Done,
}


#[derive(Serialize, Deserialize)]
struct Task {
    id: u32,
    title: String,
    state: TaskState
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file_path = "tasks.json";
    let cli = Cli::parse();


    //load tasks from file
    let mut tasks: Vec<Task> = match fs::read_to_string(file_path) {
        Ok(content) => serde_json::from_str(&content)?,
        Err(_) => Vec::new(), // file doesn't exist yet
    };



    match cli.command {
        Commands::Add { message } => {
            println!("Adding task: {}", message);

            let next_id = tasks.iter().map(|t| t.id).max().unwrap_or(0) + 1;

            //COME BACK TO THIS
            let new_task = Task {
                id: next_id,
                title: message,
                state: TaskState::Todo,
            };

            tasks.push(new_task);

            let json = serde_json::to_string_pretty(&tasks)?;
            fs::write(file_path, json)?;

            println!("task saved");

        }
        Commands::Move { id, state } => {
            println!("Moving task {} to {}", id, state);

            let mut found = false;
            if let Some(task) = tasks.iter_mut().find(|t| t.id == id) {
                task.state = match state.to_lowercase().as_str() {
                    "todo" => TaskState::Todo,
                    "inprogress" | "in_progress" => TaskState::InProgress,
                    "done" => TaskState::Done,
                    _ => {
                        println!("Invalid state: {}. Use todo, in_progress, or done.", state);
                        return Ok(());
                    }
                };

                println!("Task {} moved to {:?}", id, task.state);
                found = true; // mark that we found the task
            }

            if !found {
                println!("Task with id {} not found.", id);
                return Ok(()); // nothing to save
            }

            let json = serde_json::to_string_pretty(&tasks)?;
            fs::write(file_path, json)?;
            println!("Tasks saved!"); 
            }

        Commands::List => {
            println!("Listing tasks");
            for task in &tasks {
                println!("id: {} ", task.id);
                println!("task: {}", task.title);
            }
        }
    }
    Ok(())
}
