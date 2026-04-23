mod k8s;
mod app;
mod ui;
mod events;

use app::App;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use kube::Client;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{error::Error, io};
use tokio::sync::mpsc;
use std::time::Duration;

pub enum Message {
    UpdateClusters(Vec<k8s::CephInfo>),
    UpdatePools(Vec<k8s::CephPoolInfo>),
    UpdatePods(Vec<k8s::PodInfo>),
    UpdateLogs(Vec<String>),
    UpdateDescribe(String),
    Error(String),
}

pub enum Action {
    RestartPod(String),
    DeletePod(String),
    FetchLogs(String, Option<String>),
    DescribePod(String),
    DeletePool(String),
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {

    enable_raw_mode()?;
    
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let res = run_app(&mut terminal, &mut app).await;
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Application error: {:?}", err);
    }

    Ok(())
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<(), Box<dyn std::error::Error>> {
    let (tx, mut rx) = mpsc::channel(100);
    let (action_tx, mut action_rx) = mpsc::channel(100);
    let (event_tx, mut event_rx) = mpsc::unbounded_channel();

    std::thread::spawn(move || {
        while let Ok(event) = events::read_terminal_event() {
            if event_tx.send(event).is_err() {
                break;
            }
        }
    });

    // Spawn background worker for K8s polling and actions
    tokio::spawn(async move {
        let client_res = Client::try_default().await;
        match client_res {
            Ok(client) => {
                let mut interval = tokio::time::interval(Duration::from_secs(10));
                loop {
                    tokio::select! {
                        _ = interval.tick() => {
                            let (clusters, pools, pods) = tokio::join!(
                                k8s::get_ceph_health(&client),
                                k8s::get_ceph_pools(&client),
                                k8s::get_pods(&client),
                            );

                            match clusters {
                                Ok(clusters) => { let _ = tx.send(Message::UpdateClusters(clusters)).await; }
                                Err(e) => { let _ = tx.send(Message::Error(format!("K8s clusters error: {}", e))).await; }
                            }
                            match pools {
                                Ok(pools) => { let _ = tx.send(Message::UpdatePools(pools)).await; }
                                Err(e) => { let _ = tx.send(Message::Error(format!("K8s pools error: {}", e))).await; }
                            }
                            match pods {
                                Ok(pods) => { let _ = tx.send(Message::UpdatePods(pods)).await; }
                                Err(e) => { let _ = tx.send(Message::Error(format!("K8s pods error: {}", e))).await; }
                            }
                        }
                        Some(action) = action_rx.recv() => {
                            match action {
                                Action::RestartPod(pod_name) => {
                                    match k8s::restart_pod(&client, &pod_name).await {
                                        Ok(_) => { 
                                            if let Ok(pods) = k8s::get_pods(&client).await {
                                                let _ = tx.send(Message::UpdatePods(pods)).await;
                                            }
                                        }
                                        Err(e) => { let _ = tx.send(Message::Error(format!("Restart error: {}", e))).await; }
                                    }
                                }
                                Action::DeletePod(pod_name) => {
                                    match k8s::delete_pod(&client, &pod_name).await {
                                        Ok(_) => {
                                            if let Ok(pods) = k8s::get_pods(&client).await {
                                                let _ = tx.send(Message::UpdatePods(pods)).await;
                                            }
                                        }
                                        Err(e) => { let _ = tx.send(Message::Error(format!("Delete pod error: {}", e))).await; }
                                    }
                                }
                                Action::FetchLogs(pod_name, container_name) => {
                                    match k8s::fetch_pod_logs(&client, &pod_name, container_name).await {
                                        Ok(logs) => { let _ = tx.send(Message::UpdateLogs(logs)).await; }
                                        Err(e) => { let _ = tx.send(Message::Error(format!("Logs error: {}", e))).await; }
                                    }
                                }
                                Action::DescribePod(pod_name) => {
                                    match k8s::describe_pod(&client, &pod_name).await {
                                        Ok(content) => { let _ = tx.send(Message::UpdateDescribe(content)).await; }
                                        Err(e) => { let _ = tx.send(Message::Error(format!("Describe error: {}", e))).await; }
                                    }
                                }
                                Action::DeletePool(pool_name) => {
                                    match k8s::delete_pool(&client, &pool_name).await {
                                        Ok(_) => {
                                            if let Ok(pools) = k8s::get_ceph_pools(&client).await {
                                                let _ = tx.send(Message::UpdatePools(pools)).await;
                                            }
                                        }
                                        Err(e) => { let _ = tx.send(Message::Error(format!("Delete pool error: {}", e))).await; }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                let _ = tx.send(Message::Error(format!("K8s client error: {}", e))).await;
            }
        }
    });

    terminal.draw(|f| {
        ui::render(f, app);
    })?;

    while app.is_running {
        tokio::select! {
            Some(msg) = rx.recv() => {
                match msg {
                    Message::UpdateClusters(clusters) => app.clusters = clusters,
                    Message::UpdatePools(pools) => app.pools = pools,
                    Message::UpdatePods(pods) => app.pods = pods,
                    Message::UpdateLogs(logs) => app.logs = logs,
                    Message::UpdateDescribe(content) => app.set_describe_content(content),
                    Message::Error(e) => app.error_message = Some(e),
                }
                while let Ok(msg) = rx.try_recv() {
                    match msg {
                        Message::UpdateClusters(clusters) => app.clusters = clusters,
                        Message::UpdatePools(pools) => app.pools = pools,
                        Message::UpdatePods(pods) => app.pods = pods,
                        Message::UpdateLogs(logs) => app.logs = logs,
                        Message::UpdateDescribe(content) => app.set_describe_content(content),
                        Message::Error(e) => app.error_message = Some(e),
                    }
                }
            }
            Some(event) = event_rx.recv() => {
                if events::handle_terminal_event(app, &action_tx, event)? {
                    break;
                }
            }
        }

        terminal.draw(|f| {
            ui::render(f, app);
        })?;
    }

    Ok(())
}
