use std::fs;
use std::path::{Path, PathBuf};

use cockpit_domain::Task;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum NotesError {
    #[error("failed to read notes file {path}: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
}

#[derive(Debug, Default)]
pub struct MarkdownTasksProvider;

impl MarkdownTasksProvider {
    pub fn load_pending_top3(path: &Path) -> Result<Vec<Task>, NotesError> {
        let content = fs::read_to_string(path).map_err(|source| NotesError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        Ok(top_pending_tasks(&parse_markdown_tasks(&content), 3))
    }
}

pub fn parse_markdown_tasks(input: &str) -> Vec<Task> {
    input.lines().filter_map(parse_checkbox_line).collect()
}

pub fn top_pending_tasks(tasks: &[Task], limit: usize) -> Vec<Task> {
    tasks
        .iter()
        .filter(|task| !task.completed)
        .take(limit)
        .cloned()
        .collect()
}

fn parse_checkbox_line(line: &str) -> Option<Task> {
    let trimmed = line.trim_start();
    let (completed, title) = if let Some(title) = trimmed.strip_prefix("- [ ] ") {
        (false, title)
    } else if let Some(title) = trimmed.strip_prefix("- [x] ") {
        (true, title)
    } else if let Some(title) = trimmed.strip_prefix("- [X] ") {
        (true, title)
    } else {
        return None;
    };

    Some(Task {
        title: title.trim().to_string(),
        completed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pending_tasks() {
        let tasks = parse_markdown_tasks("- [ ] Preparar aula Rathna");

        assert_eq!(
            tasks,
            vec![Task {
                title: "Preparar aula Rathna".to_string(),
                completed: false
            }]
        );
    }

    #[test]
    fn parses_completed_tasks() {
        let tasks = parse_markdown_tasks("- [x] Revisar README Marc");

        assert_eq!(
            tasks,
            vec![Task {
                title: "Revisar README Marc".to_string(),
                completed: true
            }]
        );
    }

    #[test]
    fn ignores_common_lines() {
        let tasks = parse_markdown_tasks("# Today\nplain text\n- item");

        assert!(tasks.is_empty());
    }

    #[test]
    fn limits_pending_tasks() {
        let tasks =
            parse_markdown_tasks("- [ ] One\n- [ ] Two\n- [x] Done\n- [ ] Three\n- [ ] Four");

        let top = top_pending_tasks(&tasks, 3);

        assert_eq!(top.len(), 3);
        assert_eq!(top[2].title, "Three");
    }
}
