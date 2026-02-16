//! Task management module
//!
//! Provides task graph construction, validation, and execution for the `stacy task` command.

pub mod executor;

use crate::error::{Error, Result};
use crate::project::config::{ScriptsSection, TaskDef};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// A graph of task definitions with validation
#[derive(Debug)]
pub struct TaskGraph {
    tasks: HashMap<String, TaskDef>,
}

impl TaskGraph {
    /// Create a new TaskGraph from a ScriptsSection
    pub fn from_config(scripts: &ScriptsSection) -> Result<Self> {
        let graph = Self {
            tasks: scripts.tasks.clone(),
        };

        // Validate on construction
        graph.validate_references()?;
        graph.validate_no_cycles()?;

        Ok(graph)
    }

    /// Check if a task exists
    pub fn has_task(&self, name: &str) -> bool {
        self.tasks.contains_key(name)
    }

    /// Get a task by name
    pub fn get_task(&self, name: &str) -> Option<&TaskDef> {
        self.tasks.get(name)
    }

    /// List all tasks with their definitions
    pub fn list_tasks(&self) -> Vec<(&str, &TaskDef)> {
        let mut tasks: Vec<_> = self.tasks.iter().map(|(k, v)| (k.as_str(), v)).collect();
        tasks.sort_by_key(|(name, _)| *name);
        tasks
    }

    /// Get the number of tasks
    pub fn len(&self) -> usize {
        self.tasks.len()
    }

    /// Check if the graph is empty
    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    /// Validate that all task references exist
    fn validate_references(&self) -> Result<()> {
        for (name, task) in &self.tasks {
            let refs = self.get_task_references(task);
            for ref_name in refs {
                if !self.tasks.contains_key(&ref_name) {
                    return Err(Error::Config(format!(
                        "Task '{}' references unknown task '{}'",
                        name, ref_name
                    )));
                }
            }
        }
        Ok(())
    }

    /// Validate that there are no cycles in the task graph using DFS
    fn validate_no_cycles(&self) -> Result<()> {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();

        for name in self.tasks.keys() {
            if !visited.contains(name) {
                if let Some(cycle) =
                    self.detect_cycle(name, &mut visited, &mut rec_stack, &mut path)
                {
                    return Err(Error::Config(format!(
                        "Circular dependency detected: {}",
                        cycle.join(" -> ")
                    )));
                }
            }
        }

        Ok(())
    }

    /// DFS-based cycle detection
    fn detect_cycle(
        &self,
        name: &str,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
    ) -> Option<Vec<String>> {
        visited.insert(name.to_string());
        rec_stack.insert(name.to_string());
        path.push(name.to_string());

        if let Some(task) = self.tasks.get(name) {
            for ref_name in self.get_task_references(task) {
                if !visited.contains(&ref_name) {
                    if let Some(cycle) = self.detect_cycle(&ref_name, visited, rec_stack, path) {
                        return Some(cycle);
                    }
                } else if rec_stack.contains(&ref_name) {
                    // Found a cycle - build the cycle path
                    let mut cycle_path = path.clone();
                    cycle_path.push(ref_name);
                    return Some(cycle_path);
                }
            }
        }

        path.pop();
        rec_stack.remove(name);
        None
    }

    /// Get all task names referenced by a task definition
    fn get_task_references(&self, task: &TaskDef) -> Vec<String> {
        match task {
            TaskDef::Simple(_) => vec![],
            TaskDef::Sequential(tasks) => tasks.clone(),
            TaskDef::Complex(complex) => {
                if let Some(ref parallel) = complex.parallel {
                    parallel.clone()
                } else {
                    vec![]
                }
            }
        }
    }

    /// Find similar task names for "did you mean" suggestions
    pub fn find_similar(&self, name: &str) -> Vec<&str> {
        let name_lower = name.to_lowercase();
        let mut similar: Vec<_> = self
            .tasks
            .keys()
            .filter(|k| {
                let k_lower = k.to_lowercase();
                // Simple heuristic: starts with same char, or contains the name, or edit distance is small
                k_lower.starts_with(&name_lower[..1.min(name_lower.len())])
                    || k_lower.contains(&name_lower)
                    || name_lower.contains(&k_lower)
                    || levenshtein_distance(&k_lower, &name_lower) <= 2
            })
            .map(|s| s.as_str())
            .collect();
        similar.sort();
        similar.truncate(3);
        similar
    }
}

