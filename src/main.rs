use serde::Deserialize;

use std::env;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use scuttle::{App, Args};

extern crate scuttle;

#[derive(Deserialize, Debug)]
struct Window {
    name: String,
    actions: Vec<String>,
    pwd: String,
    select: bool,
}

#[derive(Deserialize, Debug)]
struct Config {
    session: String,
    windows: Vec<Window>,
}

/// Run a list of apps and print out the command and it's arguments before running
///
/// # Arguments
///
/// * `apps` - A vector of apps to run
fn run_apps(apps: &[App]) {
    for app in apps.iter() {
        println!("");
        println!("========================");
        println!("$ {} {}", app.command, Args(app.args.to_owned()));
        println!("========================");

        match scuttle::run_status(app) {
            Err(error) => panic!("panic{}", error),
            Ok(_status) => continue,
        };
    }
}

fn run_default() {
    let cwd = env::current_dir().unwrap();
    let current_dir = Path::new(&cwd);
    let base_name = current_dir.file_name().unwrap();

    // check if session exists
    let session = format!("-t={}", base_name.to_str().unwrap());
    let tmux_has_session = scuttle::App {
        command: String::from("tmux"),
        args: vec!["has-session".to_string(), session.to_string()],
    };

    // attach to a session
    let tmux_attach = scuttle::App {
        command: String::from("tmux"),
        args: vec![
            "attach".to_string(),
            "-t".to_string(),
            base_name.to_str().unwrap().to_string(),
        ],
    };

    // start a new session
    let tmux_new_session = scuttle::App {
        command: String::from("tmux"),
        args: vec![
            "new-session".to_string(),
            "-s".to_string(),
            base_name.to_str().unwrap().to_string(),
            "-n".to_string(),
            "Server".to_string(),
            "-d".to_string(),
        ],
    };

    // create new windows
    let base_1 = format!("{}:1", base_name.to_str().unwrap());
    let tmux_new_window_ui = scuttle::App {
        command: String::from("tmux"),
        args: vec![
            "new-window".to_string(),
            "-t".to_string(),
            base_1.to_string(),
            "-n".to_string(),
            "UI".to_string(),
        ],
    };
    let base_2 = format!("{}:2", base_name.to_str().unwrap());
    let tmux_new_window_tests = scuttle::App {
        command: String::from("tmux"),
        args: vec![
            "new-window".to_string(),
            "-t".to_string(),
            base_2.to_string(),
            "-n".to_string(),
            "Tests".to_string(),
        ],
    };
    let base_3 = format!("{}:3", base_name.to_str().unwrap());
    let tmux_new_window_code = scuttle::App {
        command: String::from("tmux"),
        args: vec![
            "new-window".to_string(),
            "-t".to_string(),
            base_3.to_string(),
            "-n".to_string(),
            "Code".to_string(),
        ],
    };
    let base_4 = format!("{}:4", base_name.to_str().unwrap());
    let tmux_new_window_zsh = scuttle::App {
        command: String::from("tmux"),
        args: vec![
            "new-window".to_string(),
            "-t".to_string(),
            base_4.to_string(),
            "-n".to_string(),
            "Zsh".to_string(),
        ],
    };

    let base_0_0 = format!("{}:0.0", base_name.to_str().unwrap());
    let tmux_select_window = scuttle::App {
        command: String::from("tmux"),
        args: vec![
            "select-window".to_string(),
            "-t".to_string(),
            base_0_0.to_string(),
        ],
    };

    match scuttle::run_status(&tmux_has_session) {
        Err(error) => panic!("{}", error),
        Ok(status) => match status.code() {
            Some(0) => {
                scuttle::run_status(&tmux_attach).unwrap();
            }
            Some(1) => {
                let apps: &[scuttle::App] = &[
                    tmux_new_session,
                    tmux_new_window_ui,
                    tmux_new_window_tests,
                    tmux_new_window_code,
                    tmux_new_window_zsh,
                    tmux_select_window,
                    tmux_attach,
                ];

                run_apps(apps);
            }
            Some(code) => println!("Unknown exit status: {}", code),
            None => (),
        },
    }
}

