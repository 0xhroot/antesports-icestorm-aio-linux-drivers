use std::time::Duration;

use ant_monitor_protocol::*;
use hidapi::{HidApi, HidDevice};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum UsbError {
    #[error("HID error: {0}")]
    Hid(#[from] hidapi::HidError),
    #[error("Device not found")]
    DeviceNotFound,
    #[error("Write failed: {0}")]
    WriteFailed(String),
    #[error("Device busy (EAGAIN)")]
    Busy,
}

pub struct AntMonitorDevice {
    device: HidDevice,
    protocol: ProtocolType,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProtocolType {
    Classic,
    IUnity,
}

impl AntMonitorDevice {
    pub fn open() -> Result<Self, UsbError> {
        let api = HidApi::new()?;

        if let Ok(device) = api.open(ANT_MONITOR_VID, ANT_MONITOR_PID) {
            device.set_blocking_mode(false)?;
            return Ok(AntMonitorDevice {
                device,
                protocol: ProtocolType::Classic,
            });
        }

        if let Ok(device) = api.open(ANTEC_IUNITY_VID, ANTEC_IUNITY_PID) {
            device.set_blocking_mode(false)?;
            return Ok(AntMonitorDevice {
                device,
                protocol: ProtocolType::IUnity,
            });
        }

        Err(UsbError::DeviceNotFound)
    }

    pub fn try_open() -> Option<Self> {
        let api = HidApi::new().ok()?;

        if let Ok(device) = api.open(ANT_MONITOR_VID, ANT_MONITOR_PID) {
            let _ = device.set_blocking_mode(false);
            return Some(AntMonitorDevice {
                device,
                protocol: ProtocolType::Classic,
            });
        }

        if let Ok(device) = api.open(ANTEC_IUNITY_VID, ANTEC_IUNITY_PID) {
            let _ = device.set_blocking_mode(false);
            return Some(AntMonitorDevice {
                device,
                protocol: ProtocolType::IUnity,
            });
        }

        None
    }

    pub fn enumerate() -> Result<Vec<(u16, u16, String)>, UsbError> {
        let api = HidApi::new()?;
        let mut devices = Vec::new();

        for info in api.device_list() {
            if (info.vendor_id() == ANT_MONITOR_VID && info.product_id() == ANT_MONITOR_PID)
                || (info.vendor_id() == ANTEC_IUNITY_VID
                    && info.product_id() == ANTEC_IUNITY_PID)
            {
                let path = info.path().to_string_lossy().to_string();
                devices.push((info.vendor_id(), info.product_id(), path));
            }
        }

        Ok(devices)
    }

    pub fn protocol(&self) -> ProtocolType {
        self.protocol
    }

    pub fn send_refresh(&self, values: &SensorValues, temp_unit: u8) -> Result<(), UsbError> {
        match self.protocol {
            ProtocolType::Classic => {
                let frame = build_refresh_frame(values, temp_unit);
                match self.device.write(&frame) {
                    Ok(written) if written == frame.len() => Ok(()),
                    Ok(0) => Err(UsbError::Busy),
                    Ok(written) => Err(UsbError::WriteFailed(format!(
                        "wrote {} of {} bytes",
                        written, frame.len()
                    ))),
                    Err(e) => Err(UsbError::Hid(e)),
                }
            }
            ProtocolType::IUnity => {
                let frame = build_iunity_frame(values.cpu_temp_c, values.gpu_temp_c);
                match self.device.write(&frame) {
                    Ok(written) if written == frame.len() => Ok(()),
                    Ok(0) => Err(UsbError::Busy),
                    Ok(written) => Err(UsbError::WriteFailed(format!(
                        "wrote {} of {} bytes",
                        written, frame.len()
                    ))),
                    Err(e) => Err(UsbError::Hid(e)),
                }
            }
        }
    }

    pub fn send_shutdown(&self) -> Result<(), UsbError> {
        match self.protocol {
            ProtocolType::Classic => {
                let frame = build_shutdown_frame();
                let _ = self.device.write(&frame)?;
            }
            ProtocolType::IUnity => {
                let frame = build_iunity_frame(0.0, 0.0);
                let _ = self.device.write(&frame)?;
            }
        }
        Ok(())
    }

    pub fn close(self) {
    }
}

pub fn wait_for_device(timeout: Duration) -> Option<AntMonitorDevice> {
    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        if let Some(device) = AntMonitorDevice::try_open() {
            return Some(device);
        }
        std::thread::sleep(Duration::from_secs(1));
    }
    None
}
