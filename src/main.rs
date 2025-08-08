mod app;
mod config;
mod event;
mod key_processor;
mod key_sender;
mod serial;
mod winusb;

use crate::key_processor::KeyMappingProcessor;
use crate::key_sender::KeySender;
use clap::Parser;
use eframe::egui;
use log::{debug, error, info, warn};
use std::sync::{Arc, mpsc};
use std::thread;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "config.json")]
    config: String,
}

fn main() {
    env_logger::init();
    let args = Args::parse();
    info!("Starting TourBox application");

    let (tourbox_sender, tourbox_receiver) = mpsc::channel();
    let (app_sender, app_receiver) = mpsc::channel();

    let config = Arc::new(match config::Config::from_file(&args.config) {
        Ok(cfg) => {
            info!("Configuration loaded from '{}'", &args.config);
            cfg
        }
        Err(e) => {
            error!(
                "Failed to read or parse config file '{}': {}",
                &args.config, e
            );
            return;
        }
    });

    match &config.device {
        config::TourBoxDevice::WinUsb { vid: _, pid: _ } => {
            winusb::winusb_tourbox_processor(config.clone(), tourbox_sender.clone());
        }
        config::TourBoxDevice::Serial {
            serial_port: _,
            baud_rate: _,
        } => {
            serial::serial_tourbox_processor(config.clone(), tourbox_sender.clone());
        }
    }

    let cfg = config.clone();
    thread::spawn(move || {
        let mut processor = KeyMappingProcessor::from_config(&cfg.mappings);
        let mut key_sender = KeySender::new();

        loop {
            let event = tourbox_receiver.recv().ok();

            if let Some(event) = event {
                let a = processor.process(event);
                debug!("{a:?}");
                for v in a.into_iter() {
                    if let Err(e) = key_sender.send_key(&v) {
                        warn!("{e}");
                    }

                    app_sender.send(v).expect("Channel to app is broken");
                }
                // send to ui
            }
        }
    });

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([500.0, 400.0]),
        ..Default::default()
    };

    info!("Starting eframe application");

    if let Err(e) = eframe::run_native(
        "TourBox Command Receiver",
        native_options,
        Box::new(move |cc| Box::new(app::TourApp::new(app_receiver, cc.egui_ctx.clone()))),
    ) {
        error!("Error running eframe application: {}", e);
    }
}
