use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::sync::Mutex;
use std::time::Instant;

use ant_monitor_protocol::SensorValues;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SensorError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("No sensors found")]
    NoSensors,
}

static CPU_STATS: LazyLock<Mutex<(u64, u64)>> = LazyLock::new(|| Mutex::new((0, 0)));

pub struct SensorReader {
    pub cpu_temp_path: Option<PathBuf>,
    pub hwmon_base: Option<PathBuf>,
    pub rapl_path: Option<PathBuf>,
    pub pump_rpm_path: Option<PathBuf>,
    pub cpu_volt_path: Option<PathBuf>,
    rapl_energy: u64,
    rapl_time: Instant,
}

impl Default for SensorReader {
    fn default() -> Self {
        Self::new()
    }
}

impl SensorReader {
    pub fn new() -> Self {
        let mut reader = SensorReader {
            cpu_temp_path: None,
            hwmon_base: None,
            rapl_path: None,
            rapl_energy: 0,
            rapl_time: Instant::now(),
            pump_rpm_path: None,
            cpu_volt_path: None,
        };
        reader.init();
        reader
    }

    fn init(&mut self) {
        let hwmon_base = Path::new("/sys/class/hwmon");
        if !hwmon_base.exists() {
            return;
        }

        if let Ok(entries) = fs::read_dir(hwmon_base) {
            for entry in entries.flatten() {
                let path = entry.path();
                let name_file = path.join("name");
                let name = fs::read_to_string(&name_file).unwrap_or_default();
                let name = name.trim().to_string();

                if name == "coretemp" || name == "k10temp" || name == "zenpower" {
                    self.hwmon_base = Some(path.clone());
                    if let Ok(dir_entries) = fs::read_dir(&path) {
                        for e in dir_entries.flatten() {
                            let p = e.path();
                            let fname = p
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("")
                                .to_string();
                            if fname.starts_with("temp") && fname.ends_with("_input") {
                                self.cpu_temp_path = Some(p);
                            }
                        }
                    }
                    if self.cpu_volt_path.is_none() {
                        self.cpu_volt_path = self.find_voltage_path(&path);
                    }
                }

                if (name == "nct6775" || name == "nct6687" || name == "it8628")
                    && self.cpu_volt_path.is_none()
                {
                    self.cpu_volt_path = self.find_voltage_path(&path);
                }
            }
        }

        let rapl = Path::new("/sys/class/powercap");
        if let Ok(entries) = fs::read_dir(rapl) {
            for entry in entries.flatten() {
                let path = entry.path();
                let name_file = path.join("name");
                if let Ok(name) = fs::read_to_string(&name_file) {
                    let name = name.trim().to_string();
                    if name == "package-0" || name == "psys" {
                        let energy = path.join("energy_uj");
                        if energy.exists() {
                            self.rapl_energy = Self::read_u64(&energy).unwrap_or(0);
                            self.rapl_time = Instant::now();
                            self.rapl_path = Some(energy);
                        }
                    }
                }
            }
        }
    }

    fn read_u64(path: &Path) -> Result<u64, SensorError> {
        let s = fs::read_to_string(path)?;
        s.trim()
            .parse::<u64>()
            .map_err(|e| SensorError::Parse(format!("{}: {}", path.display(), e)))
    }

    fn read_f32_millideg(path: &Path) -> Result<f32, SensorError> {
        let val = Self::read_u64(path)? as f32;
        Ok(val / 1000.0)
    }

    fn read_f32(path: &Path) -> Result<f32, SensorError> {
        let s = fs::read_to_string(path)?;
        s.trim()
            .parse::<f32>()
            .map_err(|e| SensorError::Parse(format!("{}: {}", path.display(), e)))
    }

    pub fn read_cpu_temp(&self) -> f32 {
        self.cpu_temp_path
            .as_ref()
            .and_then(|p| Self::read_f32_millideg(p).ok())
            .unwrap_or(0.0)
    }

    pub fn read_cpu_usage(&self) -> f32 {
        let stat = match fs::read_to_string("/proc/stat") {
            Ok(s) => s,
            Err(_) => return 0.0,
        };
        let line = match stat.lines().next() {
            Some(l) => l,
            None => return 0.0,
        };
        let parts: Vec<u64> = line
            .split_whitespace()
            .skip(1)
            .filter_map(|s| s.parse().ok())
            .collect();
        if parts.len() < 3 {
            return 0.0;
        }
        let total: u64 = parts.iter().sum();
        let idle = parts[3];

        if let Ok(mut prev) = CPU_STATS.lock() {
            if prev.0 == 0 {
                *prev = (total, idle);
                return 0.0;
            }
            let d_total = total - prev.0;
            let d_idle = idle - prev.1;
            *prev = (total, idle);
            if d_total == 0 {
                return 0.0;
            }
            (d_total - d_idle) as f32 / d_total as f32 * 100.0
        } else {
            0.0
        }
    }

