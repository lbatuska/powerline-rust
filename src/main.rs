use std::{
    env::{self, args},
    fs,
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};

use git2::{Repository, Status, StatusOptions};
use tokio::{process::Command, task::JoinHandle};
use unicode_segmentation::UnicodeSegmentation;

const TERM_GREEN: &str = r#"\[\033[32m\]"#;
const TERM_RED: &str = r#"\[\033[31m\]"#;
const TERM_ORANGE: &str = r#"\[\e[38;5;208m\]"#;
const TERM_RESET: &str = r#"\[\033[0m\]"#;

async fn rust_version() -> String {
    match Command::new("rustc").output().await {
        Ok(o) => String::from_utf8_lossy(&o.stdout).to_string(),
        Err(_) => "".into(),
    }
}

#[tokio::main]
async fn main() {
    let do_git = env::var("SKIP_GIT_STATUS").is_err();

    let pwd = Arc::new(env::var("PWD").unwrap_or("".to_owned()));

    let pwd_clone = Arc::clone(&pwd);
    let git_stat_handle: Option<JoinHandle<String>> = if do_git {
        Some(tokio::spawn(async {
            // Start timing the git_stats execution
            let start_time = Instant::now();
            let result = git_stats(pwd_clone).await;

            let elapsed_time = start_time.elapsed();

            // If it took longer than 1 second, set the environment variable
            if elapsed_time > Duration::from_millis(500) {
                println!(r#"export SKIP_GIT_STATUS="asd";"#)
            }

            result
        }))
    } else {
        None
    };

    let hostname_results = get_hostname();
    let usr_prompt_results = get_usr_and_prompt();
    let path_res_handle = tokio::spawn(get_rel_or_abs_path(pwd.clone()));

    let mut line_buffer: String = String::with_capacity(300);

    let last_return = args()
        .nth(1)
        .unwrap_or("1".to_owned())
        .parse::<i32>()
        .unwrap();

    if last_return == 0 {
        // line_buffer.push_str(" ✅");
        line_buffer.push_str(TERM_GREEN);
    } else {
        // line_buffer.push_str(" ❗");
        line_buffer.push_str(&format!("{}({})", TERM_RED, last_return));
    }
    line_buffer.push('[');
    line_buffer.push_str(&hostname_results.await);

    for grapheme in &UnicodeSegmentation::graphemes("🦀", true).collect::<Vec<&str>>() {
        line_buffer.push_str(grapheme);
    }

    let (usr, usr_prompt) = usr_prompt_results.await;

    line_buffer.push_str(&usr);

    line_buffer.push(']');
    line_buffer.push_str(TERM_RESET);
    line_buffer.push('(');

    line_buffer.push_str(&path_res_handle.await.unwrap());

    line_buffer.push(')');
    if let Some(handle) = git_stat_handle {
        line_buffer.push_str(&handle.await.unwrap());
    }
    line_buffer.push(usr_prompt);

    println!(r#"export PS1="{}""#, line_buffer);
    return;
}

async fn get_rel_or_abs_path(path: Arc<String>) -> String {
    match path.strip_prefix(dirs::home_dir().unwrap().to_str().unwrap()) {
        Some(stripped) => format!("{}{}", '~', stripped),
        None => Arc::as_ref(&path).clone(),
    }
}

async fn get_usr_and_prompt() -> (String, char) {
    let usr = &env::var("USER").unwrap_or('?'.into());
    let usr_prompt = if usr == "root" { '#' } else { '$' };
    (usr.to_owned(), usr_prompt)
}

async fn get_hostname() -> String {
    let mut line_buffer = String::with_capacity(100);

    match env::var("HOSTNAME") {
        Ok(hname) => line_buffer.push_str(&hname),
        Err(_) => {
            line_buffer.push_str(
                fs::read_to_string("/etc/hostname")
                    .unwrap()
                    .lines()
                    .next()
                    .unwrap(),
            );
        }
    }
    line_buffer
}

async fn git_stats(path: Arc<String>) -> String {
    tokio::task::spawn_blocking(move || {
        let mut line_buffer = String::with_capacity(100);
        if let Ok(repo) = Repository::discover(Path::new(Arc::as_ref(&path))) {
            line_buffer.push('|');
            match repo.head() {
                Ok(head) => {
                    let local_branch = head.shorthand().unwrap();

                    let mut stops = StatusOptions::new();
                    stops.include_untracked(true);
                    let statuses = repo.statuses(Some(&mut stops)).unwrap();

                    if !statuses.is_empty() {
                        line_buffer.push_str(TERM_ORANGE);
                    } else {
                        line_buffer.push_str(TERM_GREEN);
                    }
                    line_buffer.push_str(local_branch);

                    if statuses
                        .iter()
                        .any(|s| s.status().contains(Status::CONFLICTED))
                    {
                        line_buffer.push('❌');
                    }
                    if statuses
                        .iter()
                        .any(|s| s.status().contains(Status::WT_MODIFIED))
                    // Not Staged
                    {
                        line_buffer.push_str("✏️");
                    }
                    if statuses
                        .iter()
                        .any(|s| s.status().contains(Status::INDEX_MODIFIED))
                    // Staged
                    {
                        line_buffer.push('🚧');
                    }
                    if statuses.iter().any(|s| s.status().contains(Status::WT_NEW))
                    // Untracked
                    {
                        line_buffer.push('❓');
                    }
                }
                Err(_) => {
                    line_buffer.push_str(TERM_RED);

                    line_buffer.push_str("NO BRANCH");
                }
            }
            line_buffer.push_str(TERM_RESET);
            line_buffer.push('|');
        }
        line_buffer
    })
    .await
    .unwrap()
}
