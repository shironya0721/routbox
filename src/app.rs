use eframe::egui;
use log::error;
use std::sync::mpsc::{self, Receiver};

use crate::key_sender::TourAction;

pub struct TourApp {
    active_keys: Vec<TourAction>,
    receiver: Receiver<TourAction>,
}

impl TourApp {
    pub fn new(app_receiver: Receiver<TourAction>, ctx: egui::Context) -> Self {
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            loop {
                let a = app_receiver.recv();
                match a {
                    Ok(k) => {
                        sender.send(k).expect("Channel from app to ui is broken");
                        ctx.request_repaint();
                    }
                    Err(e) => {
                        error!("{e}");
                        panic!();
                    }
                }
            }
        });
        Self {
            receiver,
            active_keys: Vec::new(),
        }
    }
}

impl eframe::App for TourApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Ok(k) = self.receiver.try_recv() {
            self.active_keys.push(k);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("TourBox Command Receiver");
            ui.separator();
            ui.label("Active Keys:");
            egui::ScrollArea::vertical()
                .stick_to_bottom(true)
                .auto_shrink(false)
                .show(ui, |ui| {
                    for a in self.active_keys.iter() {
                        ui.label(format!("{:?}", a));
                    }
                });
        });
    }
}
