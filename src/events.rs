use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc;

use crate::app::{App, Tab, Modal};
use crate::Action;

pub fn read_terminal_event() -> Result<Event, Box<dyn std::error::Error + Send + Sync>> {
    Ok(event::read()?)
}

pub fn handle_terminal_event(
    app: &mut App,
    action_tx: &mpsc::Sender<Action>,
    event: Event,
) -> Result<bool, Box<dyn std::error::Error>> {
    if let Event::Key(key) = event {
        return handle_key_event(app, action_tx, key);
    }

    Ok(false)
}

fn handle_key_event(
    app: &mut App,
    action_tx: &mpsc::Sender<Action>,
    key: KeyEvent,
) -> Result<bool, Box<dyn std::error::Error>> {
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
            Ok(true)
        }
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.quit();
            Ok(true)
        }

        // Tab switching
        KeyCode::Tab | KeyCode::Right => {
            app.next_tab();
            Ok(false)
        }
        KeyCode::BackTab | KeyCode::Left => {
            app.prev_tab();
            Ok(false)
        }

        // Navigation
        KeyCode::Up => {
            app.move_up();
            Ok(false)
        }
        KeyCode::Down => {
            app.move_down();
            Ok(false)
        }
        KeyCode::PageUp => {
            app.page_up();
            Ok(false)
        }
        KeyCode::PageDown => {
            app.page_down();
            Ok(false)
        }

        // Actions
        KeyCode::Char('r') => {
            if app.active_tab == Tab::Pods && !app.pods.is_empty() {
                let idx = app.get_selected_index();
                if let Some(pod) = app.pods.get(idx) {
                    app.active_modal = Modal::ConfirmRestart(pod.name.clone());
                }
            }
            Ok(false)
        }
        KeyCode::Char('x') => {
            if app.active_tab == Tab::Pods && !app.pods.is_empty() {
                let idx = app.get_selected_index();
                if let Some(pod) = app.pods.get(idx) {
                    app.active_modal = Modal::ConfirmDeletePod(pod.name.clone());
                }
            }
            Ok(false)
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
                    let pod_name = pod.name.clone();
                    app.selected_pod = Some(pod_name.clone());
                    app.set_describe_content("Fetching pod details...".to_string());
                    app.describe_scroll = 0;
                    let _ = action_tx.try_send(Action::DescribePod(pod_name));
                    app.active_tab = Tab::Describe;
                }
            }
            Ok(false)
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
            Ok(false)
        }

        _ => Ok(false),
    }
}
