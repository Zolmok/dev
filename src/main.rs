use serde::{Deserialize, Serialize};

use std::env;
use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

// ============================================================================
// Core Types
// ============================================================================

/// Wrapper for displaying command arguments
pub struct Args(pub Vec<String>);

impl fmt::Display for Args {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0.join(" "))
    }
}

/// Represents an application command with its arguments
#[derive(Debug, Clone, PartialEq)]
pub struct App {
    pub command: String,
    pub args: Vec<String>,
}

// ============================================================================
// Traits for Dependency Injection
// ============================================================================

/// Trait for running processes
pub trait ProcessRunner {
    fn run(&self, app: &App) -> Result<ExitStatus, std::io::Error>;
}

/// Trait for filesystem operations
pub trait FileSystem {
    fn exists(&self, path: &Path) -> bool;
    fn open(&self, path: &Path) -> Result<File, std::io::Error>;
    fn write(&self, path: &Path, contents: &str) -> Result<(), std::io::Error>;
}

/// Trait for environment operations
pub trait Environment {
    fn current_dir(&self) -> Result<PathBuf, std::io::Error>;
}

// ============================================================================
// Real Implementations
// ============================================================================

/// Real process runner that spawns actual processes
pub struct RealProcessRunner;

impl ProcessRunner for RealProcessRunner {
    fn run(&self, app: &App) -> Result<ExitStatus, std::io::Error> {
        Command::new(&app.command)
            .args(&app.args)
            .spawn()?
            .wait()
    }
}

/// Real filesystem implementation
pub struct RealFileSystem;

impl FileSystem for RealFileSystem {
    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn open(&self, path: &Path) -> Result<File, std::io::Error> {
        File::open(path)
    }

    fn write(&self, path: &Path, contents: &str) -> Result<(), std::io::Error> {
        std::fs::write(path, contents)
    }
}

/// Real environment implementation
pub struct RealEnvironment;

impl Environment for RealEnvironment {
    fn current_dir(&self) -> Result<PathBuf, std::io::Error> {
        env::current_dir()
    }
}

// ============================================================================
// Error Types
// ============================================================================

/// Custom error type for the dev application
#[derive(Debug)]
pub struct DevError {
    message: String,
}

impl DevError {
    pub fn new(msg: impl Into<String>) -> Self {
        DevError {
            message: msg.into(),
        }
    }
}

impl fmt::Display for DevError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for DevError {}

impl From<std::io::Error> for DevError {
    fn from(err: std::io::Error) -> Self {
        DevError::new(err.to_string())
    }
}

impl From<serde_json::Error> for DevError {
    fn from(err: serde_json::Error) -> Self {
        DevError::new(format!("JSON parse error: {}", err))
    }
}

