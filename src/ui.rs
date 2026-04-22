use ratatui::{
    Frame, 
    layout::{Alignment, Constraint, Direction, Layout, Rect}, 
    style::{Color, Modifier, Style}, 
    text::{Line, Span}, 
    widgets::{Block, Borders, Paragraph, Tabs, Table, Row, Cell, Clear}
};
use crate::app::{App, Tab, Modal};

pub fn render(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header/Tabs
            Constraint::Min(0),    // Main Content
            Constraint::Length(3), // Status Bar
        ].as_ref())
        .split(f.area());

    render_header(f, app, chunks[0]);
    render_content(f, app, chunks[1]);
    render_footer(f, app, chunks[2]);
    
    // Modals are rendered last on top of everything
    render_modal(f, app);
}

fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let title = " Rook-Ceph TUI ";
    let titles = Tab::all().iter().map(|t| format!("{:?}", t)).collect::<Vec<_>>();
    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title(title))
        .select(app.active_tab.to_index())
        .style(Style::default().fg(Color::Cyan))
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
    
    f.render_widget(tabs, area);
}

fn render_content(f: &mut Frame, app: &mut App, area: Rect) {
    match app.active_tab {
        Tab::Clusters => render_clusters_table(f, app, area),
        Tab::Pools => render_pools_table(f, app, area),
        Tab::Pods => render_pods_table(f, app, area),
        Tab::Logs => render_logs(f, app, area),
        Tab::Describe => render_describe(f, app, area),
    }
}

fn render_logs(f: &mut Frame, app: &App, area: Rect) {
    let title = if let Some(pod_name) = &app.selected_pod {
        format!(" Logs for: {} ", pod_name)
    } else {
        " Logs (No pod selected) ".to_string()
    };

    let log_content = if app.logs.is_empty() {
        "No logs available. Select a pod and press 'l' to fetch logs.".to_string()
    } else {
        app.logs.join("\n")
    };

    let paragraph = Paragraph::new(log_content)
        .block(Block::default().borders(Borders::ALL).title(title))
        .scroll((app.log_scroll, 0));
        
    f.render_widget(paragraph, area);
}

fn render_describe(f: &mut Frame, app: &App, area: Rect) {
    let title = if let Some(pod_name) = &app.selected_pod {
        format!(" Describe: {} ", pod_name)
    } else {
        " Describe (No pod selected) ".to_string()
    };

    let paragraph = Paragraph::new(app.describe_content.clone())
        .block(Block::default().borders(Borders::ALL).title(title))
        .scroll((app.describe_scroll, 0));
        
    f.render_widget(paragraph, area);
}

fn render_modal(f: &mut Frame, app: &App) {
    match &app.active_modal {
        Modal::ConfirmRestart(pod_name) => {
            render_confirmation_dialog(f, "Restart Pod", &format!("Are you sure you want to restart pod {}?", pod_name));
        }
        Modal::ConfirmDeletePool(pool_name) => {
            render_confirmation_dialog(f, "Delete Pool", &format!("Are you sure you want to delete pool {}?", pool_name));
        }
        Modal::ConfirmDeletePod(pod_name) => {
            render_confirmation_dialog(f, "Delete Pod", &format!("Are you sure you want to delete pod {}?", pod_name));
        }
        Modal::None => {}
    }
}

fn render_confirmation_dialog(f: &mut Frame, title: &str, message: &str) {
    let block = Block::default()
        .title(format!(" {} ", title))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));
        
    let area = centered_rect(60, 20, f.area());
    f.render_widget(Clear, area);
    
    let text = vec![
        Line::from(message),
        Line::from(""),
        Line::from(vec![
            Span::raw("Press "),
            Span::styled("Enter", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw(" to confirm or "),
            Span::styled("Esc", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::raw(" to cancel."),
        ]),
    ];
    
    let paragraph = Paragraph::new(text)
        .block(block)
        .alignment(Alignment::Center);
        
    f.render_widget(paragraph, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ].as_ref())
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ].as_ref())
        .split(popup_layout[1])[1]
}

