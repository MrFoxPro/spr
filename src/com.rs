use std::collections::HashMap;
use tokio::sync::mpsc;
use crate::{task::TaskCommand, EvCommand, TaskConfig};

pub struct ComBundle {
	pub ev_rx: mpsc::Receiver<EvCommand>,
	pub tasks_rx: HashMap<String, mpsc::Receiver<TaskCommand>>,
}

#[derive(Clone)]
pub struct Com {
	pub ev: mpsc::Sender<EvCommand>,
	pub tasks: HashMap<String, mpsc::Sender<TaskCommand>>,
}
impl Com {
	pub fn task(&self, id: &String) -> &mpsc::Sender<TaskCommand> {
		return self.tasks.get(id).unwrap()
	}
}

impl Com {
	pub fn init(entries: &Vec<TaskConfig>) -> (ComBundle, Com) {
		let (ev_tx, ev_rx) = mpsc::channel(24);
		let (tasks, tasks_rx) = {
			let (mut tx, mut rx) = (HashMap::with_capacity(entries.len()), HashMap::with_capacity(entries.len()));
			for task in entries.iter() {
				let (ttx, rrx) = mpsc::channel(12);
				tx.insert(task.id.clone(), ttx); rx.insert(task.id.clone(), rrx);
			}
			(tx, rx)
		};

		return (
			ComBundle {
				ev_rx: ev_rx,
				tasks_rx: tasks_rx,
			},
			Com {
				ev: ev_tx,
				tasks: tasks
			}
		)
	}
}