// ============================================================================
// Configuration Types
// ============================================================================

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct Window {
    pub name: String,
    pub actions: Vec<String>,
    pub pwd: String,
    pub select: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct Config {
    pub session: String,
    pub windows: Vec<Window>,
}

/// Create the default 4-window configuration
pub fn default_config(session_name: &str) -> Config {
    Config {
        session: session_name.to_string(),
        windows: vec![
            Window {
                name: "Code".to_string(),
                actions: vec![],
                pwd: ".".to_string(),
                select: true,
            },
            Window {
                name: "Zsh".to_string(),
                actions: vec![],
                pwd: ".".to_string(),
                select: false,
            },
            Window {
                name: "Server".to_string(),
                actions: vec![],
                pwd: ".".to_string(),
                select: false,
            },
            Window {
                name: "UI".to_string(),
                actions: vec![],
                pwd: ".".to_string(),
                select: false,
            },
        ],
    }
}

/// Initialize a new .dev.json config file in the current directory
pub fn init_config<F: FileSystem, E: Environment>(fs: &F, env: &E) -> Result<(), DevError> {
    let config_path = Path::new(".dev.json");

    if fs.exists(config_path) {
        return Err(DevError::new(".dev.json already exists"));
    }

    let session_name = get_session_name_from_cwd(env)?;
    let config = default_config(&session_name);
    let json = serde_json::to_string_pretty(&config)?;

    fs.write(config_path, &json)?;
    println!("Created .dev.json");
    Ok(())
}

// ============================================================================
// App Builder Functions (Pure, No Side Effects)
// ============================================================================

/// Helper to create a tmux App for common operations
pub fn tmux_app(args: Vec<String>) -> App {
    App {
        command: String::from("tmux"),
        args,
    }
}

/// Create a new tmux window
pub fn create_window(session_name: &str, index: usize, window_name: &str) -> App {
    tmux_app(vec![
        "new-window".to_string(),
        "-t".to_string(),
        format!("{}:{}", session_name, index),
        "-n".to_string(),
        window_name.to_string(),
    ])
}

/// Send keys to a tmux pane
pub fn send_keys(target: &str, keys: &str) -> App {
    tmux_app(vec![
        "send-keys".to_string(),
        "-t".to_string(),
        target.to_string(),
        keys.to_string(),
        "C-m".to_string(),
    ])
}

/// Select a tmux window
pub fn select_window(target: &str) -> App {
    tmux_app(vec![
        "select-window".to_string(),
        "-t".to_string(),
        target.to_string(),
    ])
}

// ============================================================================
// Core Functions with Dependency Injection
// ============================================================================

/// Run a list of apps and print out the command and its arguments before running
pub fn run_apps<P: ProcessRunner>(runner: &P, apps: &[App]) -> Result<(), DevError> {
    for app in apps.iter() {
        println!();
        println!("========================");
        println!("$ {} {}", app.command, Args(app.args.to_owned()));
        println!("========================");

        runner
            .run(app)
            .map_err(|e| DevError::new(format!("Command failed: {}", e)))?;
    }
    Ok(())
}

/// Check if a tmux session exists
pub fn session_exists<P: ProcessRunner>(runner: &P, session_name: &str) -> Result<bool, DevError> {
    let app = tmux_app(vec![
        "has-session".to_string(),
        format!("-t={}", session_name),
    ]);

    let status = runner
        .run(&app)
        .map_err(|e| DevError::new(format!("Failed to check session: {}", e)))?;

    Ok(status.code() == Some(0))
}

/// Attach to an existing tmux session
pub fn attach_session<P: ProcessRunner>(runner: &P, session_name: &str) -> Result<(), DevError> {
    let app = tmux_app(vec![
        "attach".to_string(),
        "-t".to_string(),
        session_name.to_string(),
    ]);

    runner
        .run(&app)
        .map_err(|e| DevError::new(format!("Failed to attach to session: {}", e)))?;
    Ok(())
}

/// Get the current directory name as session name
pub fn get_session_name_from_cwd<E: Environment>(env: &E) -> Result<String, DevError> {
    let cwd = env.current_dir()?;
    let current_dir = Path::new(&cwd);
    current_dir
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string())
        .ok_or_else(|| DevError::new("Could not determine current directory name"))
}

/// Read configuration from a JSON file
pub fn read_config_from_file<F: FileSystem>(fs: &F, path: &Path) -> Result<Config, DevError> {
    let file = fs.open(path)?;
    let reader = BufReader::new(file);
    let config = serde_json::from_reader(reader)?;
    Ok(config)
}

/// Run the default tmux layout
pub fn run_default<P: ProcessRunner, E: Environment>(
    runner: &P,
    env: &E,
) -> Result<(), DevError> {
    let session_name = get_session_name_from_cwd(env)?;

    if session_exists(runner, &session_name)? {
        return attach_session(runner, &session_name);
    }

    // start a new session
    let tmux_new_session = tmux_app(vec![
        "new-session".to_string(),
        "-s".to_string(),
        session_name.clone(),
        "-n".to_string(),
        "Code".to_string(),
        "-d".to_string(),
    ]);

    let tmux_select_window = tmux_app(vec![
        "select-window".to_string(),
        "-t".to_string(),
        format!("{}:0.0", session_name),
    ]);

    let apps: Vec<App> = vec![
        tmux_new_session,
        create_window(&session_name, 1, "Zsh"),
        create_window(&session_name, 2, "Server"),
        create_window(&session_name, 3, "UI"),
        tmux_select_window,
    ];

    run_apps(runner, &apps)?;
    attach_session(runner, &session_name)
}