fn render_clusters_table(f: &mut Frame, app: &mut App, area: Rect) {
    let running_pods = app.pods.iter().filter(|p| p.status == "Running").count();
    let total_pods = app.pods.len();

    let header_cells = ["Cluster Name", "Health Status", "Running Pods"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(Color::Cyan)));
    let header = Row::new(header_cells)
        .style(Style::default().bg(Color::Blue))
        .height(1);

    let rows = app.clusters.iter().map(|c| {
        let health_color = match c.health.as_str() {
            "HEALTH_OK" | "OK" => Color::Green,
            "HEALTH_WARN" | "WARN" => Color::Yellow,
            "HEALTH_ERR" | "ERR" | "ERROR" => Color::Red,
            _ => Color::Gray,
        };
        
        Row::new(vec![
            Cell::from(c.name.clone()),
            Cell::from(c.health.clone()).style(Style::default().fg(health_color)),
            Cell::from(format!("{}/{}", running_pods, total_pods)),
        ])
    });

    let table = Table::new(rows, [
        Constraint::Percentage(40),
        Constraint::Percentage(30),
        Constraint::Percentage(30),
    ])
    .header(header)
    .block(Block::default().borders(Borders::ALL).title(" Ceph Clusters "))
    .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    f.render_stateful_widget(table, area, &mut app.cluster_state);
}

fn render_pools_table(f: &mut Frame, app: &mut App, area: Rect) {
    let header_cells = ["Pool Name", "Status", "Size"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(Color::Cyan)));
    let header = Row::new(header_cells)
        .style(Style::default().bg(Color::Blue))
        .height(1);

    let rows = app.pools.iter().map(|p| {
        let status_color = match p.status.as_str() {
            "Ready" | "Created" => Color::Green,
            "Progressing" => Color::Yellow,
            "Error" | "Failure" => Color::Red,
            _ => Color::Gray,
        };

        Row::new(vec![
            Cell::from(p.name.clone()),
            Cell::from(p.status.clone()).style(Style::default().fg(status_color)),
            Cell::from(p.size.to_string()),
        ])
    });

    let table = Table::new(rows, [
        Constraint::Percentage(40),
        Constraint::Percentage(40),
        Constraint::Percentage(20),
    ])
    .header(header)
    .block(Block::default().borders(Borders::ALL).title(" Ceph Block Pools "))
    .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    f.render_stateful_widget(table, area, &mut app.pool_state);
}

fn render_pods_table(f: &mut Frame, app: &mut App, area: Rect) {
    let header_cells = ["Pod Name", "Ready", "Status", "Restarts", "Node"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(Color::Cyan)));
    let header = Row::new(header_cells)
        .style(Style::default().bg(Color::Blue))
        .height(1);

    let rows = app.pods.iter().map(|p| {
        let status_color = match p.status.as_str() {
            "Running" | "Succeeded" => Color::Green,
            "Pending" => Color::Yellow,
            "Failed" | "Unknown" => Color::Red,
            _ => Color::Gray,
        };

        Row::new(vec![
            Cell::from(p.name.clone()),
            Cell::from(p.ready.clone()),
            Cell::from(p.status.clone()).style(Style::default().fg(status_color)),
            Cell::from(p.restarts.to_string()),
            Cell::from(p.node.clone()),
        ])
    });

    let table = Table::new(rows, [
        Constraint::Percentage(35),
        Constraint::Percentage(10),
        Constraint::Percentage(15),
        Constraint::Percentage(10),
        Constraint::Percentage(30),
    ])
    .header(header)
    .block(Block::default().borders(Borders::ALL).title(" Rook-Ceph Pods "))
    .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    f.render_stateful_widget(table, area, &mut app.pod_state);
}

fn render_footer(f: &mut Frame, app: &App, area: Rect) {
    let help_text = match app.active_tab {
        Tab::Pods => " [Tab/←/→] Next/Prev | [PgUp/Dn] Jump | [r] Restart | [x] Delete | [l] Logs | [d] Describe ",
        Tab::Pools => " [Tab/←/→] Next/Prev | [Up/Dn] Navigate | [d] Delete Pool | [q] Quit ",
        Tab::Describe | Tab::Logs => " [Tab/←/→] Next/Prev | [Up/Dn/PgUp/Dn] Scroll | [q] Quit ",
        _ => " [Tab/←/→] Next/Prev | [Up/Dn] Navigate | [q] Quit ",
    };

    let status_text = if let Some(err) = &app.error_message {
        format!(" ERROR: {} ", err)
    } else {
        help_text.to_string()
    };

    let status_color = if app.error_message.is_some() { Color::Red } else { Color::Blue };

    let paragraph = Paragraph::new(status_text)
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(status_color)))
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Left);

    f.render_widget(paragraph, area);
}