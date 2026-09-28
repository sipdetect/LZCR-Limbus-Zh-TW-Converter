use std::env;
use std::io;

use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

fn print_usage() {
    eprintln!("LZCR - Limbus Company 零協會文本正體中文轉換工具");
    eprintln!();
    eprintln!("用法:");
    eprintln!("  lzcr              啟動 TUI 介面（預設）");
    eprintln!("  lzcr --headless   無介面模式，直接執行轉換");
    eprintln!("  lzcr --help       顯示此說明");
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_usage();
        return;
    }

    if args.iter().any(|a| a == "--headless") {
        match lzcr::app::run_headless() {
            Ok(()) => {}
            Err(err) => {
                eprintln!("[ERROR] {err}");
                std::process::exit(1);
            }
        }
        return;
    }

    if let Err(err) = run_tui() {
        eprintln!("[ERROR] {err}");
        std::process::exit(1);
    }
}

fn run_tui() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let result = lzcr::app::run_tui_loop(&mut terminal);

    disable_raw_mode()?;
    terminal.backend_mut().execute(LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result.map_err(|e| e.into())
}