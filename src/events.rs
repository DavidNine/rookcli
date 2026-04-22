use crossterm::event;
use event::{Event, KeyCode, KeyModifiers};
use std::time::Duration;
use tokio::sync::mpsc;

use crate::app::{App, Tab, Modal};
use crate::Action;

pub fn handle_events(app: &mut App, action_tx: &mpsc::Sender<Action>) -> Result<bool, Box<dyn std::error::Error>> {
    if event::poll(Duration::from_millis(10))? {
        if let Event::Key(key) = event::read()? {
            
            // Clear error message on any key press so it doesn't "stick"
            app.error_message = None;

            // Handle Modal inputs first
            if app.active_modal != Modal::None {
                match key.code {
                    KeyCode::Enter => {
                        match &app.active_modal {
                            Modal::ConfirmRestart(pod_name) => {
                                let _ = action_tx.try_send(Action::RestartPod(pod_name.clone()));
                            }
                            Modal::ConfirmDeletePool(pool_name) => {
                                let _ = action_tx.try_send(Action::DeletePool(pool_name.clone()));
                            }
                            Modal::ConfirmDeletePod(pod_name) => {
                                let _ = action_tx.try_send(Action::DeletePod(pod_name.clone()));
                            }
                            _ => {}
                        }
                        app.active_modal = Modal::None;
                    }
                    KeyCode::Esc => {
                        app.active_modal = Modal::None;
                    }
                    _ => {}
                }
                return Ok(false);
            }

            match key.code {
                KeyCode::Char('q') => {
                    app.quit();
                    return Ok(true)
                }
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    app.quit();
                    return Ok(true);
                },
                
                // Tab switching
                KeyCode::Tab | KeyCode::Right => app.next_tab(),
                KeyCode::BackTab | KeyCode::Left => app.prev_tab(),
                
                // Navigation
                KeyCode::Up => app.move_up(),
                KeyCode::Down => app.move_down(),
                KeyCode::PageUp => app.page_up(),
                KeyCode::PageDown => app.page_down(),

                // Actions
                KeyCode::Char('r') => {
                    if app.active_tab == Tab::Pods && !app.pods.is_empty() {
                        let idx = app.get_selected_index();
                        if let Some(pod) = app.pods.get(idx) {
                            app.active_modal = Modal::ConfirmRestart(pod.name.clone());
                        }
                    }
                }
                KeyCode::Char('x') => {
                    if app.active_tab == Tab::Pods && !app.pods.is_empty() {
                        let idx = app.get_selected_index();
                        if let Some(pod) = app.pods.get(idx) {
                            app.active_modal = Modal::ConfirmDeletePod(pod.name.clone());
                        }
                    }
                }
                KeyCode::Char('d') => {
                    if app.active_tab == Tab::Pools && !app.pools.is_empty() {
                        let idx = app.get_selected_index();
                        if let Some(pool) = app.pools.get(idx) {
                            app.active_modal = Modal::ConfirmDeletePool(pool.name.clone());
                        }
                    } else if app.active_tab == Tab::Pods && !app.pods.is_empty() {
                        let idx = app.get_selected_index();
                        if let Some(pod) = app.pods.get(idx) {
                            app.selected_pod = Some(pod.name.clone());
                            app.describe_content = "Fetching pod details...".to_string();
                            app.describe_scroll = 0;
                            let _ = action_tx.try_send(Action::DescribePod(pod.name.clone()));
                            app.active_tab = Tab::Describe;
                        }
                    }
                }
                KeyCode::Char('l') => {
                    if app.active_tab == Tab::Pods && !app.pods.is_empty() {
                        let idx = app.get_selected_index();
                        if let Some(pod) = app.pods.get(idx) {
                            app.selected_pod = Some(pod.name.clone());
                            app.logs.clear();
                            app.log_scroll = 0;
                            let _ = action_tx.try_send(Action::FetchLogs(pod.name.clone(), None));
                            app.active_tab = Tab::Logs;
                        }
                    }
                }
                
                _ => {}
            }
        }
    }
    
    return Ok(false)
}