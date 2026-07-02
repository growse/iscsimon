use crate::iscsi::{Collector, Session};
use crate::net::{collect_tcp_connections, resolve_hostname};
use ratatui::widgets::TableState;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

pub const NUM_COLS: usize = 7;

pub struct App {
    pub sessions: Vec<Session>,
    pub table_state: TableState,
    pub show_help: bool,
    pub error: Option<String>,
    pub source_ips: Vec<String>,
    pub initiator_hostname: Option<String>,
    pub selected_col: usize,
    pub sort_col: usize,
    pub sort_asc: bool,
    pub filter: String,
    pub filter_mode: bool,
    filter_before_edit: String,
    all_sessions: Vec<Session>,
    collector: Collector,
    hostname_cache: HashMap<String, String>,
}

impl App {
    pub fn new() -> Self {
        Self {
            sessions: Vec::new(),
            table_state: TableState::default(),
            show_help: false,
            error: None,
            source_ips: Vec::new(),
            initiator_hostname: None,
            selected_col: 0,
            sort_col: 0,
            sort_asc: true,
            filter: String::new(),
            filter_mode: false,
            filter_before_edit: String::new(),
            all_sessions: Vec::new(),
            collector: Collector::new(),
            hostname_cache: HashMap::new(),
        }
    }

    pub fn refresh(&mut self) {
        match self.collector.collect() {
            Ok(sessions) => {
                self.all_sessions = sessions;
                self.error = None;
            }
            Err(e) => {
                self.error = Some(e.to_string());
            }
        }

        if let Ok(conns) = collect_tcp_connections() {
            let ips: HashSet<String> = conns.iter().map(|c| c.peer_addr.clone()).collect();
            let mut v: Vec<String> = ips.into_iter().collect();
            v.sort();
            self.source_ips = v;
        }

        // Resolve hostnames for new IPs (cached)
        for ip in &self.source_ips {
            if !self.hostname_cache.contains_key(ip)
                && let Some(host) = resolve_hostname(ip)
            {
                self.hostname_cache.insert(ip.clone(), host);
            }
        }

        // Single resolved hostname → use for Initiator column; multiple → fall back to IQN
        let hostnames: HashSet<&str> = self
            .source_ips
            .iter()
            .filter_map(|ip| self.hostname_cache.get(ip).map(String::as_str))
            .collect();
        self.initiator_hostname = if hostnames.len() == 1 {
            hostnames.into_iter().next().map(str::to_string)
        } else {
            None
        };

        self.apply_filter();
        self.do_sort();
    }

    fn apply_filter(&mut self) {
        if self.filter.is_empty() {
            self.sessions = self.all_sessions.clone();
        } else {
            let needle = self.filter.to_lowercase();
            self.sessions = self
                .all_sessions
                .iter()
                .filter(|s| session_matches(s, &needle))
                .cloned()
                .collect();
        }

        let len = self.sessions.len();
        if let Some(sel) = self.table_state.selected() {
            if len == 0 {
                self.table_state.select(None);
            } else if sel >= len {
                self.table_state.select(Some(len - 1));
            }
        } else if len > 0 {
            self.table_state.select(Some(0));
        }
    }

    pub fn enter_filter_mode(&mut self) {
        self.filter_before_edit = self.filter.clone();
        self.filter_mode = true;
    }

    pub fn confirm_filter(&mut self) {
        self.filter_mode = false;
    }

    pub fn cancel_filter(&mut self) {
        self.filter_mode = false;
        self.filter = self.filter_before_edit.clone();
        self.apply_filter();
        self.do_sort();
    }

    pub fn filter_push_char(&mut self, c: char) {
        self.filter.push(c);
        self.apply_filter();
        self.do_sort();
    }

    pub fn filter_backspace(&mut self) {
        self.filter.pop();
        self.apply_filter();
        self.do_sort();
    }

    pub fn set_sort(&mut self, col: usize) {
        if self.sort_col == col {
            self.sort_asc = !self.sort_asc;
        } else {
            self.sort_col = col;
            self.sort_asc = true;
        }
        self.do_sort();
    }

    pub fn col_next(&mut self) {
        self.selected_col = (self.selected_col + 1).min(NUM_COLS - 1);
    }

    pub fn col_prev(&mut self) {
        self.selected_col = self.selected_col.saturating_sub(1);
    }

    fn do_sort(&mut self) {
        let col = self.sort_col;
        let asc = self.sort_asc;
        self.sessions.sort_by(|a, b| {
            let ord = match col {
                0 => a.target_iqn.cmp(&b.target_iqn),
                1 => a.initiator_iqn.cmp(&b.initiator_iqn),
                2 => {
                    let da = a.luns.first().map(|l| l.device.as_str()).unwrap_or("");
                    let db = b.luns.first().map(|l| l.device.as_str()).unwrap_or("");
                    da.cmp(db)
                }
                3 => a
                    .read_rate
                    .partial_cmp(&b.read_rate)
                    .unwrap_or(Ordering::Equal),
                4 => a
                    .write_rate
                    .partial_cmp(&b.write_rate)
                    .unwrap_or(Ordering::Equal),
                5 => a.total_read_mb.cmp(&b.total_read_mb),
                6 => a.total_write_mb.cmp(&b.total_write_mb),
                _ => Ordering::Equal,
            };
            if asc { ord } else { ord.reverse() }
        });
    }

    pub fn select_next(&mut self) {
        let len = self.sessions.len();
        if len == 0 {
            return;
        }
        let next = self
            .table_state
            .selected()
            .map(|i| (i + 1).min(len - 1))
            .unwrap_or(0);
        self.table_state.select(Some(next));
    }

    pub fn select_prev(&mut self) {
        let len = self.sessions.len();
        if len == 0 {
            return;
        }
        let prev = self
            .table_state
            .selected()
            .map(|i| i.saturating_sub(1))
            .unwrap_or(0);
        self.table_state.select(Some(prev));
    }

    pub fn select_first(&mut self) {
        if !self.sessions.is_empty() {
            self.table_state.select(Some(0));
        }
    }

    pub fn select_last(&mut self) {
        let len = self.sessions.len();
        if len > 0 {
            self.table_state.select(Some(len - 1));
        }
    }

    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }
}

/// Case-insensitive substring match against target IQN, initiator IQN, and device paths.
fn session_matches(session: &Session, needle: &str) -> bool {
    session.target_iqn.to_lowercase().contains(needle)
        || session.initiator_iqn.to_lowercase().contains(needle)
        || session
            .luns
            .iter()
            .any(|l| l.device.to_lowercase().contains(needle))
}