    pub fn read_cpu_power(&mut self) -> f32 {
        if let Some(ref rapl) = self.rapl_path {
            let e_now = Self::read_u64(rapl).unwrap_or(self.rapl_energy);
            let t_now = Instant::now();
            let dt = (t_now - self.rapl_time).as_secs_f64();
            let de = e_now.saturating_sub(self.rapl_energy);
            self.rapl_energy = e_now;
            self.rapl_time = t_now;
            if dt > 0.0 {
                return (de as f64 / 1_000_000.0 / dt) as f32;
            }
        }
        0.0
    }

    pub fn read_cpu_freq(&self) -> f32 {
        let path = Path::new("/sys/devices/system/cpu/cpu0/cpufreq/scaling_cur_freq");
        if let Ok(val) = Self::read_u64(path) {
            return val as f32 / 1000.0;
        }
        0.0
    }

    pub fn read_cpu_voltage(&self) -> f32 {
        self.cpu_volt_path
            .as_ref()
            .and_then(|p| Self::read_f32(p).ok())
            .unwrap_or(0.0)
    }

    fn find_voltage_path(&self, hwmon_path: &Path) -> Option<PathBuf> {
        if let Ok(entries) = fs::read_dir(hwmon_path) {
            for e in entries.flatten() {
                let p = e.path();
                let fname = p
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();
                if fname.starts_with("in") && fname.ends_with("_input") {
                    return Some(p);
                }
            }
        }
        None
    }

    pub fn read_fan_rpm(&self) -> f32 {
        if let Ok(entries) = fs::read_dir("/sys/class/hwmon") {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Ok(dir_entries) = fs::read_dir(&path) {
                    for e in dir_entries.flatten() {
                        let p = e.path();
                        let fname = p
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("")
                            .to_string();
                        if fname.starts_with("fan") && fname.ends_with("_input") {
                            return Self::read_f32(&p).unwrap_or(0.0);
                        }
                    }
                }
            }
        }
        0.0
    }

    pub fn read_pump_rpm(&self) -> f32 {
        self.pump_rpm_path
            .as_ref()
            .and_then(|p| Self::read_f32(p).ok())
            .unwrap_or(0.0)
    }

    pub fn read_ram_usage(&self) -> u8 {
        let info = match fs::read_to_string("/proc/meminfo") {
            Ok(s) => s,
            Err(_) => return 0,
        };
        let mut total = 0u64;
        let mut available = 0u64;
        for line in info.lines() {
            if line.starts_with("MemTotal:") {
                if let Some(val) = line.split_whitespace().nth(1) {
                    total = val.parse().unwrap_or(0);
                }
            } else if line.starts_with("MemAvailable:") {
                if let Some(val) = line.split_whitespace().nth(1) {
                    available = val.parse().unwrap_or(0);
                }
            }
        }
        if total == 0 {
            return 0;
        }
        let used = total - available;
        (used as f64 / total as f64 * 100.0) as u8
    }

    pub fn read_all(&mut self) -> SensorValues {
        let gpu_temp = self.read_gpu_temp_generic();
        SensorValues {
            cpu_temp_c: self.read_cpu_temp(),
            cpu_usage_pct: self.read_cpu_usage(),
            cpu_power_w: self.read_cpu_power(),
            cpu_freq_mhz: self.read_cpu_freq(),
            cpu_voltage_v: self.read_cpu_voltage(),
            gpu_temp_c: gpu_temp,
            gpu_usage_pct: 0.0,
            gpu_power_w: 0.0,
            gpu_freq_mhz: 0.0,
            fan_rpm: self.read_fan_rpm(),
            pump_rpm: self.read_pump_rpm(),
            ram_usage_pct: self.read_ram_usage(),
        }
    }

    pub fn read_gpu_temp_generic(&self) -> f32 {
        let drm = Path::new("/sys/class/drm");
        if let Ok(entries) = fs::read_dir(drm) {
            for entry in entries.flatten() {
                let path = entry.path();
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();
                if name.starts_with("card") && !name.contains("-") {
                    let hwmon_dir = path.join("device").join("hwmon");
                    if let Ok(hwmons) = fs::read_dir(&hwmon_dir) {
                        for hwmon in hwmons.flatten() {
                            let temp = hwmon.path().join("temp1_input");
                            if temp.exists() {
                                return Self::read_f32_millideg(&temp).unwrap_or(0.0);
                            }
                        }
                    }
                }
            }
        }
        0.0
    }
}