/// Simple Levenshtein distance implementation
#[allow(clippy::needless_range_loop)]
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let m = a_chars.len();
    let n = b_chars.len();

    if m == 0 {
        return n;
    }
    if n == 0 {
        return m;
    }

    let mut matrix = vec![vec![0usize; n + 1]; m + 1];

    for i in 0..=m {
        matrix[i][0] = i;
    }
    for j in 0..=n {
        matrix[0][j] = j;
    }

    for i in 1..=m {
        for j in 1..=n {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            matrix[i][j] = (matrix[i - 1][j] + 1)
                .min(matrix[i][j - 1] + 1)
                .min(matrix[i - 1][j - 1] + cost);
        }
    }

    matrix[m][n]
}

/// Get a description for a task definition
pub fn task_description(task: &TaskDef) -> String {
    match task {
        TaskDef::Simple(path) => format!("Run {}", path.display()),
        TaskDef::Sequential(tasks) => format!("Run {} tasks sequentially", tasks.len()),
        TaskDef::Complex(complex) => {
            if let Some(ref desc) = complex.description {
                desc.clone()
            } else if let Some(ref parallel) = complex.parallel {
                format!("Run {} tasks in parallel", parallel.len())
            } else if let Some(ref script) = complex.script {
                format!("Run {}", script.display())
            } else {
                "Complex task".to_string()
            }
        }
    }
}

