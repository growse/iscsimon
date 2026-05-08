use crate::iscsi::{Collector, Session};
use crate::net::collect_tcp_connections;
use ratatui::widgets::TableState;
use std::collections::HashSet;

pub struct App {
    pub sessions: Vec<Session>,
    pub table_state: TableState,
    pub show_help: bool,
    pub error: Option<String>,
    pub source_ips: Vec<String>,
    collector: Collector,
}

impl App {
    pub fn new() -> Self {
        Self {
            sessions: Vec::new(),
            table_state: TableState::default(),
            show_help: false,
            error: None,
            source_ips: Vec::new(),
            collector: Collector::new(),
        }
    }

    pub fn refresh(&mut self) {
        match self.collector.collect() {
            Ok(sessions) => {
                self.sessions = sessions;
                self.error = None;
                // Clamp selection
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
            Err(e) => {
                self.error = Some(e.to_string());
            }
        }

        match collect_tcp_connections() {
            Ok(conns) => {
                let ips: HashSet<String> =
                    conns.iter().map(|c| c.peer_addr.clone()).collect();
                let mut v: Vec<String> = ips.into_iter().collect();
                v.sort();
                self.source_ips = v;
            }
            Err(_) => {}
        }
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
