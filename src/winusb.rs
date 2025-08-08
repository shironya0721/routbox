use log::{error, info, warn};
use rusb::{Context, Device, DeviceDescriptor, DeviceHandle, Direction, TransferType, UsbContext};
use std::io;
use std::sync::Arc;
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

use crate::config::{self, Config};
use crate::event::{self, InputEvent};

struct Endpoints {
    in_address: u8,
    out_address: u8,
}

// This function is a translation of the Python script's logic to find the device and endpoints.
fn find_device_and_endpoints<T: UsbContext>(
    context: &mut T,
    vid: u16,
    pid: u16,
) -> Result<(Device<T>, DeviceDescriptor, Endpoints), rusb::Error> {
    for device in context.devices()?.iter() {
        let device_desc = device.device_descriptor()?;
        if device_desc.vendor_id() == vid && device_desc.product_id() == pid {
            info!("Found device with VID={:04x}, PID={:04x}", vid, pid);
            let config_desc = device.config_descriptor(0)?; // Assuming first configuration

            let mut in_address = None;
            let mut out_address = None;

            for interface in config_desc.interfaces() {
                for interface_desc in interface.descriptors() {
                    if interface_desc.interface_number() == 1 {
                        for endpoint_desc in interface_desc.endpoint_descriptors() {
                            if endpoint_desc.transfer_type() == TransferType::Bulk {
                                if endpoint_desc.direction() == Direction::In {
                                    in_address = Some(endpoint_desc.address());
                                } else if endpoint_desc.direction() == Direction::Out {
                                    out_address = Some(endpoint_desc.address());
                                }
                            }
                        }
                    }
                }
            }

            if let (Some(in_addr), Some(out_addr)) = (in_address, out_address) {
                info!(
                    "Found bulk endpoints: IN=0x{:02x}, OUT=0x{:02x}",
                    in_addr, out_addr
                );
                return Ok((
                    device,
                    device_desc,
                    Endpoints {
                        in_address: in_addr,
                        out_address: out_addr,
                    },
                ));
            }
        }
    }
    Err(rusb::Error::NoDevice)
}

fn initialize_winusb_device(
    vid: u16,
    pid: u16,
) -> Result<(DeviceHandle<Context>, Endpoints), io::Error> {
    let mut context = Context::new().map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    let (device, _, endpoints) =
        find_device_and_endpoints(&mut context, vid, pid).map_err(|e| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("Failed to find USB device {:04x}:{:04x}: {}", vid, pid, e),
            )
        })?;

    let handle = device.open().map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("Could not open USB device: {}", e),
        )
    })?;

    // The python script does device.set_configuration() which rusb does automatically on open.
    // We may need to detach kernel driver if necessary, especially on Linux.
    // On Windows, this is often not needed if the correct driver (e.g., WinUSB) is installed.
    if handle.kernel_driver_active(1).unwrap_or(false) {
        info!("Detaching kernel driver from interface 1");
        handle.detach_kernel_driver(1).map_err(|e| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("Could not detach kernel driver: {}", e),
            )
        })?;
    }

    info!("Claiming interface 1");
    handle.claim_interface(1).map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("Could not claim interface 1: {}", e),
        )
    })?;

    let init_command = [0xB5, 0x00, 0x07, 0x04, 0x00, 0x09, 0x00, 0xFE];
    info!("Sending initialization command: {:02X?}", init_command);
    handle
        .write_bulk(endpoints.out_address, &init_command, Duration::from_secs(1))
        .map_err(|e| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("Could not send init command: {}", e),
            )
        })?;

    info!("WinUSB device initialized successfully");
    Ok((handle, endpoints))
}

pub fn winusb_tourbox_processor(cfg: Arc<Config>, ev_sender: Sender<InputEvent>) {
    // These should come from config, but for now, let's use the example values from the python script.
    // The user's TODO list indicates they will update the config later.

    thread::spawn(move || {
        if let config::TourBoxDevice::WinUsb { vid, pid } = cfg.device {
            info!("WinUSB thread started for device {:04x}:{:04x}", vid, pid);
            loop {
                let (handle, endpoints) = loop {
                    match initialize_winusb_device(vid, pid) {
                        Ok(p) => break p,
                        Err(e) => {
                            warn!(
                                "Could not initialize WinUSB device: {}. Retrying in 5 seconds...",
                                e
                            );
                            thread::sleep(Duration::from_secs(5));
                        }
                    }
                };

                let mut read_buf = [0u8; 64];
                loop {
                    match handle.read_bulk(
                        endpoints.in_address,
                        &mut read_buf,
                        Duration::from_secs(1),
                    ) {
                        Ok(count) => {
                            if count > 0 {
                                // The python script just prints the hex values.
                                // The logic here is copied from serial.rs to process the bytes.
                                // This assumes the data format is the same.
                                let key_code = read_buf[0];
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
                                    warn!("UI thread has been closed. Exiting WinUSB thread.");
                                    // Before returning, it's good practice to release the interface.
                                    handle.release_interface(1).ok();
                                    return;
                                }
                            }
                        }
                        Err(rusb::Error::Timeout) => (), // Timeouts are expected, just continue.
                        Err(e) => {
                            error!("WinUSB read error: {}", e);
                            // On error, release the interface and break the inner loop to re-initialize.
                            handle.release_interface(1).ok();
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