/// Get the script path for a task that runs a single script
pub fn task_script(task: &TaskDef) -> Option<&PathBuf> {
    match task {
        TaskDef::Simple(path) => Some(path),
        TaskDef::Sequential(_) => None,
        TaskDef::Complex(complex) => complex.script.as_ref(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::config::ComplexTask;

    fn make_scripts(tasks: Vec<(&str, TaskDef)>) -> ScriptsSection {
        ScriptsSection {
            tasks: tasks.into_iter().map(|(k, v)| (k.to_string(), v)).collect(),
        }
    }

    #[test]
    fn test_empty_graph() {
        let scripts = ScriptsSection::default();
        let graph = TaskGraph::from_config(&scripts).unwrap();
        assert!(graph.is_empty());
        assert_eq!(graph.len(), 0);
    }

    #[test]
    fn test_simple_tasks() {
        let scripts = make_scripts(vec![
            ("clean", TaskDef::Simple(PathBuf::from("src/01_clean.do"))),
            (
                "analyze",
                TaskDef::Simple(PathBuf::from("src/02_analyze.do")),
            ),
        ]);

        let graph = TaskGraph::from_config(&scripts).unwrap();
        assert_eq!(graph.len(), 2);
        assert!(graph.has_task("clean"));
        assert!(graph.has_task("analyze"));
        assert!(!graph.has_task("missing"));
    }

    #[test]
    fn test_sequential_tasks() {
        let scripts = make_scripts(vec![
            ("clean", TaskDef::Simple(PathBuf::from("src/01_clean.do"))),
            (
                "analyze",
                TaskDef::Simple(PathBuf::from("src/02_analyze.do")),
            ),
            (
                "all",
                TaskDef::Sequential(vec!["clean".to_string(), "analyze".to_string()]),
            ),
        ]);

        let graph = TaskGraph::from_config(&scripts).unwrap();
        assert_eq!(graph.len(), 3);
    }

    #[test]
    fn test_parallel_tasks() {
        let scripts = make_scripts(vec![
            ("tables", TaskDef::Simple(PathBuf::from("src/tables.do"))),
            ("figures", TaskDef::Simple(PathBuf::from("src/figures.do"))),
            (
                "outputs",
                TaskDef::Complex(ComplexTask {
                    parallel: Some(vec!["tables".to_string(), "figures".to_string()]),
                    script: None,
                    args: None,
                    description: None,
                }),
            ),
        ]);

        let graph = TaskGraph::from_config(&scripts).unwrap();
        assert_eq!(graph.len(), 3);
    }

    #[test]
    fn test_missing_reference() {
        let scripts = make_scripts(vec![(
            "all",
            TaskDef::Sequential(vec!["clean".to_string(), "analyze".to_string()]),
        )]);

        let result = TaskGraph::from_config(&scripts);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("references unknown task"));
    }

    #[test]
    fn test_direct_cycle() {
        let scripts = make_scripts(vec![
            ("a", TaskDef::Sequential(vec!["b".to_string()])),
            ("b", TaskDef::Sequential(vec!["a".to_string()])),
        ]);

        let result = TaskGraph::from_config(&scripts);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Circular dependency"));
    }

    #[test]
    fn test_indirect_cycle() {
        let scripts = make_scripts(vec![
            ("a", TaskDef::Sequential(vec!["b".to_string()])),
            ("b", TaskDef::Sequential(vec!["c".to_string()])),
            ("c", TaskDef::Sequential(vec!["a".to_string()])),
        ]);

        let result = TaskGraph::from_config(&scripts);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Circular dependency"));
    }

    #[test]
    fn test_self_reference_cycle() {
        let scripts = make_scripts(vec![(
            "loop",
            TaskDef::Sequential(vec!["loop".to_string()]),
        )]);

        let result = TaskGraph::from_config(&scripts);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Circular dependency"));
    }

    #[test]
    fn test_list_tasks_sorted() {
        let scripts = make_scripts(vec![
            ("zebra", TaskDef::Simple(PathBuf::from("src/zebra.do"))),
            ("alpha", TaskDef::Simple(PathBuf::from("src/alpha.do"))),
            ("beta", TaskDef::Simple(PathBuf::from("src/beta.do"))),
        ]);

        let graph = TaskGraph::from_config(&scripts).unwrap();
        let tasks = graph.list_tasks();
        assert_eq!(tasks[0].0, "alpha");
        assert_eq!(tasks[1].0, "beta");
        assert_eq!(tasks[2].0, "zebra");
    }

    #[test]
    fn test_find_similar() {
        let scripts = make_scripts(vec![
            ("clean", TaskDef::Simple(PathBuf::from("clean.do"))),
            ("analyze", TaskDef::Simple(PathBuf::from("analyze.do"))),
            ("cleanup", TaskDef::Simple(PathBuf::from("cleanup.do"))),
        ]);

        let graph = TaskGraph::from_config(&scripts).unwrap();
        let similar = graph.find_similar("clen");
        assert!(similar.contains(&"clean"));
    }

    #[test]
    fn test_task_description() {
        assert_eq!(
            task_description(&TaskDef::Simple(PathBuf::from("test.do"))),
            "Run test.do"
        );

        assert_eq!(
            task_description(&TaskDef::Sequential(vec!["a".to_string(), "b".to_string()])),
            "Run 2 tasks sequentially"
        );

        assert_eq!(
            task_description(&TaskDef::Complex(ComplexTask {
                parallel: Some(vec!["a".to_string(), "b".to_string()]),
                script: None,
                args: None,
                description: None,
            })),
            "Run 2 tasks in parallel"
        );

        assert_eq!(
            task_description(&TaskDef::Complex(ComplexTask {
                parallel: None,
                script: None,
                args: None,
                description: Some("My custom task".to_string()),
            })),
            "My custom task"
        );
    }

    #[test]
    fn test_levenshtein_distance() {
        assert_eq!(levenshtein_distance("", ""), 0);
        assert_eq!(levenshtein_distance("abc", ""), 3);
        assert_eq!(levenshtein_distance("", "abc"), 3);
        assert_eq!(levenshtein_distance("abc", "abc"), 0);
        assert_eq!(levenshtein_distance("abc", "abd"), 1);
        assert_eq!(levenshtein_distance("clean", "clen"), 1);
        assert_eq!(levenshtein_distance("analyze", "analyis"), 2);
    }
}
