use std::io::{stdout, Write};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use ant_monitor_protocol::{SensorValues, ANT_MONITOR_PID, ANT_MONITOR_VID, ANTEC_IUNITY_PID, ANTEC_IUNITY_VID};
use ant_monitor_sensors::SensorReader;
use ant_monitor_usb::{AntMonitorDevice, ProtocolType};

enum Message {
    DeviceConnected { vid: u16, pid: u16, protocol: String },
    DeviceDisconnected,
    SensorData(SensorValues),
}

fn main() {
    let (tx, rx) = mpsc::channel::<Message>();

    thread::spawn(move || {
        let mut sensors = SensorReader::new();
        loop {
            let device = AntMonitorDevice::try_open();
            match device {
                Some(dev) => {
                    let vid = if dev.protocol() == ProtocolType::Classic {
                        ANT_MONITOR_VID
                    } else {
                        ANTEC_IUNITY_VID
                    };
                    let pid = if dev.protocol() == ProtocolType::Classic {
                        ANT_MONITOR_PID
                    } else {
                        ANTEC_IUNITY_PID
                    };
                    let _ = tx.send(Message::DeviceConnected {
                        vid,
                        pid,
                        protocol: format!("{:?}", dev.protocol()),
                    });

                    loop {
                        let sv = sensors.read_all();
                        if dev.send_refresh(&sv, 0).is_err() {
                            let _ = tx.send(Message::DeviceDisconnected);
                            break;
                        }
                        let _ = tx.send(Message::SensorData(sv));
                        thread::sleep(Duration::from_millis(200));
                    }
                }
                None => {
                    let _ = tx.send(Message::DeviceDisconnected);
                    thread::sleep(Duration::from_secs(2));
                }
            }
        }
    });

    println!("ANTESPORTS Monitor CLI");
    println!("======================");
    println!("Waiting for device...\n");

    loop {
        match rx.recv() {
            Ok(Message::DeviceConnected { vid, pid, protocol }) => {
                println!(
                    "=== Device Connected: {:#06x}:{:#06x} ({}) ===",
                    vid, pid, protocol
                );
            }
            Ok(Message::DeviceDisconnected) => {
                println!("\n=== Device Disconnected ===");
            }
            Ok(Message::SensorData(sv)) => {
                print!(
                    "\rCPU: {:5.1}°C {:3.0}% {:5.0}MHz | GPU: {:5.1}°C {:3.0}% | Fan: {:4.0} RPM | RAM: {:2}%   ",
                    sv.cpu_temp_c, sv.cpu_usage_pct, sv.cpu_freq_mhz,
                    sv.gpu_temp_c, sv.gpu_usage_pct,
                    sv.fan_rpm, sv.ram_usage_pct
                );
                stdout().flush().ok();
            }
            Err(_) => break,
        }
    }
}
