use moq_lite::*;

use glib::prelude::Cast;
use glib::object::ObjectExt;
use glib;
use gstreamer;
use gstreamer::prelude::ElementExt;
use gstreamer::prelude::GstBinExt;

use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Context;

pub struct Abrcontroller {
	track: TrackConsumer
}

impl Abrcontroller {

	pub fn new(track: TrackConsumer) -> Self {
		Self { track }
	}

	pub async fn run(mut self) -> anyhow::Result<()> {

		tracing::info!("(abrcontroller.rs) Initializing gstreamer");
		gstreamer::init()?;

		tracing::info!("(abrcontroller.rs) Setting parameters");
		let strating_string = "\
			ximagesrc use-damage=0 ! video/x-raw,framerate=25/1 ! \
			videoconvert ! queue ! \
			x264enc name=encoder tune=zerolatency speed-preset=veryfast key-int-max=30 bitrate=8000 vbv-buf-capacity=1000 ! \
			h264parse ! \
			isofmp4mux fragment-duration=4000000 ! \
			fdsink fd=1
		";

		tracing::info!("(abrcontroller.rs) Launching gstreamer pipeline");
		let pipeline = gstreamer::parse::launch(strating_string)?.dynamic_cast::<gstreamer::Pipeline>().expect("(abrcontroller.rs) Pipeline expected");
		tracing::info!("(abrcontroller.rs) Setting up an encoder");
		let encoder = pipeline.by_name("encoder").expect("(abrcontroller.rs) Encoder element not found");

		tracing::info!("(abrcontroller.rs) Setting pipeline state to playing");
		pipeline.set_state(gstreamer::State::Playing);

		tracing::info!("(abrcontroller.rs) Entering loop to request info what to do with the stream");

		// Adding a vector for timestamps and set bitrates
		let mut changedBitrates: Vec<(u64,u32)> = vec![];
		// Setting the previous changed value to be used for comparing the bitrate, 8000 kbps as the default
		let mut previousValue: u32 = 8000;

		loop {
			match self.track.next_group().await {
				Ok(Some(mut group)) => {
					tracing::info!("[moq-abr-controller] Received a new group");
					//println!("[moq-abr-controller] Received a new group");

					let base = group.read_frame().await.context("[moq-abr-controller] Failed to get the first object")?.context("[moq-abr-controller] Empty group")?;
					let base = String::from_utf8_lossy(&base);
					tracing::info!("[moq-abr-controller] The value: {}", base);
					if base.to_string() == "high" {
						tracing::info!("[moq-abr-controller] Setting new bitrate to 9000 kbps!");
						const newValue:u32 = 9000;
						encoder.set_property("bitrate", &newValue);
						//encoder.set_property("bitrate", 9000);
					} else if base.to_string() == "low" {
						tracing::info!("[moq-abr-controller] Setting new bitrate to 1000 kbps!");
						const newValue:u32 = 1000;
						encoder.set_property("bitrate", &newValue);
						//encoder.set_property("bitrate", 1000);
					} else {
						// Parse input from a text
						let splitted: Vec<&str> = base.split(";").collect();
						let key = splitted[0];
						let value = splitted[1];

						if key == "abr" {
							tracing::info!("[moq-abr-controller] Adjusting the bitrate to {}!", value);
							let mut newValue:u32 = value.parse().unwrap();

							let currentTime = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

							// 0 cannot be set as the bitrate, must use 1 at least
							// Also, checking if value is not over 10000 [kbps]
							if newValue == 0 {
								newValue = 1;
							} else if newValue > 10000 {
								newValue = 10000
							}

							let timestampAndNewvalue = (currentTime, newValue);
							changedBitrates.push(timestampAndNewvalue);

							if newValue != previousValue {
								tracing::info!("{:?}",changedBitrates);
								encoder.set_property("bitrate", newValue);
								tracing::info!("[moq-abr-controller] The new value has been set!");
							} else {
								tracing::info!("[moq-abr-controller] The newly given bitrate is the same than the previous one");
							}
						}

					}
				}

				Ok(None) => {
					println!("[moq-abr-controller] Stream ended (next_group returned None). Waiting briefly...");
					tokio::time::sleep(std::time::Duration::from_secs(1)).await;
				}

				Err(e) => {
					println!("[moq-abr-controller] Error while reading group: {:?}", e);
					tokio::time::sleep(std::time::Duration::from_secs(1)).await;
				}
			}
		}

		///println!("Loop stopped...");
		Ok(())
	}
}
