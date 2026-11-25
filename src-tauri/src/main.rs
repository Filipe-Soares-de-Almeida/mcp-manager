#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod backup;
mod claude;
mod config;
mod gui;
mod launch;
mod locale;
mod manager;
mod mcp;
mod ops;
mod paths;
mod store;
mod watcher;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mode = |flag: &str| args.iter().any(|a| a == flag);
    let result = if mode("--mcp") {
        mcp::run()
    } else if mode("--watch") {
        watcher::run()
    } else {
        gui::run();
        Ok(())
    };
    if let Err(e) = result {
        eprintln!("mcp-manager: {e:#}");
        paths::log(&paths::data_dir(), &format!("exited with an error: {e:#}"));
        std::process::exit(1);
    }
}
