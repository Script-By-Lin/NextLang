use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::env;
use std::path::Path;
use std::process::{Child, Command};
use std::sync::mpsc::channel;
use std::time::{Duration, Instant};

/// RAII Guard ensuring child process is terminated when supervisor exits or restarts.
pub struct ChildProcessGuard {
    child: Option<Child>,
}

impl ChildProcessGuard {
    pub fn new() -> Self {
        Self { child: None }
    }

    pub fn set(&mut self, child: Child) {
        self.kill();
        self.child = Some(child);
    }

    pub fn kill(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Default for ChildProcessGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for ChildProcessGuard {
    fn drop(&mut self) {
        self.kill();
    }
}

/// Checks whether a modified filesystem path is relevant for triggering application reload.
pub fn is_watched_file(path: &Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if name.is_empty() {
        return false;
    }

    // Ignore hidden files, temporary files, editor backups, or swap files
    if name.starts_with('.')
        || name.starts_with('#')
        || name.ends_with('~')
        || name.ends_with(".swp")
        || name.ends_with(".tmp")
    {
        return false;
    }

    // Ignore build artifacts, VCS, and package caches
    for comp in path.components() {
        if let std::path::Component::Normal(c) = comp {
            let s = c.to_string_lossy();
            if s == ".git"
                || s == ".nextlang"
                || s == "target"
                || s == "dist"
                || s == ".nx_cache"
                || s == "scratch"
                || s == "node_modules"
            {
                return false;
            }
        }
    }

    // Watch NextLang source files and project manifest
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        if ext == "nx" {
            return true;
        }
    }
    if name == "nextlang.toml" {
        return true;
    }
    false
}

/// Spawns the application process using current executable running `nx run <entry_path>`.
pub fn spawn_app_process(
    entry_path: &Path,
    host: Option<&str>,
    port: Option<&str>,
) -> std::io::Result<Child> {
    let current_exe = env::current_exe()?;
    let mut cmd = Command::new(current_exe);
    cmd.arg("run").arg(entry_path);
    if let Some(h) = host {
        cmd.env("APP_HOST", h);
    }
    if let Some(p) = port {
        cmd.env("APP_PORT", p);
    }
    cmd.spawn()
}

/// Runs the development reload supervisor.
pub fn run_with_reload(
    watch_root: &Path,
    entry_path: &Path,
    host: Option<&str>,
    port: Option<&str>,
) -> Result<(), ()> {
    println!("============================================================");
    println!("  NextLang Development Reload Supervisor Enabled");
    println!("  Watching directory: {}", watch_root.display());
    println!("  Entry point: {}", entry_path.display());
    if let (Some(h), Some(p)) = (host, port) {
        println!("  Target: http://{}:{}", h, p);
    }
    println!("  Press Ctrl+C to stop.");
    println!("============================================================");

    let (tx, rx) = channel::<notify::Result<Event>>();
    let mut watcher = match RecommendedWatcher::new(
        move |res| {
            let _ = tx.send(res);
        },
        Config::default(),
    ) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("[NextAPI Reload Error] Failed to initialize watcher: {}", e);
            return Err(());
        }
    };

    if let Err(e) = watcher.watch(watch_root, RecursiveMode::Recursive) {
        eprintln!(
            "[NextAPI Reload Error] Failed to watch path '{}': {}",
            watch_root.display(),
            e
        );
        return Err(());
    }

    let mut guard = ChildProcessGuard::new();

    println!("[NextAPI Reload] Starting application...");
    match spawn_app_process(entry_path, host, port) {
        Ok(child) => {
            guard.set(child);
            println!("[NextAPI Reload] Application started successfully.");
        }
        Err(e) => {
            eprintln!("[NextAPI Reload Error] Failed to start application: {}", e);
            eprintln!("[NextAPI Reload] Waiting for file changes to reload...");
        }
    }

    let debounce_duration = Duration::from_millis(200);

    while let Ok(first_event) = rx.recv() {
        let mut changed_paths = Vec::new();
        if let Ok(event) = first_event {
            for p in event.paths {
                if is_watched_file(&p) {
                    changed_paths.push(p);
                }
            }
        }

        // Debounce subsequent rapid file events
        let start_debounce = Instant::now();
        while start_debounce.elapsed() < debounce_duration {
            let remaining = debounce_duration.saturating_sub(start_debounce.elapsed());
            if let Ok(Ok(event)) = rx.recv_timeout(remaining) {
                for p in event.paths {
                    if is_watched_file(&p) {
                        changed_paths.push(p);
                    }
                }
            }
        }

        if changed_paths.is_empty() {
            continue;
        }

        let first_changed = &changed_paths[0];
        let rel_path = first_changed
            .strip_prefix(watch_root)
            .unwrap_or(first_changed);
        println!(
            "\n[NextAPI Reload] Change detected in '{}'",
            rel_path.display()
        );
        println!("[NextAPI Reload] Restarting application...");

        // 1. Kill old application process to release bound ports and sockets
        guard.kill();

        // 2. Short pause for socket cleanup
        std::thread::sleep(Duration::from_millis(100));

        // 3. Restart application
        match spawn_app_process(entry_path, host, port) {
            Ok(child) => {
                guard.set(child);
                println!("[NextAPI Reload] Application restarted successfully.");
            }
            Err(e) => {
                eprintln!("[NextAPI Reload Error] Failed to restart application: {}", e);
                eprintln!("[NextAPI Reload] Waiting for file changes to reload...");
            }
        }
    }

    println!("\n[NextAPI Reload] Shutting down reload supervisor...");
    guard.kill();
    Ok(())
}