/// Run with a JSON configuration
pub fn run_with_json_config<P: ProcessRunner>(
    runner: &P,
    config: Config,
) -> Result<(), DevError> {
    let session_name = &config.session;

    if session_exists(runner, session_name)? {
        return attach_session(runner, session_name);
    }

    // Get first window name, default to "Main" if no windows defined
    let first_window_name = config
        .windows
        .first()
        .map(|w| w.name.clone())
        .unwrap_or_else(|| "Main".to_string());

    // Start a new session with the first window
    let tmux_new_session = tmux_app(vec![
        "new-session".to_string(),
        "-s".to_string(),
        session_name.clone(),
        "-n".to_string(),
        first_window_name,
        "-d".to_string(),
    ]);

    let mut windows: Vec<App> = vec![tmux_new_session];
    let mut directories: Vec<App> = vec![];
    let mut commands: Vec<App> = vec![];
    let mut selected_window: Option<App> = None;

    for (index, window) in config.windows.iter().enumerate() {
        let base_split = format!("{}:{}.0", session_name, index);

        // Create window (skip first since it's created with the session)
        if index > 0 {
            windows.push(create_window(session_name, index, &window.name));
        }

        // Build directory PWD command
        directories.push(send_keys(&base_split, &format!("cd {}", &window.pwd)));

        // Build action commands
        for action in &window.actions {
            commands.push(send_keys(&base_split, action));
        }

        // Track which window to select
        if window.select {
            selected_window = Some(select_window(&base_split));
        }
    }

    // Create windows
    run_apps(runner, &windows)?;
    // Change directories
    run_apps(runner, &directories)?;
    // Run commands
    run_apps(runner, &commands)?;

    // Select window if specified
    if let Some(ref app) = selected_window {
        runner
            .run(app)
            .map_err(|e| DevError::new(format!("Failed to select window: {}", e)))?;
    }

    attach_session(runner, session_name)
}

/// Run with a Zellij KDL configuration
pub fn run_with_kdl_config<P: ProcessRunner>(
    runner: &P,
    layout_path: &str,
) -> Result<(), DevError> {
    let app = App {
        command: String::from("zellij"),
        args: vec!["--layout".to_string(), layout_path.to_string()],
    };

    runner
        .run(&app)
        .map_err(|e| DevError::new(format!("Failed to run zellij: {}", e)))?;
    Ok(())
}

/// Main run function
pub fn run<P: ProcessRunner, F: FileSystem, E: Environment>(
    runner: &P,
    fs: &F,
    env: &E,
) -> Result<(), DevError> {
    let config_json = Path::new("./.dev.json");
    let config_kdl = Path::new("./.dev.kdl");

    // Check for JSON config first
    if fs.exists(config_json) {
        let config = read_config_from_file(fs, config_json)?;
        return run_with_json_config(runner, config);
    }

    // Check for KDL config
    if fs.exists(config_kdl) {
        return run_with_kdl_config(runner, "./.dev.kdl");
    }

    // Fall back to default layout
    run_default(runner, env)
}

// ============================================================================
// Main Entry Point
// ============================================================================

