// use color_print::cprintln;
use tokio::io::AsyncReadExt;
use tokio_vsock::{VsockListener, VsockAddr};
use crate::{common::AppResult, context, EvCommand, Message};

const VSOCK_BLOCK_SIZE: usize = 2048;

#[tracing::instrument]
pub async fn listen_vsock() {
	let listener = VsockListener::bind(VsockAddr::new(libc::VMADDR_CID_ANY, 9000)).unwrap();
	// cprintln!("<#6ec207><bold>listening VSOCK on {}", listener.local_addr().unwrap());
	tracing::info!("listening VSOCK on {}", listener.local_addr().unwrap());
	loop {
		let (mut stream, addr) = match listener.accept().await {
			Ok(x) => x,
			Err(err) => {
				tracing::warn!("failed to accept VSOCK connection");
				continue
			}
		};
		tracing::debug!("accepted VSOCK {addr}");

		loop {
			let mut buf = vec![0u8; VSOCK_BLOCK_SIZE];
			let count = match stream.read(&mut buf).await {
				Ok(0) => {
					tracing::debug!("VSOCK {addr} disconnected (EOF)");
					break
				},
				Ok(read_bytes) => read_bytes,
				Err(err) => break tracing::error!("read failed {err}"),
			};
			buf.truncate(count);
			match bincode::decode_from_slice::<Message, _>(&buf, bincode::config::standard()) {
				Ok((msg, ..)) => { context().com.ev.send(EvCommand::RemoteMessage(msg)).await; }
				Err(err) => tracing::error!("{err}"),
			}
		}
	}
}
