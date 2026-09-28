use std::collections::VecDeque;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{self, Receiver, TryRecvError},
    Arc,
};

use chrono::Local;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::config;
use crate::converter::Converter;
use crate::error::AppError;
use crate::steam_registry;
use crate::ui;

const MAX_LOG_LINES: usize = 250;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Ready,
    Running,
    Success,
    Failed,
}

#[derive(Debug, Clone, Copy)]
pub enum LogLevel {
    Info,
    Success,
    Warn,
    Error,
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: LogLevel,
    pub message: String,
}

#[derive(Debug, Clone)]
struct ProgressUpdate {
    progress: f64,
    message: String,
    current_file: Option<String>,
    total_files: Option<usize>,
    processed_files: Option<usize>,
}

enum WorkerEvent {
    Progress(ProgressUpdate),
    Finished(Result<(), String>),
}

#[derive(Debug, Clone, Default)]
pub struct GamePathInfo {
    pub found: bool,
    pub game_path: Option<String>,
    pub output_path: Option<String>,
}

pub struct App {
    pub phase: Phase,
    pub progress: f64,
    pub message: String,
    pub error: Option<String>,
    pub current_file: Option<String>,
    pub total_files: Option<usize>,
    pub processed_files: Option<usize>,
    pub logs: VecDeque<LogEntry>,
    pub log_scroll: usize,
    worker_rx: Option<Receiver<WorkerEvent>>,
    cancel_flag: Option<Arc<AtomicBool>>,
    pub config: Option<config::Config>,
    pub game_info: GamePathInfo,
    pub should_quit: bool,
    last_logged_message: Option<String>,
}

impl App {
    pub fn new() -> Self {
        let mut app = Self {
            phase: Phase::Ready,
            progress: 0.0,
            message: "按 S 開始轉換，按 Q 離開".to_string(),
            error: None,
            current_file: None,
            total_files: None,
            processed_files: None,
            logs: VecDeque::with_capacity(MAX_LOG_LINES),
            log_scroll: 0,
            worker_rx: None,
            cancel_flag: None,
            config: None,
            game_info: detect_game_path_info(),
            should_quit: false,
            last_logged_message: None,
        };

        app.reload_config();
        if app.game_info.found {
            app.push_log(LogLevel::Success, "已偵測到 Limbus Company 安裝路徑");
        } else {
            app.push_log(
                LogLevel::Warn,
                "找不到 Limbus Company 路徑，將使用預設輸出資料夾",
            );
        }

        app.push_log(LogLevel::Info, "TUI 已就緒，可開始執行轉換");
        app
    }

    pub fn push_log(&mut self, level: LogLevel, message: impl Into<String>) {
        let message = message.into();
        let timestamp = Local::now().format("%H:%M:%S").to_string();

        if self.logs.len() >= MAX_LOG_LINES {
            self.logs.pop_front();
        }

        self.logs.push_back(LogEntry {
            timestamp,
            level,
            message,
        });
    }

    pub fn reload_config(&mut self) {
        match config::load_config() {
            Ok(cfg) => {
                let ver = cfg.format_version;
                self.config = Some(cfg);
                self.error = None;
                if self.phase == Phase::Failed {
                    self.phase = Phase::Ready;
                    self.message = "按 S 開始轉換，按 Q 離開".to_string();
                }
                self.push_log(
                    LogLevel::Info,
                    format!("設定檔已載入（格式 v{ver}，舊版會自動遷移並覆寫）"),
                );
            }
            Err(err) => {
                // 理論上 load_config 已盡力自癒；仍失敗才標 Failed
                self.config = None;
                self.error = Some(err.to_string());
                self.phase = Phase::Failed;
                self.push_log(LogLevel::Error, format!("設定檔載入失敗: {err}"));
            }
        }
    }

    fn running(&self) -> bool {
        self.phase == Phase::Running
    }

    pub fn start_conversion(&mut self) {
        if self.running() {
            self.push_log(LogLevel::Warn, "轉換已在執行中");
            return;
        }

        let (tx, rx) = mpsc::channel::<WorkerEvent>();
        let cancel_flag = Arc::new(AtomicBool::new(false));
        let worker_cancel_flag = Arc::clone(&cancel_flag);

        self.worker_rx = Some(rx);
        self.cancel_flag = Some(cancel_flag);
        self.phase = Phase::Running;
        self.progress = 0.0;
        self.error = None;
        self.message = "啟動轉換工作...".to_string();
        self.current_file = None;
        self.total_files = None;
        self.processed_files = None;
        self.last_logged_message = None;
        self.push_log(LogLevel::Info, "開始執行轉換流程");

        std::thread::spawn(move || {
            let progress_tx = tx.clone();
            let callback = Box::new(
                move |progress: f64,
                      message: String,
                      current_file: Option<String>,
                      total_files: Option<usize>,
                      processed_files: Option<usize>| {
                    let _ = progress_tx.send(WorkerEvent::Progress(ProgressUpdate {
                        progress,
                        message,
                        current_file,
                        total_files,
                        processed_files,
                    }));
                },
            );

            let result = Converter::new_with_callback_and_cancel(callback, worker_cancel_flag)
                .and_then(|mut converter| converter.run())
                .map_err(|e| e.to_string());

            let _ = tx.send(WorkerEvent::Finished(result));
        });
    }

