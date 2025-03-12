use std::{process::ExitStatus, sync::Arc, time::Duration};
use tokio::{io, process::{Child as Process, Command}, sync::{mpsc, oneshot, Mutex, MutexGuard}};
use yansi::Paint;
use crate::{cli::style, context, TaskConfig};

pub enum TaskCommand {
	Stop,
	Restart,
	RestartWith { on_stop: Option<oneshot::Sender<()>>, on_start: Option<oneshot::Sender<()>> },
	Break,
}

pub async fn task_manager(config: TaskConfig, mut rx: mpsc::Receiver<TaskCommand>) {
	let mut controller = ProcessController::new(config.clone());
	if !config.no_start {
		context().com.task(&config.id).send(TaskCommand::Restart).await;
	}

	let controller = Arc::new(Mutex::new(controller));
	let (waiter, _) = tokio::sync::broadcast::channel::<()>(1);

	let acquire_controller = async || -> MutexGuard<ProcessController> {
		if let Ok(ctrl) = controller.try_lock() { return ctrl }
		waiter.send(());
		return controller.lock().await
	};

	let start_waiter = || tokio::spawn({
		let (controller, mut rx, id) = (controller.clone(), waiter.subscribe(), config.id.clone());
		async move {
			let mut ctrl = controller.lock().await;
			let Some(ref mut process) = ctrl.process else { return };
			tokio::select! {
				_ = rx.recv() => {},
				_ = process.wait() => { context().com.ev.send(crate::EvCommand::ProcessExited(id)).await; },
			}
		}
	});

	while let Some(cmd) = rx.recv().await {
		let mut controller = acquire_controller().await;

		match cmd {
			TaskCommand::Restart => {
				controller.restart().await;
				tracing::info!("{}", format!("{} started", config.id).paint(style::NB));
				start_waiter();
			}
			TaskCommand::RestartWith { on_stop, on_start } => {
				controller.stop().await;
				on_stop.map(|c| c.send(()));

				controller.restart().await;
				tracing::info!("{}", format!("{} started", config.id).paint(style::NB));

				start_waiter();
				on_start.map(|c| c.send(()));
			}
			TaskCommand::Stop => {
				controller.stop().await;
			}
			TaskCommand::Break => {
				controller.stop().await;
				return
			}
		}
	}
}

struct ProcessController {
	config: TaskConfig,
	process: Option<Process>,
}
impl ProcessController {
	pub fn new(config: TaskConfig) -> Self {
		return Self { config, process: None }
	}
	#[tracing::instrument(skip_all, fields(task = self.config.id, pid = self.process.as_ref().and_then(|p| p.id())))]
	pub async fn restart(&mut self) {
		self.stop().await;

		let spawn_result = Command::new("sh").arg("-c").arg(self.config.cmd.as_str())
			.stdout(std::io::stdout()).stderr(std::io::stderr())
			.spawn();

		self.process = match spawn_result {
			Ok(p) => Some(p),
			Err(err) => {
				tracing::error!("{}\n{err}", "failed to start process".on_red());
				None
			}
		};
	}
	#[tracing::instrument(skip_all, fields(task = self.config.id, pid = self.process.as_ref().and_then(|p| p.id())))]
	pub async fn stop(&mut self) {
		if self.exit_status().await.is_some() { return }

		let Some(ref mut process) = self.process else { return };
		let Some(id) = process.id() else { return tracing::error!("failed to get process id") };

		unsafe { libc::kill(id as _, libc::SIGINT) };
		match tokio::time::timeout(Duration::from_millis(3000), process.wait()).await {
			Ok(Ok(status)) => return tracing::debug!("{}", format!("{status}").paint(style::INFO)),
			_ => tracing::warn!("{}", "failed to stop in 3000ms, will use `kill -s SIGINTT`".on_red()),
		}
		let mut kill_task = Command::new("kill").arg("-9").arg(id.to_string())
			.stdout(std::io::stdout()).stderr(std::io::stderr())
			.spawn().inspect_err(|err| tracing::error!("{}", "failed to `kill -9`\n{err}".on_red())).unwrap();

		kill_task.wait().await;
	}
	pub async fn exit_status(&mut self) -> Option<ExitStatus> {
		let Some(ref mut process) = self.process else { return None };
		return match process.try_wait() {
			Ok(Some(status)) => return status.into(),
			Ok(None) => return None,
			Err(_) => return None,
		}
	}
}
