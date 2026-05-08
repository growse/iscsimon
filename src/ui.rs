use crate::app::App;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Cell, Clear, Paragraph, Row, Table,
    },
};

const HEADER_TITLES: [&str; 7] = [
    "Target IQN",
    "Initiator IQN",
    "Device",
    "Read Rate",
    "Write Rate",
    "Total Read",
    "Total Write",
];

const COL_WIDTHS: [Constraint; 7] = [
    Constraint::Percentage(24),
    Constraint::Percentage(24),
    Constraint::Percentage(20),
    Constraint::Percentage(8),
    Constraint::Percentage(8),
    Constraint::Percentage(8),
    Constraint::Percentage(8),
];

pub fn draw(f: &mut Frame, app: &mut App) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    draw_header(f, app, chunks[0]);
    draw_table(f, app, chunks[1]);
    draw_footer(f, chunks[2]);

    if app.show_help {
        draw_help(f, area);
    }

    if let Some(ref err) = app.error.clone() {
        draw_error(f, area, err);
    }
}

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let session_count = app.sessions.len();
    let ip_str = if app.source_ips.is_empty() {
        String::from("no connections")
    } else {
        app.source_ips.join(", ")
    };

    let title = format!(
        " iscsimon — {} session{} — {} — {} ",
        session_count,
        if session_count == 1 { "" } else { "s" },
        ip_str,
        now
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(Span::styled(
            " iSCSI Monitor ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ));

    let para = Paragraph::new(title)
        .block(block)
        .alignment(Alignment::Center);
    f.render_widget(para, area);
}

fn draw_table(f: &mut Frame, app: &mut App, area: Rect) {
    let header_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let selected_style = Style::default()
        .bg(Color::DarkGray)
        .add_modifier(Modifier::BOLD);

    let header_cells: Vec<Cell> = HEADER_TITLES
        .iter()
        .map(|h| Cell::from(*h).style(header_style))
        .collect();
    let header = Row::new(header_cells).height(1).bottom_margin(0);

    let rows: Vec<Row> = app
        .sessions
        .iter()
        .map(|s| {
            let device = s
                .luns
                .first()
                .map(|l| abbreviate_device(&l.device))
                .unwrap_or_else(|| String::from("—"));

            let cells = vec![
                Cell::from(abbreviate_iqn(&s.target_iqn)),
                Cell::from(abbreviate_iqn(&s.initiator_iqn)),
                Cell::from(device),
                Cell::from(format_rate(s.read_rate)),
                Cell::from(format_rate(s.write_rate)),
                Cell::from(format_mb(s.total_read_mb)),
                Cell::from(format_mb(s.total_write_mb)),
            ];
            Row::new(cells).height(1)
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(Span::styled(
            " Sessions ",
            Style::default().fg(Color::White),
        ));

    let table = Table::new(rows, COL_WIDTHS)
        .header(header)
        .block(block)
        .row_highlight_style(selected_style);

    f.render_stateful_widget(table, area, &mut app.table_state);
}

fn draw_footer(f: &mut Frame, area: Rect) {
    let keys = vec![
        ("↑/↓", "navigate"),
        ("g/G", "top/bottom"),
        ("?", "help"),
        ("q", "quit"),
    ];

    let spans: Vec<Span> = keys
        .iter()
        .flat_map(|(key, desc)| {
            vec![
                Span::styled(*key, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::styled(format!(" {} ", desc), Style::default().fg(Color::DarkGray)),
                Span::styled("│", Style::default().fg(Color::DarkGray)),
                Span::raw(" "),
            ]
        })
        .collect();

    let para = Paragraph::new(Line::from(spans)).alignment(Alignment::Center);
    f.render_widget(para, area);
}

fn draw_help(f: &mut Frame, area: Rect) {
    let help_text = vec![
        Line::from(Span::styled(
            " Keyboard Shortcuts ",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ↑ / k    ", Style::default().fg(Color::Cyan)),
            Span::raw("Move selection up"),
        ]),
        Line::from(vec![
            Span::styled("  ↓ / j    ", Style::default().fg(Color::Cyan)),
            Span::raw("Move selection down"),
        ]),
        Line::from(vec![
            Span::styled("  g        ", Style::default().fg(Color::Cyan)),
            Span::raw("Jump to first session"),
        ]),
        Line::from(vec![
            Span::styled("  G        ", Style::default().fg(Color::Cyan)),
            Span::raw("Jump to last session"),
        ]),
        Line::from(vec![
            Span::styled("  ?        ", Style::default().fg(Color::Cyan)),
            Span::raw("Toggle this help"),
        ]),
        Line::from(vec![
            Span::styled("  q / Esc  ", Style::default().fg(Color::Cyan)),
            Span::raw("Quit"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Note: Read/write rates have 1 MB/s resolution",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            "  (sourced from kernel scsi_tgt_port statistics)",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let popup = centered_rect(50, 60, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(Span::styled(" Help ", Style::default().fg(Color::Cyan)));

    let para = Paragraph::new(help_text).block(block);

    f.render_widget(Clear, popup);
    f.render_widget(para, popup);
}

fn draw_error(f: &mut Frame, area: Rect, err: &str) {
    let popup = centered_rect(60, 20, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red))
        .title(Span::styled(
            " Error ",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ));

    let para = Paragraph::new(err.to_string())
        .block(block)
        .style(Style::default().fg(Color::Red));

    f.render_widget(Clear, popup);
    f.render_widget(para, popup);
}

/// Return a rect centered in `area` with the given percentage dimensions.
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vert = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vert[1])[1]
}

/// Show just the last component of an IQN: `iqn.2024-12.com.growse.mill:foo` → `foo`
fn abbreviate_iqn(iqn: &str) -> String {
    iqn.rsplit(':')
        .next()
        .map(str::to_string)
        .unwrap_or_else(|| iqn.to_string())
}

/// Strip `/dev/` prefix from device path for compact display.
fn abbreviate_device(dev: &str) -> String {
    dev.strip_prefix("/dev/").unwrap_or(dev).to_string()
}

fn format_rate(bytes_per_sec: f64) -> String {
    if bytes_per_sec < 1024.0 {
        format!("{:.0} B/s", bytes_per_sec)
    } else if bytes_per_sec < 1024.0 * 1024.0 {
        format!("{:.1} K/s", bytes_per_sec / 1024.0)
    } else if bytes_per_sec < 1024.0 * 1024.0 * 1024.0 {
        format!("{:.1} M/s", bytes_per_sec / (1024.0 * 1024.0))
    } else {
        format!("{:.1} G/s", bytes_per_sec / (1024.0 * 1024.0 * 1024.0))
    }
}

fn format_mb(mb: u64) -> String {
    if mb < 1024 {
        format!("{} MB", mb)
    } else if mb < 1024 * 1024 {
        format!("{:.1} GB", mb as f64 / 1024.0)
    } else {
        format!("{:.1} TB", mb as f64 / (1024.0 * 1024.0))
    }
}
