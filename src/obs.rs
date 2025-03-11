use crate::common::AppResult;
use std::io::IsTerminal;
use tracing::{Level};
use tracing_subscriber::{filter, layer::{Layer, SubscriberExt}};

pub fn configure_tracing() -> AppResult<()> {
	let app_filter = filter::filter_fn(|metadata| {
		if metadata.target().starts_with(env!("CARGO_CRATE_NAME")) {
			return true
		}
		#[cfg(debug_assertions)]
		return *metadata.level() <= Level::DEBUG;
		return *metadata.level() <= Level::INFO;
	});

	let stdout_layer = {
		let stdout_layer = tracing_subscriber::fmt::layer();
		#[cfg(debug_assertions)]
		let stdout_layer = stdout_layer.without_time();

		stdout_layer
			.with_writer(std::io::stdout)
			// .with_ansi(std::io::stdout().is_terminal())
			// .with_file(cfg!(debug_assertions)).with_line_number(cfg!(debug_assertions))
			.with_file(false).with_line_number(false)
			.without_time()
			// .with_timer(timer.clone())
			.with_level(false).with_target(false)
			.with_filter(app_filter.clone())
	};

	let subscriber = tracing_subscriber::registry()
		.with(stdout_layer);

	tracing::subscriber::set_global_default(subscriber)?;

	Ok(())
}
