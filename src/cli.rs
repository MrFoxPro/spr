use crate::{common::AppResult, Config, TaskConfig};

const ARG_C: &str = "-c";
const ARG_CN: &str = "-cn";
const ARG_NOTIFY_VSOCK: &str = "--notify-vsock";
const ARG_LISTEN_VSOCK: &str = "--listen-vsock";

pub fn parse() -> AppResult<Config> {
	let mut cfg = Config::default();
	let args: Vec<String> = std::env::args().skip(1).map(|arg| arg.trim().to_owned()).collect();

	let len = args.len();
	if len == 0 { return Err("commands weren't specified".into()) }

	let last = len - 1;

	fn is_arg(s: &String) -> bool { [ARG_C, ARG_CN, ARG_NOTIFY_VSOCK, ARG_LISTEN_VSOCK].contains(&s.as_ref()) }
	fn is_word(s: &String) -> bool { !is_arg(s) }

	let mut iter = args.into_iter().peekable();
	while let Some(word) = iter.next() {
		match word.as_str() {
			ARG_C | ARG_CN => {
				let cmd   = iter.next_if(is_word).expect("cmd was expected");
				let alias = iter.next_if(is_word).expect("alias expected");
				let id    = iter.next_if(is_word);
				cfg.tasks.push(TaskConfig { cmd, alias: alias.clone(), id: id.unwrap_or(alias), no_start: word == ARG_CN });
			}
			ARG_NOTIFY_VSOCK => {
				let Some(word) = iter.next_if(is_word) else { panic!("expected cid:port for {word}") };
				let Some((Some(port), Some(cid))) = word.split_once(":").map(|(a, b)| (a.parse::<u32>().ok(), b.parse::<u32>().ok()))
				else { panic!("{word} parameter should be in cid:port format (e.g. 3:9000)") };
				cfg.notify_vsock = Some((port, cid));
			}
			ARG_LISTEN_VSOCK => {
				let Some(word) = iter.next_if(is_word) else { panic!("expected port for {word}") };
				let Some(port) = word.parse::<u32>().ok() else { panic!("port should be u32 number") };
				cfg.listen_vsock = Some(port);
			}
			_ => panic!("unknown word {word}")
		}
	}
	return Ok(cfg)
}

pub mod style {
	use yansi::{Color, Style};
	pub const NB: Style = Color::Rgb(255, 165, 0).bold();
	pub const INFO: Style = Color::Rgb(110, 194, 7).bold();
}
