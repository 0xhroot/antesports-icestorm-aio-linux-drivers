use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use ant_monitor_protocol::TEMP_UNIT_CELSIUS;
use ant_monitor_sensors::SensorReader;
use ant_monitor_usb::{AntMonitorDevice, UsbError};

const REFRESH_INTERVAL: Duration = Duration::from_millis(200);
const RECONNECT_DELAY: Duration = Duration::from_secs(2);
const DEVICE_TIMEOUT: Duration = Duration::from_secs(10);
const STARTUP_BURST_COUNT: usize = 5;

fn main() {
    simple_logging::log_to_stderr(log::LevelFilter::Info);
    log::info!("ANTESPORTS Monitor Daemon v{}", env!("CARGO_PKG_VERSION"));

    let running = Arc::new(AtomicBool::new(true));
    let mut sensors = SensorReader::new();

    // Block SIGTERM and SIGINT in the main thread before spawning any threads,
    // so child threads inherit the mask and only the sigwait thread handles them.
    let mut sigset: libc::sigset_t = unsafe { std::mem::zeroed() };
    unsafe {
        libc::sigemptyset(&mut sigset);
        libc::sigaddset(&mut sigset, libc::SIGTERM);
        libc::sigaddset(&mut sigset, libc::SIGINT);
        libc::pthread_sigmask(libc::SIG_BLOCK, &sigset, std::ptr::null_mut());
    }

    spawn_signal_handler(running.clone());

    log::info!("Waiting for ANTESPORTS Monitor device...");
    let device = match AntMonitorDevice::try_open() {
        Some(d) => {
            log::info!("Device connected (protocol: {:?})", d.protocol());
            d
        }
        None => match ant_monitor_usb::wait_for_device(DEVICE_TIMEOUT) {
            Some(d) => {
                log::info!("Device connected (protocol: {:?})", d.protocol());
                d
            }
            None => {
                log::error!("No ANTESPORTS Monitor device found after {}s", DEVICE_TIMEOUT.as_secs());
                std::process::exit(1);
            }
        },
    };

    run_loop(device, &mut sensors, &running);
}

fn spawn_signal_handler(running: Arc<AtomicBool>) {
    // The signal handler thread inherits the blocked signal mask from main,
    // so SIGTERM and SIGINT are blocked here too.
    // sigwait atomically unblocks them, waits, then re-blocks.
    std::thread::spawn(move || {
        let mut sigset: libc::sigset_t = unsafe { std::mem::zeroed() };
        unsafe {
            libc::sigemptyset(&mut sigset);
            libc::sigaddset(&mut sigset, libc::SIGTERM);
            libc::sigaddset(&mut sigset, libc::SIGINT);
        }
        loop {
            let mut sig: libc::c_int = 0;
            let ret = unsafe { libc::sigwait(&sigset, &mut sig) };
            if ret == 0 {
                log::info!("Received signal {}, shutting down...", sig);
                running.store(false, Ordering::SeqCst);
                // Give the main loop 2s to exit gracefully, then force exit
                std::thread::sleep(Duration::from_secs(2));
                log::warn!("Shutdown timeout, force exiting");
                unsafe { libc::_exit(0); }
            }
        }
    });
}

fn run_loop(
    device: AntMonitorDevice,
    sensors: &mut SensorReader,
    running: &AtomicBool,
) {
    let sv = sensors.read_all();
    log::info!(
        "CPU:{:.1}°C GPU:{:.1}°C Fan:{:.0} RPM RAM:{}%",
        sv.cpu_temp_c, sv.gpu_temp_c, sv.fan_rpm, sv.ram_usage_pct
    );

    let mut burst_remaining = STARTUP_BURST_COUNT;
    let mut device = device;

    while running.load(Ordering::SeqCst) {
        let sv = sensors.read_all();

        match device.send_refresh(&sv, TEMP_UNIT_CELSIUS) {
            Ok(()) => {
                if burst_remaining > 0 {
                    burst_remaining -= 1;
                    continue;
                }
                log::debug!(
                    "CPU:{:.1}°C GPU:{:.1}°C Fan:{:.0} RPM RAM:{}%",
                    sv.cpu_temp_c,
                    sv.gpu_temp_c,
                    sv.fan_rpm,
                    sv.ram_usage_pct
                );
            }
            Err(UsbError::Busy) => {
                std::thread::sleep(Duration::from_millis(10));
                continue;
            }
            Err(UsbError::DeviceNotFound) | Err(UsbError::WriteFailed(_)) => {
                log::warn!("Device disconnected, reconnecting...");
                burst_remaining = STARTUP_BURST_COUNT;
                device = match wait_for_device_retry(running) {
                    Some(d) => {
                        log::info!("Device reconnected");
                        d
                    }
                    None => break,
                };
                continue;
            }
            Err(e) => {
                log::error!("USB error: {e}");
                std::thread::sleep(RECONNECT_DELAY);
                continue;
            }
        }

        std::thread::sleep(REFRESH_INTERVAL);
    }

    let _ = device.send_shutdown();
    log::info!("Daemon stopped");
}

fn wait_for_device_retry(running: &AtomicBool) -> Option<AntMonitorDevice> {
    while running.load(Ordering::SeqCst) {
        if let Some(d) = AntMonitorDevice::try_open() {
            return Some(d);
        }
        std::thread::sleep(RECONNECT_DELAY);
    }
    None
}
