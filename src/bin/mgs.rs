use anyhow::Result;
use clap::Parser;
use mgs::cli::{Cli, Command, KeyCommand, RepoCommand, UserCommand};
use mgs::cli::{init, repo, user};

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {:#}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let data_dir = cli.data_dir();

    match cli.command {
        Command::Init => init::run_init(&data_dir),
        Command::User { command } => match command {
            UserCommand::Add { username, key } => user::run_user_add(&data_dir, &username, &key),
            UserCommand::List => user::run_user_list(&data_dir),
            UserCommand::Remove { username } => user::run_user_remove(&data_dir, &username),
            UserCommand::Key { command } => match command {
                KeyCommand::Add { username, key } => user::run_key_add(&data_dir, &username, &key),
                KeyCommand::List { username } => user::run_key_list(&data_dir, &username),
                KeyCommand::Remove { fingerprint } => user::run_key_remove(&data_dir, &fingerprint),
            },
        },
        Command::Repo { command } => match command {
            RepoCommand::Create { name, owner } => {
                repo::run_repo_create(&data_dir, &name, owner.as_deref())
            }
            RepoCommand::List => repo::run_repo_list(&data_dir),
            RepoCommand::Remove { name } => repo::run_repo_remove(&data_dir, &name),
        },
    }
}
