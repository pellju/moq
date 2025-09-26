use url::Url;

use anyhow::Context;
use clap::Parser;

mod abrcontroller;
use moq_lite::*;


/// Based on moq-clock and
#[derive(Parser,Clone)]
pub struct Config {
	#[arg()]
	pub url: Url,

	/// The name of the broadcast to publish or subscribe to.
	#[arg(long, default_value = "abr")]
	pub broadcast: String,

	#[command(flatten)]
	pub client: moq_native::ClientConfig,

	/// The default name for the track
	#[arg(long, default_value = "seconds")]
	pub track: String,

	/// The log configuration
	#[command(flatten)]
	pub log: moq_native::Log,

}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	let config = Config::parse();
	config.log.init();
	//tracing::info!("[moq-abr-controller] config inited!");

	let client: moq_native::Client = config.client.init()?;
	//tracing::info!("[moq-abr-controller] Client defined!");
	//tracing::info!(url = ?config.url, "[moq-abr-controller] Connecting to server...\n");

	let session = client.connect(config.url).await?;
	//tracing::info!("[moq-abr-controller] Session created!");
	let origin = moq_lite::Origin::produce();
	//tracing::info!("[moq-abr-controller] Origin created!");
	let session = moq_lite::Session::connect(session, None, Some(origin.producer)).await?;
	//tracing::info!("[moq-abr-controller] Session set!");

	let track = Track { name: config.track, priority: 0, };
	//tracing::info!("[moq-abr-controller] Track defined!");
	//tracing::info!("[moq-abr-controller] Track: {}", track.name);

	let broadcast = origin.consumer.consume_broadcast(&config.broadcast).context("[moq-abr-controller] broadcast not found!")?; // Hajoaa tähän...
	//tracing::info!("[moq-abr-controller] Broadcast defined!");
	let track = broadcast.subscribe_track(&track);
	tracing::info!("[moq-abr-controller] Creating a new video stream:");
	let receiver = abrcontroller::Abrcontroller::new(track);

	tokio::select! {
		res = session.closed() => Err(res.into()),
		_ = receiver.run() => Ok(()),
	}

}