fn main() {
    let args: Vec<String> = env::args().collect();

    let runner = RealProcessRunner;
    let fs = RealFileSystem;
    let env = RealEnvironment;

    // Handle --init flag
    if args.len() > 1 && args[1] == "--init" {
        if let Err(e) = init_config(&fs, &env) {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
        return;
    }

    if let Err(e) = run(&runner, &fs, &env) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::os::unix::process::ExitStatusExt;

    // ========================================================================
    // Mock Implementations
    // ========================================================================

    /// Mock process runner that records calls and returns configurable exit codes
    struct MockProcessRunner {
        calls: RefCell<Vec<App>>,
        return_codes: RefCell<Vec<i32>>,
    }

    impl MockProcessRunner {
        fn new() -> Self {
            MockProcessRunner {
                calls: RefCell::new(Vec::new()),
                return_codes: RefCell::new(Vec::new()),
            }
        }

        fn with_return_codes(codes: Vec<i32>) -> Self {
            MockProcessRunner {
                calls: RefCell::new(Vec::new()),
                return_codes: RefCell::new(codes),
            }
        }

        fn get_calls(&self) -> Vec<App> {
            self.calls.borrow().clone()
        }
    }

    impl ProcessRunner for MockProcessRunner {
        fn run(&self, app: &App) -> Result<ExitStatus, std::io::Error> {
            self.calls.borrow_mut().push(app.clone());
            let code = self
                .return_codes
                .borrow_mut()
                .pop()
                .unwrap_or(0);
            Ok(ExitStatus::from_raw(code << 8)) // Exit codes are shifted on Unix
        }
    }

    /// Mock filesystem
    struct MockFileSystem {
        files: RefCell<HashMap<PathBuf, String>>,
    }

    impl MockFileSystem {
        fn new() -> Self {
            MockFileSystem {
                files: RefCell::new(HashMap::new()),
            }
        }

        fn with_file(self, path: &str, contents: &str) -> Self {
            self.files.borrow_mut().insert(PathBuf::from(path), contents.to_string());
            self
        }

        fn get_written(&self, path: &str) -> Option<String> {
            self.files.borrow().get(&PathBuf::from(path)).cloned()
        }
    }

    impl FileSystem for MockFileSystem {
        fn exists(&self, path: &Path) -> bool {
            self.files.borrow().contains_key(path)
        }

        fn open(&self, _path: &Path) -> Result<File, std::io::Error> {
            // For testing, we need to create a real temp file
            // This is a limitation - we'll test file reading separately
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Mock file not found",
            ))
        }

        fn write(&self, path: &Path, contents: &str) -> Result<(), std::io::Error> {
            self.files.borrow_mut().insert(path.to_path_buf(), contents.to_string());
            Ok(())
        }
    }

    /// Mock environment
    struct MockEnvironment {
        cwd: PathBuf,
    }

    impl MockEnvironment {
        fn new(cwd: &str) -> Self {
            MockEnvironment {
                cwd: PathBuf::from(cwd),
            }
        }
    }

    impl Environment for MockEnvironment {
        fn current_dir(&self) -> Result<PathBuf, std::io::Error> {
            Ok(self.cwd.clone())
        }
    }

    // ========================================================================
    // Phase 1: Pure Function Tests
    // ========================================================================

    mod args_tests {
        use super::*;

        #[test]
        fn test_args_display_empty() {
            let args = Args(vec![]);
            assert_eq!(format!("{}", args), "");
        }

        #[test]
        fn test_args_display_single() {
            let args = Args(vec!["foo".to_string()]);
            assert_eq!(format!("{}", args), "foo");
        }

        #[test]
        fn test_args_display_multiple() {
            let args = Args(vec!["foo".to_string(), "bar".to_string(), "baz".to_string()]);
            assert_eq!(format!("{}", args), "foo bar baz");
        }

        #[test]
        fn test_args_display_with_spaces() {
            let args = Args(vec!["hello world".to_string(), "test".to_string()]);
            assert_eq!(format!("{}", args), "hello world test");
        }
    }

    mod app_builder_tests {
        use super::*;

        #[test]
        fn test_tmux_app() {
            let app = tmux_app(vec!["arg1".to_string(), "arg2".to_string()]);
            assert_eq!(app.command, "tmux");
            assert_eq!(app.args, vec!["arg1", "arg2"]);
        }

        #[test]
        fn test_tmux_app_empty_args() {
            let app = tmux_app(vec![]);
            assert_eq!(app.command, "tmux");
            assert!(app.args.is_empty());
        }

        #[test]
        fn test_create_window() {
            let app = create_window("my-session", 2, "Editor");
            assert_eq!(app.command, "tmux");
            assert_eq!(
                app.args,
                vec!["new-window", "-t", "my-session:2", "-n", "Editor"]
            );
        }

        #[test]
        fn test_create_window_index_zero() {
            let app = create_window("test", 0, "First");
            assert_eq!(app.args[2], "test:0");
        }

        #[test]
        fn test_send_keys() {
            let app = send_keys("session:0.0", "npm start");
            assert_eq!(app.command, "tmux");
            assert_eq!(
                app.args,
                vec!["send-keys", "-t", "session:0.0", "npm start", "C-m"]
            );
        }

        #[test]
        fn test_send_keys_with_special_chars() {
            let app = send_keys("s:1.0", "echo 'hello'");
            assert_eq!(app.args[3], "echo 'hello'");
        }

        #[test]
        fn test_select_window() {
            let app = select_window("my-session:1.0");
            assert_eq!(app.command, "tmux");
            assert_eq!(app.args, vec!["select-window", "-t", "my-session:1.0"]);
        }
    }

    mod dev_error_tests {
        use super::*;

        #[test]
        fn test_dev_error_new() {
            let err = DevError::new("test error");
            assert_eq!(err.message, "test error");
        }

        #[test]
        fn test_dev_error_display() {
            let err = DevError::new("display test");
            assert_eq!(format!("{}", err), "display test");
        }

        #[test]
        fn test_dev_error_from_io_error() {
            let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
            let dev_err: DevError = io_err.into();
            assert!(dev_err.message.contains("file not found"));
        }

        #[test]
        fn test_dev_error_from_json_error() {
            let json_str = "{ invalid json }";
            let json_err = serde_json::from_str::<Config>(json_str).unwrap_err();
            let dev_err: DevError = json_err.into();
            assert!(dev_err.message.starts_with("JSON parse error:"));
        }
    }

    mod config_tests {
        use super::*;

        #[test]
        fn test_config_deserialize_valid() {
            let json = r#"{
                "session": "my-app",
                "windows": [
                    {
                        "name": "Editor",
                        "actions": ["vim"],
                        "pwd": ".",
                        "select": true
                    }
                ]
            }"#;

            let config: Config = serde_json::from_str(json).unwrap();
            assert_eq!(config.session, "my-app");
            assert_eq!(config.windows.len(), 1);
            assert_eq!(config.windows[0].name, "Editor");
            assert_eq!(config.windows[0].actions, vec!["vim"]);
            assert_eq!(config.windows[0].pwd, ".");
            assert!(config.windows[0].select);
        }

        #[test]
        fn test_config_deserialize_empty_windows() {
            let json = r#"{
                "session": "empty",
                "windows": []
            }"#;

            let config: Config = serde_json::from_str(json).unwrap();
            assert_eq!(config.session, "empty");
            assert!(config.windows.is_empty());
        }

        #[test]
        fn test_config_deserialize_multiple_windows() {
            let json = r#"{
                "session": "multi",
                "windows": [
                    {"name": "One", "actions": [], "pwd": ".", "select": false},
                    {"name": "Two", "actions": ["cmd1", "cmd2"], "pwd": "./sub", "select": true}
                ]
            }"#;

            let config: Config = serde_json::from_str(json).unwrap();
            assert_eq!(config.windows.len(), 2);
            assert_eq!(config.windows[0].name, "One");
            assert_eq!(config.windows[1].name, "Two");
            assert_eq!(config.windows[1].actions.len(), 2);
        }

        #[test]
        fn test_config_deserialize_missing_field() {
            let json = r#"{"session": "test"}"#;
            let result: Result<Config, _> = serde_json::from_str(json);
            assert!(result.is_err());
        }

        #[test]
        fn test_window_deserialize_valid() {
            let json = r#"{
                "name": "Test",
                "actions": ["action1"],
                "pwd": "/home",
                "select": false
            }"#;

            let window: Window = serde_json::from_str(json).unwrap();
            assert_eq!(window.name, "Test");
            assert!(!window.select);
        }
    }

    // ========================================================================
    // Phase 5: Integration Tests with Mocks
    // ========================================================================

    mod session_exists_tests {
        use super::*;

        #[test]
        fn test_session_exists_returns_true() {
            let runner = MockProcessRunner::with_return_codes(vec![0]);
            let result = session_exists(&runner, "my-session").unwrap();
            assert!(result);

            let calls = runner.get_calls();
            assert_eq!(calls.len(), 1);
            assert_eq!(calls[0].command, "tmux");
            assert!(calls[0].args.contains(&"has-session".to_string()));
        }

        #[test]
        fn test_session_exists_returns_false() {
            let runner = MockProcessRunner::with_return_codes(vec![1]);
            let result = session_exists(&runner, "nonexistent").unwrap();
            assert!(!result);
        }
    }

    mod attach_session_tests {
        use super::*;

        #[test]
        fn test_attach_session_success() {
            let runner = MockProcessRunner::new();
            let result = attach_session(&runner, "my-session");
            assert!(result.is_ok());

            let calls = runner.get_calls();
            assert_eq!(calls.len(), 1);
            assert_eq!(calls[0].command, "tmux");
            assert_eq!(calls[0].args, vec!["attach", "-t", "my-session"]);
        }
    }

    mod run_apps_tests {
        use super::*;

        #[test]
        fn test_run_apps_empty() {
            let runner = MockProcessRunner::new();
            let result = run_apps(&runner, &[]);
            assert!(result.is_ok());
            assert!(runner.get_calls().is_empty());
        }

        #[test]
        fn test_run_apps_multiple() {
            let runner = MockProcessRunner::new();
            let apps = vec![
                tmux_app(vec!["cmd1".to_string()]),
                tmux_app(vec!["cmd2".to_string()]),
            ];

            let result = run_apps(&runner, &apps);
            assert!(result.is_ok());
            assert_eq!(runner.get_calls().len(), 2);
        }
    }

    mod get_session_name_tests {
        use super::*;

        #[test]
        fn test_get_session_name_from_cwd() {
            let env = MockEnvironment::new("/home/user/my-project");
            let result = get_session_name_from_cwd(&env).unwrap();
            assert_eq!(result, "my-project");
        }

        #[test]
        fn test_get_session_name_nested_path() {
            let env = MockEnvironment::new("/a/b/c/deep-folder");
            let result = get_session_name_from_cwd(&env).unwrap();
            assert_eq!(result, "deep-folder");
        }
    }

    mod run_default_tests {
        use super::*;

        #[test]
        fn test_run_default_session_exists() {
            // Return 0 for has-session (exists), 0 for attach
            let runner = MockProcessRunner::with_return_codes(vec![0, 0]);
            let env = MockEnvironment::new("/home/user/test-project");

            let result = run_default(&runner, &env);
            assert!(result.is_ok());

            let calls = runner.get_calls();
            // Should only call has-session and attach
            assert_eq!(calls.len(), 2);
            assert!(calls[0].args.contains(&"has-session".to_string()));
            assert!(calls[1].args.contains(&"attach".to_string()));
        }

        #[test]
        fn test_run_default_creates_session() {
            // Return 1 for has-session (doesn't exist), then 0 for all other commands
            let runner = MockProcessRunner::with_return_codes(vec![0, 0, 0, 0, 0, 0, 0, 1]);
            let env = MockEnvironment::new("/home/user/new-project");

            let result = run_default(&runner, &env);
            assert!(result.is_ok());

            let calls = runner.get_calls();
            // Should create session, windows, select, and attach
            assert!(calls.len() > 2);

            // First call should be has-session
            assert!(calls[0].args.contains(&"has-session".to_string()));

            // Should include new-session
            let has_new_session = calls.iter().any(|c| c.args.contains(&"new-session".to_string()));
            assert!(has_new_session);
        }
    }

    mod run_with_json_config_tests {
        use super::*;

        #[test]
        fn test_run_with_json_config_session_exists() {
            let runner = MockProcessRunner::with_return_codes(vec![0, 0]);
            let config = Config {
                session: "existing".to_string(),
                windows: vec![Window {
                    name: "Main".to_string(),
                    actions: vec![],
                    pwd: ".".to_string(),
                    select: true,
                }],
            };

            let result = run_with_json_config(&runner, config);
            assert!(result.is_ok());

            let calls = runner.get_calls();
            assert_eq!(calls.len(), 2); // has-session + attach
        }

        #[test]
        fn test_run_with_json_config_creates_windows() {
            // Session doesn't exist
            let runner = MockProcessRunner::with_return_codes(vec![0, 0, 0, 0, 0, 0, 1]);
            let config = Config {
                session: "new-session".to_string(),
                windows: vec![
                    Window {
                        name: "Build".to_string(),
                        actions: vec!["npm run build".to_string()],
                        pwd: ".".to_string(),
                        select: true,
                    },
                    Window {
                        name: "Test".to_string(),
                        actions: vec![],
                        pwd: "./tests".to_string(),
                        select: false,
                    },
                ],
            };

            let result = run_with_json_config(&runner, config);
            assert!(result.is_ok());

            let calls = runner.get_calls();

            // Should have new-session call
            let has_new_session = calls.iter().any(|c| c.args.contains(&"new-session".to_string()));
            assert!(has_new_session);

            // Should have new-window for second window
            let has_new_window = calls.iter().any(|c| c.args.contains(&"new-window".to_string()));
            assert!(has_new_window);

            // Should have send-keys for actions
            let has_send_keys = calls.iter().any(|c| c.args.contains(&"send-keys".to_string()));
            assert!(has_send_keys);
        }
    }

    mod run_with_kdl_config_tests {
        use super::*;

        #[test]
        fn test_run_with_kdl_config() {
            let runner = MockProcessRunner::new();
            let result = run_with_kdl_config(&runner, "./.dev.kdl");
            assert!(result.is_ok());

            let calls = runner.get_calls();
            assert_eq!(calls.len(), 1);
            assert_eq!(calls[0].command, "zellij");
            assert_eq!(calls[0].args, vec!["--layout", "./.dev.kdl"]);
        }
    }

    mod run_tests {
        use super::*;

        #[test]
        fn test_run_no_config_files() {
            let runner = MockProcessRunner::with_return_codes(vec![0, 0, 0, 0, 0, 0, 0, 1]);
            let fs = MockFileSystem::new(); // No files
            let env = MockEnvironment::new("/home/user/project");

            let result = run(&runner, &fs, &env);
            assert!(result.is_ok());

            // Should run default layout
            let calls = runner.get_calls();
            assert!(!calls.is_empty());
        }

        #[test]
        fn test_run_with_kdl_file() {
            let runner = MockProcessRunner::new();
            let fs = MockFileSystem::new().with_file("./.dev.kdl", "layout {}");
            let env = MockEnvironment::new("/home/user/project");

            let result = run(&runner, &fs, &env);
            assert!(result.is_ok());

            let calls = runner.get_calls();
            assert_eq!(calls[0].command, "zellij");
        }
    }

    mod default_config_tests {
        use super::*;

        #[test]
        fn test_default_config_creates_correct_structure() {
            let config = default_config("my-project");

            assert_eq!(config.session, "my-project");
            assert_eq!(config.windows.len(), 4);

            assert_eq!(config.windows[0].name, "Code");
            assert!(config.windows[0].select);

            assert_eq!(config.windows[1].name, "Zsh");
            assert!(!config.windows[1].select);

            assert_eq!(config.windows[2].name, "Server");
            assert_eq!(config.windows[3].name, "UI");
        }

        #[test]
        fn test_default_config_all_windows_have_current_dir() {
            let config = default_config("test");

            for window in &config.windows {
                assert_eq!(window.pwd, ".");
                assert!(window.actions.is_empty());
            }
        }
    }

    mod init_config_tests {
        use super::*;

        #[test]
        fn test_init_config_creates_file() {
            let fs = MockFileSystem::new();
            let env = MockEnvironment::new("/home/user/my-project");

            let result = init_config(&fs, &env);
            assert!(result.is_ok());

            let written = fs.get_written(".dev.json");
            assert!(written.is_some());

            // Verify it's valid JSON and has correct structure
            let config: Config = serde_json::from_str(&written.unwrap()).unwrap();
            assert_eq!(config.session, "my-project");
            assert_eq!(config.windows.len(), 4);
        }

        #[test]
        fn test_init_config_fails_if_file_exists() {
            let fs = MockFileSystem::new().with_file(".dev.json", "{}");
            let env = MockEnvironment::new("/home/user/project");

            let result = init_config(&fs, &env);
            assert!(result.is_err());
            assert!(result.unwrap_err().message.contains("already exists"));
        }
    }
}
