use crate::{common::AppResult, Config, TaskConfig};

const ARG_C: &str = "-c";
const ARG_CN: &str = "-cn";
const ARG_NOTIFY_VSOCK: &str = "--notify-vsock";
const ARG_LISTEN_VSOCK: &str = "--listen-vsock";

pub fn parse() -> AppResult<Config> {
	let mut cfg = Config::default();
	let args: Vec<String> = std::env::args().skip(2).map(|arg| arg.trim().to_owned()).collect();

	let len = args.len();
	if len == 0 { return Err("commands weren't specified".into()) }

	let last = len - 1;

	enum State {
		None,
		Task { cmd: Option<String>, alias: Option<String>, no_start: bool },
		NotifyVsock,
		ListenVSock,
	}
	let mut state = State::Task { cmd: None, alias: None, no_start: false };

	fn is_arg(s: impl AsRef<str>) -> bool {
		[ARG_C, ARG_CN, ARG_NOTIFY_VSOCK, ARG_LISTEN_VSOCK].contains(&s.as_ref())
	}

	for (idx, word) in args.iter().enumerate() {
		let is_word = !is_arg(&word);
		let is_last = idx == last;

		match &mut state {
			State::Task { cmd, alias, no_start } => {
				if is_word {
					if cmd.is_none() { *cmd = word.to_owned().into(); }
					else { *alias = word.to_owned().into(); }
					continue
				}
				let Some(cmd) = cmd else { return Err("command was not specified before {idx} index".into()) };
				cfg.tasks.push(TaskConfig { cmd: cmd.clone(), id: alias.clone().unwrap_or_else(|| cmd.clone()), no_start: *no_start });
			}
			State::NotifyVsock => {
				if is_word {
					let Some(path) = word.split_once(":").map(|(a, b)| (a.to_owned(), b.to_owned()))
					else { return Err("--notify-vsock should be in cid:port format (e.g. 3:9000)".into()) };
					cfg.notify_vsock = path.into();
					continue
				}
			}
			State::ListenVSock => {
			}
			State::None => {}
		}

		if !is_arg(word) && idx != last { return Err(format!("unexepcted argument at {idx} index: {word}").into()) }

		match word.as_str() {
			ARG_C | ARG_CN => {
				state = State::Task { cmd: None, alias: None, no_start: word == "-cn" };
			}
			ARG_NOTIFY_VSOCK => {
				state = State::NotifyVsock;
			}
			ARG_LISTEN_VSOCK => {
				state = State::ListenVSock;
				cfg.listen_vsock = true;
			}
			_ => {}
		}
	}
	return Ok(cfg)
}

pub mod style {
	use yansi::{Color, Style};
	pub const NB: Style = Color::Rgb(255, 165, 0).bold();
	pub const INFO: Style = Color::Rgb(110, 194, 7).bold();
}