    pub fn request_cancel(&mut self) {
        if !self.running() {
            self.push_log(LogLevel::Warn, "目前沒有進行中的轉換");
            return;
        }

        if let Some(cancel_flag) = &self.cancel_flag {
            cancel_flag.store(true, Ordering::Relaxed);
            self.push_log(LogLevel::Warn, "已送出取消請求，等待安全停止...");
            self.message = "正在取消轉換...".to_string();
        }
    }

    fn handle_worker_event(&mut self, event: WorkerEvent) {
        match event {
            WorkerEvent::Progress(update) => {
                self.progress = update.progress.clamp(0.0, 100.0);
                self.message = update.message.clone();
                self.current_file = update.current_file;
                self.total_files = update.total_files;
                self.processed_files = update.processed_files;

                if self.last_logged_message.as_deref() != Some(update.message.as_str()) {
                    self.last_logged_message = Some(update.message.clone());
                    self.push_log(LogLevel::Info, update.message);
                }
            }
            WorkerEvent::Finished(result) => {
                self.worker_rx = None;
                self.cancel_flag = None;
                self.last_logged_message = None;

                match result {
                    Ok(()) => {
                        self.phase = Phase::Success;
                        self.progress = 100.0;
                        self.message = "轉換完成".to_string();
                        self.error = None;
                        self.push_log(LogLevel::Success, "轉換成功完成");
                        self.reload_config();
                    }
                    Err(err) => {
                        if err.contains("使用者已取消轉換") {
                            self.phase = Phase::Ready;
                            self.progress = 0.0;
                            self.message = "轉換已取消".to_string();
                            self.error = None;
                            self.push_log(LogLevel::Warn, "轉換已取消");
                        } else {
                            self.phase = Phase::Failed;
                            self.progress = 0.0;
                            self.message = "轉換失敗".to_string();
                            self.error = Some(err.clone());
                            self.push_log(LogLevel::Error, format!("轉換失敗: {err}"));
                        }
                    }
                }
            }
        }
    }

    fn poll_worker_events(&mut self) {
        let mut events = Vec::new();

        if let Some(rx) = &self.worker_rx {
            loop {
                match rx.try_recv() {
                    Ok(event) => events.push(event),
                    Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => break,
                }
            }
        }

        for event in events {
            self.handle_worker_event(event);
        }
    }

    pub fn tick(&mut self) {
        self.poll_worker_events();
    }

    pub fn current_step_index(&self) -> usize {
        if self.phase == Phase::Success {
            return 3;
        }

        // 進度對齊 converter：初始化 <10、下載/解壓 <55、文字轉換 <84、氣泡著色 ≥84
        let p = self.progress;
        if p < 10.0 {
            0 // 初始化
        } else if p < 55.0 {
            1 // 下載 / 解壓
        } else if p < 84.0 {
            2 // 文字轉換
        } else {
            3 // 氣泡著色及收尾
        }
    }

    pub fn scroll_logs_up(&mut self, amount: usize) {
        self.log_scroll = self.log_scroll.saturating_add(amount);
    }

    pub fn scroll_logs_down(&mut self, amount: usize) {
        self.log_scroll = self.log_scroll.saturating_sub(amount);
    }

    pub fn handle_key(&mut self, key: event::KeyEvent) {
        if key.kind != KeyEventKind::Press && key.kind != KeyEventKind::Repeat {
            return;
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                if self.running() {
                    self.request_cancel();
                } else {
                    self.should_quit = true;
                }
            }
            KeyCode::Char('s') => self.start_conversion(),
            KeyCode::Char('x') => self.request_cancel(),
            KeyCode::Char('r') => {
                self.reload_config();
                self.game_info = detect_game_path_info();
                self.push_log(LogLevel::Info, "已重新載入設定與路徑資訊");
            }
            KeyCode::Up => self.scroll_logs_up(1),
            KeyCode::Down => self.scroll_logs_down(1),
            KeyCode::PageUp => self.scroll_logs_up(10),
            KeyCode::PageDown => self.scroll_logs_down(10),
            KeyCode::Home => self.log_scroll = usize::MAX,
            KeyCode::End => self.log_scroll = 0,
            _ => {}
        }
    }
}

pub fn detect_game_path_info() -> GamePathInfo {
    match steam_registry::find_limbus_company_path() {
        Ok(game_path) => {
            let lang_path = game_path.join("LimbusCompany_Data").join("Lang");
            let output_path = lang_path.join("LLC_zh-Hant");

            GamePathInfo {
                found: true,
                game_path: Some(game_path.display().to_string()),
                output_path: Some(output_path.display().to_string()),
            }
        }
        Err(_) => GamePathInfo::default(),
    }
}

pub fn run_headless() -> Result<(), AppError> {
    Converter::new()?.run()
}

pub fn run_tui_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
) -> std::io::Result<()> {
    use std::time::Duration;

    let mut app = App::new();

    loop {
        app.tick();
        terminal.draw(|frame| ui::draw_ui(frame, &app))?;

        if app.should_quit {
            break;
        }

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                app.handle_key(key);
            }
        }
    }

    Ok(())
}