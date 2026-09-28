use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{App, LogLevel, Phase};
use crate::config;
use crate::util::{ellipsize, normalize_release_date, release_date_from_tag};

fn installed_release_date(cfg: &config::Config) -> Option<String> {
    normalize_release_date(cfg.last_release_date.as_deref())
        .or_else(|| cfg.last_release_tag.as_deref().and_then(release_date_from_tag))
}

fn translation_provider_parts(app: &App) -> (String, String) {
    if let Some(cfg) = &app.config {
        (cfg.repo_owner.clone(), cfg.repo_name.clone())
    } else {
        ("設定讀取失敗".to_string(), "-".to_string())
    }
}

fn detected_system_label() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "⊞ Windows"
    }
    #[cfg(target_os = "linux")]
    {
        "🐧 Linux"
    }
    #[cfg(target_os = "macos")]
    {
        "macOS"
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        "Unknown"
    }
}

pub fn draw_ui(frame: &mut Frame, app: &App) {
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(20), Constraint::Length(3)])
        .split(frame.area());

    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(46), Constraint::Length(1), Constraint::Min(20)])
        .split(root[0]);

    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(10),
            Constraint::Length(5),
            Constraint::Length(3),
            Constraint::Length(7),
            Constraint::Min(6),
        ])
        .split(content_chunks[0]);

    render_header(frame, left_chunks[0]);
    render_status_panel(frame, left_chunks[1], app);
    render_steps_panel(frame, left_chunks[2], app);
    render_bubble_panel(frame, left_chunks[3], app);
    render_info_panel(frame, left_chunks[4], app);

    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(8), Constraint::Length(8)])
        .split(content_chunks[2]);

    render_log_panel(frame, right_chunks[0], app);
    render_detail_panel(frame, right_chunks[1], app);

    render_footer(frame, root[1], app);
}

