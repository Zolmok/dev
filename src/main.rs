use std::env;
use std::path::Path;

extern crate scuttle;

fn main() {
    let cwd = env::current_dir().unwrap();
    let current_dir = Path::new(&cwd);
    let base_name = current_dir.file_name().unwrap();

    // check if session exists
    let session = format!("-t={}", base_name.to_str().unwrap());
    let tmux_has_session = scuttle::App {
        command: String::from("tmux"),
        args: vec!["has-session", session.as_str()],
    };

    // attach to a session
    let tmux_attach = scuttle::App {
        command: String::from("tmux"),
        args: vec!["attach", "-t", base_name.to_str().unwrap()],
    };

    // start a new session
    let tmux_new_session = scuttle::App {
        command: String::from("tmux"),
        args: vec![
            "new-session",
            "-s",
            base_name.to_str().unwrap(),
            "-n",
            "Server",
            "-d",
        ],
    };

    // create new windows
    let base_1 = format!("{}:1", base_name.to_str().unwrap());
    let tmux_new_window_ui = scuttle::App {
        command: String::from("tmux"),
        args: vec!["new-window", "-t", base_1.as_str(), "-n", "UI"],
    };
    let base_2 = format!("{}:2", base_name.to_str().unwrap());
    let tmux_new_window_tests = scuttle::App {
        command: String::from("tmux"),
        args: vec!["new-window", "-t", base_2.as_str(), "-n", "Tests"],
    };
    let base_3 = format!("{}:3", base_name.to_str().unwrap());
    let tmux_new_window_code = scuttle::App {
        command: String::from("tmux"),
        args: vec!["new-window", "-t", base_3.as_str(), "-n", "Code"],
    };
    let base_4 = format!("{}:4", base_name.to_str().unwrap());
    let tmux_new_window_zsh = scuttle::App {
        command: String::from("tmux"),
        args: vec!["new-window", "-t", base_4.as_str(), "-n", "Zsh"],
    };

    let base_0_0 = format!("{}:0.0", base_name.to_str().unwrap());
    let tmux_select_window = scuttle::App {
        command: String::from("tmux"),
        args: vec!["select-window", "-t", base_0_0.as_str()],
    };

    match scuttle::run_app(&tmux_has_session) {
        Err(error) => panic!("{}", error),
        Ok(status) => match status.code() {
            Some(0) => {
                scuttle::run_app(&tmux_attach).unwrap();
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

                scuttle::run_apps(apps);
            }
            Some(code) => println!("Unknown exit status: {}", code),
            None => (),
        },
    }
}
