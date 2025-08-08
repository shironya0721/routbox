use log::{error, info, warn};
use serialport::{DataBits, Parity, SerialPort, StopBits};
use std::io::{self, Read, Write};
use std::sync::Arc;
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

use crate::config::Config;
use crate::event::{self, InputEvent};

fn initialize_serial_device(
    port_name: &str,
    baud_rate: u32,
) -> Result<Box<dyn SerialPort>, io::Error> {
    info!(
        "Opening serial port '{}' with baud rate {}",
        port_name, baud_rate
    );
    let mut port = serialport::new(port_name, baud_rate)
        .data_bits(DataBits::Eight)
        .parity(Parity::None)
        .stop_bits(StopBits::One)
        .timeout(Duration::from_millis(10))
        .open()
        .map_err(|e| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("Failed to open serial port '{}': {}", port_name, e),
            )
        })?;

    info!("Setting DTR and RTS to false");
    port.write_data_terminal_ready(false)?;
    port.write_request_to_send(false)?;

    let init_command = [0xB5, 0x00, 0x07, 0x04, 0x00, 0x09, 0x00, 0xFE];
    info!("Sending initialization command: {:02X?}", init_command);
    port.write_all(&init_command)?;
    port.flush()?;

    let bytes_to_read = port.bytes_to_read().unwrap_or(0);
    if bytes_to_read > 0 {
        info!("Device has {} bytes to read back", bytes_to_read);
        let mut read_buf = vec![0; bytes_to_read as usize];
        if port.read_exact(&mut read_buf).is_ok() {
            info!("Received data from device: {:02X?}", read_buf);
        } else {
            warn!("Could not read response from device");
        }
    }

    info!("Clearing serial port buffers");
    port.clear(serialport::ClearBuffer::All)?;

    info!("Serial device initialized successfully");
    Ok(port)
}

pub fn serial_tourbox_processor(cfg: Arc<Config>, ev_sender: Sender<InputEvent>) {
    thread::spawn(move || {
        if let crate::config::TourBoxDevice::Serial {
            ref serial_port,
            ref baud_rate,
        } = cfg.device
        {
            info!(
                "Serial thread started for port '{}' at {} baud",
                serial_port, baud_rate
            );
            loop {
                let mut port = loop {
                    match initialize_serial_device(&serial_port, *baud_rate) {
                        Ok(p) => break p,
                        Err(e) => {
                            warn!(
                                "Could not initialize serial device: {}. Retrying in 5 seconds...",
                                e
                            );
                            thread::sleep(Duration::from_secs(5));
                        }
                    }
                };
                let mut byte_buf = [0; 1];
                loop {
                    match port.read(&mut byte_buf) {
                        Ok(count) => {
                            if count > 0 {
                                let key_code = byte_buf[0];
                                let key_code_hex = format!("0x{:02x}", key_code);

                                let ev = if let Some(key_name) =
                                    cfg.key_map.stateless.get(&key_code_hex)
                                {
                                    event::InputEvent::KeyPressed(key_name.clone())
                                } else if let Some(key_name) =
                                    cfg.key_map.stateful.get(&key_code_hex)
                                {
                                    event::InputEvent::KeyPressed(key_name.clone())
                                } else if let Some(key_name) = cfg
                                    .key_map
                                    .stateful
                                    .get(&format!("0x{:02x}", key_code - 0x80))
                                {
                                    event::InputEvent::KeyReleased(key_name.clone())
                                } else {
                                    warn!("Unknown key code {key_code_hex}.");
                                    continue;
                                };

                                if ev_sender.send(ev).is_err() {
                                    warn!("UI thread has been closed. Exiting serial thread.");
                                    return;
                                }
                            }
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => (),
                        Err(e) => {
                            error!("Serial port error: {}", e);
                            break;
                        }
                    }
                }
            }
        } else {
            panic!("Invalid state");
        }
    });
}