fn render_header(frame: &mut Frame, area: ratatui::layout::Rect) {
    let content_lines = vec![
        Line::from(vec![
            Span::styled(
                "Limbus",
                Style::default()
                    .fg(Color::Rgb(239, 68, 68))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(
                "Company",
                Style::default()
                    .fg(Color::Rgb(245, 158, 11))
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(Span::raw("")),
        Line::from(Span::styled(
            "零協會文本正體中文化轉換工具",
            Style::default()
                .fg(Color::Rgb(248, 250, 252))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::raw("")),
        Line::from(Span::styled(
            format!("系統偵測 : {}", detected_system_label()),
            Style::default()
                .fg(Color::Rgb(148, 163, 184))
                .add_modifier(Modifier::BOLD),
        )),
    ];

    let inner_height = area.height.saturating_sub(2) as usize;
    let top_padding = inner_height.saturating_sub(content_lines.len()) / 2;
    let mut lines = Vec::with_capacity(top_padding + content_lines.len());
    for _ in 0..top_padding {
        lines.push(Line::from(Span::raw("")));
    }
    lines.extend(content_lines);

    let header = Paragraph::new(lines)
        .alignment(ratatui::layout::Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(51, 65, 85))),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(header, area);
}

fn render_status_panel(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let text_date = app
        .config
        .as_ref()
        .and_then(installed_release_date)
        .unwrap_or_else(|| "尚未記錄".to_string());

    let content_lines = vec![Line::from(vec![
        Span::styled(
            "遊戲文本更新日期 : ",
            Style::default()
                .fg(Color::Rgb(148, 163, 184))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            text_date,
            Style::default()
                .fg(Color::Rgb(74, 222, 128))
                .add_modifier(Modifier::BOLD),
        ),
    ])];

    let inner_height = area.height.saturating_sub(2) as usize;
    let top_padding = inner_height.saturating_sub(content_lines.len()) / 2;
    let mut lines = Vec::with_capacity(top_padding + content_lines.len());
    for _ in 0..top_padding {
        lines.push(Line::from(Span::raw("")));
    }
    lines.extend(content_lines);

    let status = Paragraph::new(lines)
        .alignment(ratatui::layout::Alignment::Center)
        .block(
            Block::default()
                .title(" Status ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(51, 65, 85))),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(status, area);
}

fn render_steps_panel(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    // 沿用原本四段流程文案，最後一段為戰鬥氣泡著色
    let steps = ["初始化", "下載/解壓", "文字轉換", "氣泡/語音"];
    let current = app.current_step_index();

    let mut spans = Vec::new();
    for (index, step) in steps.iter().enumerate() {
        let style = if index < current {
            Style::default()
                .fg(Color::Rgb(16, 185, 129))
                .add_modifier(Modifier::BOLD)
        } else if index == current {
            Style::default()
                .fg(Color::Rgb(245, 158, 11))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Rgb(100, 116, 139))
        };

        let dot = if index < current {
            "●"
        } else if index == current {
            "◉"
        } else {
            "○"
        };

        spans.push(Span::styled(format!("{dot} {step}"), style));
        if index != steps.len() - 1 {
            spans.push(Span::styled(
                "  ─  ",
                Style::default().fg(Color::Rgb(71, 85, 105)),
            ));
        }
    }

    let content_line = Line::from(spans);
    let inner_height = area.height.saturating_sub(2) as usize;
    let top_padding = inner_height.saturating_sub(1) / 2;
    let mut lines = Vec::with_capacity(top_padding + 1);
    for _ in 0..top_padding {
        lines.push(Line::from(Span::raw("")));
    }
    lines.push(content_line);

    let panel = Paragraph::new(lines)
        .alignment(ratatui::layout::Alignment::Center)
        .block(
            Block::default()
                .title(" Pipeline ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(51, 65, 85))),
        );

    frame.render_widget(panel, area);
}

/// 戰鬥語音氣泡區塊：原有說明文字垂直／水平置中。
fn render_bubble_panel(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let active = app.phase == Phase::Running && app.current_step_index() >= 3;
    let title_style = Style::default()
        .fg(if active {
            Color::Rgb(245, 158, 11)
        } else {
            Color::Rgb(248, 250, 252)
        })
        .add_modifier(Modifier::BOLD);
    let body_style = Style::default()
        .fg(Color::Rgb(148, 163, 184))
        .add_modifier(Modifier::BOLD);

    let content_lines = vec![
        Line::from(Span::styled("戰鬥氣泡／人格語音", title_style)),
        Line::from(Span::raw("")),
        Line::from(Span::styled("依罪人代表色自動著色", body_style)),
        Line::from(Span::styled("強制覆寫輸出檔", body_style)),
    ];

    let inner_height = area.height.saturating_sub(2) as usize;
    let top_padding = inner_height.saturating_sub(content_lines.len()) / 2;
    let mut lines = Vec::with_capacity(top_padding + content_lines.len());
    for _ in 0..top_padding {
        lines.push(Line::from(Span::raw("")));
    }
    lines.extend(content_lines);

    let panel = Paragraph::new(lines)
        .alignment(ratatui::layout::Alignment::Center)
        .block(
            Block::default()
                .title(" Battle Bubble ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(51, 65, 85))),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(panel, area);
}

fn render_info_panel(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let (translation_owner, translation_repo) = translation_provider_parts(app);
    let section_style = Style::default()
        .fg(Color::Rgb(148, 163, 184))
        .add_modifier(Modifier::BOLD);
    let trans_owner_style = Style::default().fg(Color::Rgb(74, 222, 128));
    let trans_repo_style = Style::default().fg(Color::Rgb(190, 242, 100));

    let lines = vec![
        Line::from(Span::styled("◆ 翻譯來源提供者", section_style)),
        Line::from(Span::styled(
            format!("  ├ {}", ellipsize(&translation_owner, 36)),
            trans_owner_style,
        )),
        Line::from(Span::styled(
            format!("  └ {}", ellipsize(&translation_repo, 36)),
            trans_repo_style,
        )),
    ];

    let info = Paragraph::new(lines).block(
        Block::default()
            .title(" Provider ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(51, 65, 85))),
    );

    frame.render_widget(info, area);
}

fn render_log_panel(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let visible_lines = area.height.saturating_sub(2) as usize;
    let total_logs = app.logs.len();

    let max_scroll = total_logs.saturating_sub(visible_lines);
    let scroll = app.log_scroll.min(max_scroll);
    let start_index = total_logs.saturating_sub(visible_lines.saturating_add(scroll));

    let items: Vec<ListItem> = app
        .logs
        .iter()
        .skip(start_index)
        .take(visible_lines)
        .map(|entry| {
            let (icon, color) = match entry.level {
                LogLevel::Info => ("•", Color::Rgb(148, 163, 184)),
                LogLevel::Success => ("✓", Color::Rgb(16, 185, 129)),
                LogLevel::Warn => ("⚠", Color::Rgb(245, 158, 11)),
                LogLevel::Error => ("✗", Color::Rgb(239, 68, 68)),
            };

            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("[{}] ", entry.timestamp),
                    Style::default().fg(Color::Rgb(100, 116, 139)),
                ),
                Span::styled(
                    format!("{icon} "),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(entry.message.clone(), Style::default().fg(color)),
            ]))
        })
        .collect();

    let shown_start = if total_logs == 0 { 0 } else { start_index + 1 };
    let shown_end = if total_logs == 0 {
        0
    } else {
        (start_index + visible_lines).min(total_logs)
    };

    let title = if scroll > 0 {
        format!(" Logs {shown_start}-{shown_end}/{total_logs} (↑{scroll}) ")
    } else {
        format!(" Logs {shown_start}-{shown_end}/{total_logs} ")
    };

    let logs = List::new(items).block(
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(51, 65, 85))),
    );

    frame.render_widget(logs, area);
}

fn render_detail_panel(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let current_file = app
        .current_file
        .as_deref()
        .map(|s| ellipsize(s, 48))
        .unwrap_or_else(|| "-".to_string());

    let file_counter = match (app.processed_files, app.total_files) {
        (Some(done), Some(total)) => format!("{done}/{total}"),
        _ => "-".to_string(),
    };

    let mut lines = vec![
        kv_line("目前檔案", &current_file),
        kv_line("處理進度", &file_counter),
    ];

    if let Some(err) = &app.error {
        lines.push(kv_line("錯誤", &ellipsize(err, 48)));
    } else {
        lines.push(kv_line("錯誤", "-"));
    }

    if let Some(output) = &app.game_info.output_path {
        lines.push(kv_line("輸出目標", &ellipsize(output, 48)));
    }

    if let Some(game_path) = &app.game_info.game_path {
        lines.push(kv_line("遊戲目錄", &ellipsize(game_path, 48)));
    }

    let panel = Paragraph::new(lines).block(
        Block::default()
            .title(" Details ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(51, 65, 85))),
    );

    frame.render_widget(panel, area);
}

fn render_footer(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let controls = if matches!(app.phase, Phase::Success | Phase::Failed) {
        "[S] 重新轉換  [R] 重載設定  [Q] 離開"
    } else if app.phase == Phase::Running {
        "[X] 取消轉換  [↑↓] 捲動日誌"
    } else {
        "[S] 開始轉換  [R] 重載設定  [Q] 離開  [↑↓] 捲動日誌"
    };

    let footer = Paragraph::new(Line::from(Span::styled(
        controls,
        Style::default()
            .fg(Color::Rgb(148, 163, 184))
            .add_modifier(Modifier::BOLD),
    )))
    .alignment(ratatui::layout::Alignment::Center)
    .block(
        Block::default()
            .title(" Controls ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(51, 65, 85))),
    );

    frame.render_widget(footer, area);
}

fn kv_line(label: &str, value: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("{label:<10} "),
            Style::default()
                .fg(Color::Rgb(148, 163, 184))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            value.to_string(),
            Style::default().fg(Color::Rgb(226, 232, 240)),
        ),
    ])
}