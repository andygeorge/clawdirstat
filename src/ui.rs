use crate::scanner::Entry;

/// Format a byte count as a human-readable string.
pub fn format_size(bytes: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = 1024 * KIB;
    const GIB: u64 = 1024 * MIB;

    if bytes >= GIB {
        format!("{:.1} GiB", bytes as f64 / GIB as f64)
    } else if bytes >= MIB {
        format!("{:.1} MiB", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.1} KiB", bytes as f64 / KIB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Prepare display rows from scan entries, limited to `count` top-level entries.
pub fn prepare_rows(entries: &[Entry], count: Option<usize>) -> Vec<DisplayRow> {
    let iter: Box<dyn Iterator<Item = &Entry>> = match count {
        Some(n) => Box::new(entries.iter().take(n)),
        None => Box::new(entries.iter()),
    };

    iter.map(|e| DisplayRow {
        name: e.name.clone(),
        size: e.size,
        size_label: format_size(e.size),
        is_dir: e.is_dir,
    })
    .collect()
}

#[derive(Debug, Clone)]
pub struct DisplayRow {
    pub name: String,
    pub size: u64,
    pub size_label: String,
    pub is_dir: bool,
}

/// Compute a bar width (0..=bar_width) proportional to the entry's share of total size.
pub fn bar_fraction(entry_size: u64, total_size: u64, bar_width: u16) -> u16 {
    if total_size == 0 {
        return 0;
    }
    ((entry_size as f64 / total_size as f64) * bar_width as f64).round() as u16
}

pub mod app {
    use super::*;
    use crate::scanner;
    use anyhow::Result;
    use crossterm::{
        event::{self, Event, KeyCode},
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    };
    use ratatui::{
        Terminal,
        backend::CrosstermBackend,
        layout::{Constraint, Direction, Layout},
        style::{Color, Style},
        widgets::{Block, Borders, Gauge, List, ListItem, ListState},
    };
    use std::io;

    pub fn run(root: Entry, count: Option<usize>) -> Result<()> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Sort top-level children by size descending
        let mut top_entries = root.children.clone();
        scanner::sort_by_size(&mut top_entries);

        let rows = prepare_rows(&top_entries, count);
        let total_size: u64 = rows.iter().map(|r| r.size).sum();

        let mut list_state = ListState::default();
        if !rows.is_empty() {
            list_state.select(Some(0));
        }

        loop {
            terminal.draw(|f| {
                let area = f.area();
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(3), Constraint::Length(1)])
                    .split(area);

                // Build list items
                let items: Vec<ListItem> = rows
                    .iter()
                    .map(|r| {
                        let bar_w = bar_fraction(r.size, total_size, 20);
                        let bar = "█".repeat(bar_w as usize);
                        let prefix = if r.is_dir { "/" } else { " " };
                        let label = format!(
                            "{}{:<30} {:>10}  {}",
                            prefix, r.name, r.size_label, bar
                        );
                        ListItem::new(label)
                    })
                    .collect();

                let list = List::new(items)
                    .block(Block::default().borders(Borders::ALL).title("clawdirstat"))
                    .highlight_style(Style::default().fg(Color::Yellow));

                f.render_stateful_widget(list, chunks[0], &mut list_state);

                // Show selected entry's children breakdown if it's a dir
                if let Some(selected) = list_state.selected() {
                    if let Some(row) = rows.get(selected) {
                        if row.is_dir {
                            if let Some(entry) = top_entries.get(selected) {
                                let mut child_rows = entry.children.clone();
                                scanner::sort_by_size(&mut child_rows);
                                let child_total: u64 = child_rows.iter().map(|c| c.size).sum();

                                let child_area = if area.height > 20 {
                                    let splits = Layout::default()
                                        .direction(Direction::Vertical)
                                        .constraints([
                                            Constraint::Percentage(50),
                                            Constraint::Percentage(50),
                                        ])
                                        .split(area);
                                    Some(splits[1])
                                } else {
                                    None
                                };

                                if let Some(ca) = child_area {
                                    let child_items: Vec<ListItem> = child_rows
                                        .iter()
                                        .take(20)
                                        .map(|c| {
                                            let bar_w = bar_fraction(c.size, child_total, 20);
                                            let bar = "█".repeat(bar_w as usize);
                                            let prefix = if c.is_dir { "/" } else { " " };
                                            ListItem::new(format!(
                                                "{}{:<30} {:>10}  {}",
                                                prefix,
                                                c.name,
                                                format_size(c.size),
                                                bar
                                            ))
                                        })
                                        .collect();

                                    let child_list = List::new(child_items).block(
                                        Block::default()
                                            .borders(Borders::ALL)
                                            .title(format!("/{}", row.name)),
                                    );
                                    f.render_widget(child_list, ca);
                                }
                            }
                        }
                    }
                }

                // Progress bar showing selected entry's fraction
                if let Some(selected) = list_state.selected() {
                    if let Some(row) = rows.get(selected) {
                        let pct = if total_size > 0 {
                            (row.size as f64 / total_size as f64 * 100.0) as u16
                        } else {
                            0
                        };
                        let gauge = Gauge::default()
                            .block(Block::default())
                            .gauge_style(Style::default().fg(Color::Green))
                            .percent(pct);
                        f.render_widget(gauge, chunks[1]);
                    }
                }
            })?;

            if event::poll(std::time::Duration::from_millis(200))? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        KeyCode::Down | KeyCode::Char('j') => {
                            let next = list_state
                                .selected()
                                .map(|i| (i + 1).min(rows.len().saturating_sub(1)))
                                .unwrap_or(0);
                            list_state.select(Some(next));
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            let prev = list_state
                                .selected()
                                .map(|i| i.saturating_sub(1))
                                .unwrap_or(0);
                            list_state.select(Some(prev));
                        }
                        _ => {}
                    }
                }
            }
        }

        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scanner::Entry;
    use std::path::PathBuf;

    fn make_entry(name: &str, size: u64, is_dir: bool) -> Entry {
        Entry {
            path: PathBuf::from(name),
            name: name.to_string(),
            size,
            is_dir,
            children: vec![],
        }
    }

    #[test]
    fn test_format_size_bytes() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(512), "512 B");
    }

    #[test]
    fn test_format_size_kib() {
        assert_eq!(format_size(1024), "1.0 KiB");
        assert_eq!(format_size(2048), "2.0 KiB");
    }

    #[test]
    fn test_format_size_mib() {
        assert_eq!(format_size(1024 * 1024), "1.0 MiB");
    }

    #[test]
    fn test_format_size_gib() {
        assert_eq!(format_size(1024 * 1024 * 1024), "1.0 GiB");
    }

    #[test]
    fn test_prepare_rows_no_limit() {
        let entries = vec![
            make_entry("alpha", 1000, true),
            make_entry("beta", 500, false),
        ];
        let rows = prepare_rows(&entries, None);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].name, "alpha");
    }

    #[test]
    fn test_prepare_rows_with_limit() {
        let entries = vec![
            make_entry("a", 100, false),
            make_entry("b", 200, false),
            make_entry("c", 300, false),
        ];
        let rows = prepare_rows(&entries, Some(2));
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn test_prepare_rows_limit_larger_than_entries() {
        let entries = vec![make_entry("x", 10, false)];
        let rows = prepare_rows(&entries, Some(50));
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn test_bar_fraction_zero_total() {
        assert_eq!(bar_fraction(100, 0, 20), 0);
    }

    #[test]
    fn test_bar_fraction_half() {
        assert_eq!(bar_fraction(50, 100, 20), 10);
    }

    #[test]
    fn test_bar_fraction_full() {
        assert_eq!(bar_fraction(100, 100, 20), 20);
    }
}
