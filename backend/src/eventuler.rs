use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Duration, Utc};
use daggy::{Dag, NodeIndex, Walker, WouldCycle};
use serde::{Deserialize, Serialize};
use thiserror::Error;

type Seconds = i64;

#[derive(Serialize, Deserialize)]
struct Node {
    name: String,
    color: String,
}

#[derive(Serialize, Deserialize)]
struct Link {
    source: usize,
    target: usize,
    value: i32,
    color: String,
}

#[derive(Serialize, Deserialize)]
pub struct Graph {
    nodes: Vec<Node>,
    links: Vec<Link>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    name: String,
    dependencies: Vec<String>,
    frequency: Seconds,
    current: DateTime<Utc>,
}

impl Job {
    fn updated(&self, now: &DateTime<Utc>) -> bool {
        let time_passed = now.signed_duration_since(self.current);
        let frequency = Duration::seconds(self.frequency);
        time_passed < frequency
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Task {
    name: String,
    current: DateTime<Utc>,
    target: DateTime<Utc>,
}

impl Task {
    pub async fn run(&self) {
    }
}

#[derive(Default)]
pub struct Eventuler {
    dag: daggy::Dag<Job, u32, u32>,
    name_to_idx: HashMap<String, NodeIndex<u32>>,
}

#[derive(Error, Debug)]
pub enum EventulerError {
    #[error("there is cycle")]
    WouldCycle(#[from] WouldCycle<u32>),

    #[error("node `{0}` not found")]
    NodeNotFound(String),
}

impl Eventuler {
    pub fn new() -> Self {
        Self {
            dag: Dag::new(),
            name_to_idx: HashMap::new(),
        }
    }

    pub fn insert(&mut self, job: Job) -> Result<(), EventulerError> {
        let job_idx = self.dag.add_node(job.clone());

        for name in job.dependencies.into_iter() {
            match self.name_to_idx.get(&name) {
                Some(dep_idx) => self.dag.update_edge(*dep_idx, job_idx, 0)?,
                None => return Err(EventulerError::NodeNotFound(name)),
            };
        }

        self.name_to_idx.entry(job.name).or_insert(job_idx);

        Ok(())
    }

    pub fn trigger(
        &mut self,
        name: &str,
        updatetime: DateTime<Utc>,
    ) -> Result<HashSet<Task>, EventulerError> {
        match self.name_to_idx.get(name) {
            Some(idx) => {
                self.dag[*idx].current = updatetime;
                Ok(self
                    .dag
                    .children(*idx)
                    .iter(&self.dag)
                    .filter_map(|(_, n)| self.task_if_ready(&n))
                    .collect())
            }
            None => Err(EventulerError::NodeNotFound(name.to_string())),
        }
    }

    fn task_if_ready(&self, n: &NodeIndex<u32>) -> Option<Task> {
        let job = &self.dag[*n];

        let parents = || self.dag.parents(*n).iter(&self.dag);

        let parents_min_current = parents().map(|(_, n)| self.dag[n].current).min()?;

        let time_diff = parents_min_current - job.current;
        let n_frequencies = time_diff.num_seconds() / job.frequency;
        let target = job.current + Duration::seconds(job.frequency * n_frequencies);

        if job.updated(&target) {
            return None;
        }

        let all_parents_updated = parents().all(|(_, n)| self.dag[n].updated(&target));

        if !all_parents_updated {
            return None;
        }

        let task = Task {
            name: job.name.clone(),
            current: job.current,
            target,
        };

        Some(task)
    }

    pub fn graph(&self) -> Graph {
        let now = Utc::now();
        let nodes = self
            .dag
            .raw_nodes()
            .iter()
            .map(|n| {
                let color = match n.weight.updated(&now) {
                    true => "green",
                    false => "red",
                };
                Node {
                    name: n.weight.name.clone(),
                    color: color.to_string(),
                }
            })
            .collect();
        let links = self
            .dag
            .raw_edges()
            .iter()
            .map(|e| {
                let color = match self.dag[e.source()].updated(&now) {
                    true => "green",
                    false => "red",
                };
                Link {
                    source: e.source().index(),
                    target: e.target().index(),
                    value: 1,
                    color: color.to_string(),
                }
            })
            .collect();
        Graph { nodes, links }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trigger() {
        let mut eventuler = Eventuler::new();
        let frequency = 10;
        let target = Utc::now();
        let current = target - Duration::seconds(frequency);

        let first_j = Job {
            name: "1".to_string(),
            dependencies: Vec::new(),
            frequency,
            current,
        };

        let second_j = Job {
            name: "2".to_string(),
            dependencies: vec!["1".to_string()],
            frequency,
            current,
        };

        eventuler.insert(first_j).unwrap();
        eventuler.insert(second_j).unwrap();

        let expected_task = Task {
            name: "2".to_string(),
            current,
            target,
        };

        let mut expected = HashSet::new();
        expected.insert(expected_task);

        assert_eq!(eventuler.trigger("1", target).unwrap(), expected)
    }
}
