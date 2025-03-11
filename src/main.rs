#![allow(unused)]

mod com;
mod common;
mod obs;
mod cli;
mod task;
mod vsock;

use std::{io::{BufRead, Write}, sync::OnceLock, thread};
use bincode::{Decode, Encode};
use cli::style;
use task::task_manager;
use tokio::{signal, sync::mpsc, task::JoinSet};
use yansi::Paint;

#[derive(Clone)]
pub struct Context {
	com: com::Com,
	cfg: Config,
}
pub static CONTEXT: OnceLock<Context> = OnceLock::new();
pub fn context() -> &'static Context {
	if let Some(ctx) = CONTEXT.get() { return ctx }
	unreachable!()
}

#[derive(Clone)]
struct TaskConfig {
	id: String,
	cmd: String,
	no_start: bool,
}
#[derive(Default, Clone)]
struct Config {
	notify_vsock: Option<(String, String)>,
	listen_vsock: bool,
	tasks: Vec<TaskConfig>,
}

fn main() {
	let art = tokio::runtime::Builder::new_multi_thread()
		.enable_all()
		.build().unwrap();

 	art.block_on(start_app());
}

async fn start_app() {
	obs::configure_tracing();
	let config = cli::parse().unwrap();

	let (mut com_bundle, com) = com::Com::init(&config.tasks);
	CONTEXT.set(Context {
		com,
		cfg: config,
	});

	let mut general_tasks = JoinSet::new();
	let mut child_tasks = JoinSet::new();
	general_tasks.spawn(event_manager(com_bundle.ev_rx));

	if context().cfg.listen_vsock {
		general_tasks.spawn(vsock::listen_vsock());
	}

	for task_config in context().cfg.tasks.clone().into_iter() {
		let rx = com_bundle.tasks_rx.remove(&task_config.id).unwrap();
		child_tasks.spawn(task_manager(task_config, rx));
	}
	thread::spawn(readline);

	signal::ctrl_c().await.unwrap();
	tracing::info!("{}", "shutting down".paint(style::INFO));

	for task in context().com.tasks.values() {
		task.send(task::TaskCommand::Break).await;
	}
	tracing::info!("{}", "waiting for child tasks...".paint(style::INFO));

	child_tasks.join_all().await;
	general_tasks.shutdown();
	return println!()
}

fn readline() {
	let stdin = std::io::stdin();
	let mut handle = stdin.lock();

	let mut line = String::new();
	loop {
		String::clear(&mut line);
		handle.read_line(&mut line);
		let name = line.trim().to_string();
		context().com.ev.blocking_send(EvCommand::ReadLine(name));
	}
}

pub enum EvCommand {
	ReadLine(String),
	RemoteMessage(Message),
	ProcessExited(String),
}
async fn event_manager(mut rx: mpsc::Receiver<EvCommand>) {
	let ctx = context();
	while let Some(cmd) = rx.recv().await {
		match cmd {
			EvCommand::ReadLine(line) => {
				if line == "R" {
					for task in ctx.com.tasks.values() {
						task.send(task::TaskCommand::Restart).await;
					}
					continue
				}
				let Some(task) = ctx.com.tasks.get(&line) else {
					tracing::error!("{}", format!("task {line} was not found").paint(style::NB));
					continue
				};
				ctx.com.task(&line).send(task::TaskCommand::Restart).await;
				continue
			}
			EvCommand::ProcessExited(id) => {
				tracing::error!("{}", format!("{id} exited").paint(style::INFO));
				if let Some(ref notify_path) = context().cfg.notify_vsock {
					tokio::spawn(async move {
						use tokio_vsock::{VsockStream, VsockAddr};
						let Ok(mut vsock) = VsockStream::connect(VsockAddr::new(3, 9000)).await else { return };
						let msg = Message { variant: MessageVariant::ProcessExited(id) };
						bincode::encode_into_std_write(msg, &mut vsock, bincode::config::standard()).unwrap();
					});
				}
			}
			EvCommand::RemoteMessage(Message { variant }) => {
				tracing::debug!("receviced message {variant:?}");
				match variant {
					MessageVariant::ProcessExited(id) => {
						let Some(task) = ctx.com.tasks.get(&id) else { continue };
						task.send(task::TaskCommand::Restart).await;
						continue
					}
				}
			}
		}
	}
}

#[derive(Debug, Encode, Decode)]
pub struct Message {
	pub variant: MessageVariant
}

#[derive(Debug, Encode, Decode)]
pub enum MessageVariant {
	ProcessExited(String)
}