fn read_config_from_file<P: AsRef<Path>>(path: P) -> Result<Config, String> {
    // Open the file in read-only mode with buffer.
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) => return Err(error.to_string()),
    };
    let reader = BufReader::new(file);

    let config = match serde_json::from_reader(reader) {
        Ok(config) => config,
        Err(error) => panic!("Unable to parse json: {}", error),
    };

    Ok(config)
}

fn run_with_config(config: Config) {
    let base_name = config.session;

    // check if session exists
    let session = format!("-t={}", base_name.to_owned());
    let tmux_has_session = scuttle::App {
        command: String::from("tmux"),
        args: vec!["has-session".to_string(), session.to_string()],
    };

    // attach to a session
    let tmux_attach = scuttle::App {
        command: String::from("tmux"),
        args: vec!["attach".to_string(), "-t".to_string(), base_name.to_owned()],
    };

    // start a new session
    let tmux_new_session = scuttle::App {
        command: String::from("tmux"),
        args: vec![
            "new-session".to_string(),
            "-s".to_string(),
            base_name.to_owned(),
            "-n".to_string(),
            // use first windows name
            config.windows[0].name.to_owned(),
            "-d".to_string(),
        ],
    };

    let mut index = 0;
    let mut windows: Vec<scuttle::App> = vec![tmux_new_session]; // mutable
    let mut directories: Vec<scuttle::App> = vec![];
    let mut commands: Vec<scuttle::App> = vec![];
    let mut tmux_select: scuttle::App = scuttle::App {
        command: "".to_string(),
        args: vec![],
    };
    for window in config.windows.iter() {
        let base_window = format!("{}:{}", base_name.to_owned(), index);
        let base_split = format!("{}:{}.0", base_name.to_owned(), index);
        let tmux_new_window = scuttle::App {
            command: String::from("tmux"),
            args: vec![
                "new-window".to_string(),
                "-t".to_string(),
                base_window.to_string(),
                "-n".to_string(),
                window.name.to_owned(),
            ],
        };

        // build directory PWDs
        let pwd = format!("cd {}", &window.pwd);
        let tmux_pwd = scuttle::App {
            command: String::from("tmux"),
            args: vec![
                "send-keys".to_string(),
                "-t".to_string(),
                base_split.to_string(),
                pwd.to_string(),
                "C-m".to_string(),
            ],
        };

        directories.push(tmux_pwd);

        // build commands
        for action in window.actions.iter() {
            let tmux_send_keys = scuttle::App {
                command: String::from("tmux"),
                args: vec![
                    "send-keys".to_string(),
                    "-t".to_string(),
                    base_split.to_string(),
                    action.to_string(),
                    "C-m".to_string(),
                ],
            };

            commands.push(tmux_send_keys);
        }

        if window.select {
            tmux_select = scuttle::App {
                command: String::from("tmux"),
                args: vec![
                    "select-window".to_string(),
                    "-t".to_string(),
                    base_split.to_string(),
                ],
            }
        };

        index = index + 1;
        windows.push(tmux_new_window);
    }

    // has-session
    match scuttle::run_status(&tmux_has_session) {
        Err(error) => panic!("{}", error),
        Ok(status) => match status.code() {
            Some(0) => {
                // attach
                scuttle::run_status(&tmux_attach).unwrap();
            }
            Some(1) => {
                // new-window
                let apps: &[scuttle::App] = &windows;
                let pwds: &[scuttle::App] = &directories;
                let keys: &[scuttle::App] = &commands;

                // create window
                run_apps(apps);
                // changed directory to pwd
                run_apps(pwds);
                // run commands
                run_apps(keys);
                // select window
                if tmux_select.command != "" {
                    scuttle::run_status(&tmux_select).unwrap();
                }
                // attach
                scuttle::run_status(&tmux_attach).unwrap();
            }
            Some(code) => println!("Unknown exit status: {}", code),
            None => (),
        },
    }
}

fn main() {
    let config_file = "./.dev.json";

    match read_config_from_file(config_file) {
        Ok(config) => run_with_config(config),
        _ => run_default(),
    };
}
