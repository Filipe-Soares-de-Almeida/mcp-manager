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
mod update;
mod watcher;

/// The value after `flag`, as in `--wait-pid 1234`.
fn arg_value<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).map(String::as_str)
}

/// `mcp-manager.exe --update`: checks the release feed and installs a newer
/// version in place, without the window.
fn update_from_cli() -> anyhow::Result<()> {
    let data = paths::data_dir();
    let (check, feed) = update::check("en")?;
    if !check.available {
        paths::log(&data, &format!("--update: {} is the latest version", check.current));
        return Ok(());
    }
    let exe = std::env::current_exe()?;
    update::install(&feed, &exe, &|_, _| {})?;
    paths::log(&data, &format!("--update: {} replaced by {}", check.current, check.latest));
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mode = |flag: &str| args.iter().any(|a| a == flag);
    let result = if mode("--mcp") {
        mcp::run()
    } else if mode("--watch") {
        watcher::run()
    } else if mode("--update") {
        update_from_cli()
    } else {
        // After an update the old process may still be closing; the single
        // instance lock would hand this start to it.
        if let Some(pid) = arg_value(&args, "--wait-pid").and_then(|p| p.parse().ok()) {
            launch::wait_for_exit(pid, std::time::Duration::from_secs(20));
        }
        gui::run(arg_value(&args, "--updated-from").map(str::to_string));
        Ok(())
    };
    if let Err(e) = result {
        eprintln!("mcp-manager: {e:#}");
        paths::log(&paths::data_dir(), &format!("exited with an error: {e:#}"));
        std::process::exit(1);
    }
}
