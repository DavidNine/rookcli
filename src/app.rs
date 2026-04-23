use crate::k8s::{CephInfo, CephPoolInfo, PodInfo};
use ratatui::widgets::TableState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Clusters,
    Pools,
    Pods,
    Logs,
    Describe,
}

pub const ALL_TABS: [Tab; 5] = [
    Tab::Clusters,
    Tab::Pools,
    Tab::Pods,
    Tab::Logs,
    Tab::Describe,
];

impl Tab {
    pub fn all() -> &'static [Tab] {
        &ALL_TABS
    }

    pub fn to_index(&self) -> usize {
        match self {
            Tab::Clusters => 0,
            Tab::Pools => 1,
            Tab::Pods => 2,
            Tab::Logs => 3,
            Tab::Describe => 4,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Modal {
    None,
    ConfirmRestart(String),
    ConfirmDeletePool(String),
    ConfirmDeletePod(String),
}

pub struct App {
    pub is_running: bool,
    pub active_tab: Tab,
    pub clusters: Vec<CephInfo>,
    pub pools: Vec<CephPoolInfo>,
    pub pods: Vec<PodInfo>,
    pub error_message: Option<String>,
    pub active_modal: Modal,
    pub selected_pod: Option<String>,
    pub logs: Vec<String>,
    pub describe_content: String,
    pub describe_line_count: u16,
    
    pub cluster_state: TableState,
    pub pool_state: TableState,
    pub pod_state: TableState,
    pub log_scroll: u16,
    pub describe_scroll: u16,
}

impl App {
    pub fn new() -> Self {
        let mut cluster_state = TableState::default();
        cluster_state.select(Some(0));
        let mut pool_state = TableState::default();
        pool_state.select(Some(0));
        let mut pod_state = TableState::default();
        pod_state.select(Some(0));

        Self {
            is_running: true,
            active_tab: Tab::Clusters,
            clusters: Vec::new(),
            pools: Vec::new(),
            pods: Vec::new(),
            error_message: None,
            active_modal: Modal::None,
            selected_pod: None,
            logs: Vec::new(),
            describe_content: String::new(),
            describe_line_count: 0,
            cluster_state,
            pool_state,
            pod_state,
            log_scroll: 0,
            describe_scroll: 0,
        }
    }

    pub fn quit(&mut self) {
        self.is_running = false;
    }

    pub fn next_tab(&mut self) {
        if self.active_modal != Modal::None { return; }
        let tabs = Tab::all();
        let current_index = self.active_tab.to_index();
        let next_index = (current_index + 1) % tabs.len();
        self.active_tab = tabs[next_index];
    }

    pub fn prev_tab(&mut self) {
        if self.active_modal != Modal::None { return; }
        let tabs = Tab::all();
        let current_index = self.active_tab.to_index();
        let prev_index = if current_index == 0 {
            tabs.len() - 1
        } else {
            current_index - 1
        };
        self.active_tab = tabs[prev_index];
    }

    pub fn move_up(&mut self) {
        if self.active_modal != Modal::None { return; }
        match self.active_tab {
            Tab::Clusters => {
                let len = self.clusters.len();
                Self::move_state_up(&mut self.cluster_state, len);
            }
            Tab::Pools => {
                let len = self.pools.len();
                Self::move_state_up(&mut self.pool_state, len);
            }
            Tab::Pods => {
                let len = self.pods.len();
                Self::move_state_up(&mut self.pod_state, len);
            }
            Tab::Logs => {
                if self.log_scroll > 0 {
                    self.log_scroll -= 1;
                } else {
                    self.log_scroll = (self.logs.len() as u16).saturating_sub(1);
                }
            }
            Tab::Describe => {
                if self.describe_scroll > 0 {
                    self.describe_scroll -= 1;
                } else {
                    self.describe_scroll = self.describe_line_count.saturating_sub(1);
                }
            }
        }
    }

    fn move_state_up(state: &mut TableState, len: usize) {
        if len == 0 { return; }
        let i = match state.selected() {
            Some(i) => {
                if i == 0 {
                    len - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        state.select(Some(i));
    }

    pub fn move_down(&mut self) {
        if self.active_modal != Modal::None { return; }
        match self.active_tab {
            Tab::Clusters => {
                let len = self.clusters.len();
                Self::move_state_down(&mut self.cluster_state, len);
            }
            Tab::Pools => {
                let len = self.pools.len();
                Self::move_state_down(&mut self.pool_state, len);
            }
            Tab::Pods => {
                let len = self.pods.len();
                Self::move_state_down(&mut self.pod_state, len);
            }
            Tab::Logs => {
                if self.log_scroll < (self.logs.len() as u16).saturating_sub(1) {
                    self.log_scroll += 1;
                } else {
                    self.log_scroll = 0;
                }
            }
            Tab::Describe => {
                if self.describe_scroll < self.describe_line_count.saturating_sub(1) {
                    self.describe_scroll += 1;
                } else {
                    self.describe_scroll = 0;
                }
            }
        }
    }

    fn move_state_down(state: &mut TableState, len: usize) {
        if len == 0 { return; }
        let i = match state.selected() {
            Some(i) => {
                if i >= len - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        state.select(Some(i));
    }

    pub fn page_up(&mut self) {
        for _ in 0..10 { self.move_up(); }
    }

    pub fn page_down(&mut self) {
        for _ in 0..10 { self.move_down(); }
    }

    pub fn get_selected_index(&self) -> usize {
        match self.active_tab {
            Tab::Clusters => self.cluster_state.selected().unwrap_or(0),
            Tab::Pools => self.pool_state.selected().unwrap_or(0),
            Tab::Pods => self.pod_state.selected().unwrap_or(0),
            Tab::Logs | Tab::Describe => 0,
        }
    }

    pub fn set_describe_content(&mut self, content: String) {
        self.describe_line_count = content.lines().count() as u16;
        self.describe_content = content;
    }
}
