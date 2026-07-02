use chrono::{Datelike, Timelike};
use serde::{Deserialize, Serialize};

pub const ANT_MONITOR_VID: u16 = 0x5131;
pub const ANT_MONITOR_PID: u16 = 0x2007;

pub const ANTEC_IUNITY_VID: u16 = 0x2022;
pub const ANTEC_IUNITY_PID: u16 = 0x0522;

pub const REPORT_ID: u8 = 0x00;
pub const REPORT_LEN: usize = 64;

const HEADER: u8 = 0x00;
const SHUTDOWN_MARKER: u8 = 0x0F;

pub const TEMP_UNIT_CELSIUS: u8 = 0;
pub const TEMP_UNIT_FAHRENHEIT: u8 = 1;

pub fn b(x: f32) -> u8 {
    (x.trunc().clamp(0.0, 255.0)) as u8
}

pub fn build_refresh_frame(values: &SensorValues, temp_unit: u8) -> Vec<u8> {
    let mut frame = vec![REPORT_ID, HEADER];
    frame.extend_from_slice(&values.to_value_array(temp_unit));
    frame.resize(REPORT_LEN + 1, 0x00);
    frame
}

pub fn build_shutdown_frame() -> Vec<u8> {
    let mut frame = vec![REPORT_ID, SHUTDOWN_MARKER];
    frame.resize(REPORT_LEN + 1, 0x00);
    frame
}

pub fn build_iunity_frame(cpu_temp: f32, gpu_temp: f32) -> Vec<u8> {
    fn digits(temp: f32) -> (u8, u8, u8) {
        let t = temp.clamp(0.0, 99.9);
        let tens = (t / 10.0).trunc() as u8;
        let units = (t % 10.0).trunc() as u8;
        let tenths = ((t * 10.0) % 10.0).trunc() as u8;
        (tens, units, tenths)
    }

    let (ct, cu, ctenths) = digits(cpu_temp);
    let (gt, gu, gtenths) = digits(gpu_temp);

    let mut payload = vec![0x55, 0xAA, 0x01, 0x01, 0x06];
    payload.push(ct);
    payload.push(cu);
    payload.push(ctenths);
    payload.push(gt);
    payload.push(gu);
    payload.push(gtenths);
    let checksum: u8 = payload.iter().copied().reduce(|a, b| a.wrapping_add(b)).unwrap();
    payload.push(checksum);
    payload
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorValues {
    pub cpu_temp_c: f32,
    pub cpu_usage_pct: f32,
    pub cpu_power_w: f32,
    pub cpu_freq_mhz: f32,
    pub cpu_voltage_v: f32,
    pub gpu_temp_c: f32,
    pub gpu_usage_pct: f32,
    pub gpu_power_w: f32,
    pub gpu_freq_mhz: f32,
    pub fan_rpm: f32,
    pub pump_rpm: f32,
    pub ram_usage_pct: u8,
}

impl SensorValues {
    pub fn to_value_array(&self, temp_unit: u8) -> [u8; 33] {
        let mut v = [0u8; 33];
        let cpu_t = if temp_unit == TEMP_UNIT_FAHRENHEIT {
            self.cpu_temp_c * 9.0 / 5.0 + 32.0
        } else {
            self.cpu_temp_c
        };
        let gpu_t = if temp_unit == TEMP_UNIT_FAHRENHEIT {
            self.gpu_temp_c * 9.0 / 5.0 + 32.0
        } else {
            self.gpu_temp_c
        };

        let cpu_t_i = cpu_t.floor();
        v[0] = b(cpu_t_i);
        v[1] = b((cpu_t - cpu_t_i) * 100.0);
        v[2] = temp_unit;
        v[3] = b(self.cpu_usage_pct);
        let cpu_p_i = self.cpu_power_w.floor();
        v[4] = b(cpu_p_i % 100.0);
        v[5] = b((self.cpu_power_w - cpu_p_i) * 100.0);
        v[31] = b(cpu_p_i / 100.0);
        let cpu_f_i = self.cpu_freq_mhz.floor();
        v[6] = b(cpu_f_i / 100.0);
        v[7] = b(cpu_f_i % 100.0);
        let cpu_v_i = self.cpu_voltage_v.floor();
        v[8] = b(cpu_v_i);
        v[9] = b(cpu_v_i * 100.0);
        v[10] = b(gpu_t.floor());
        v[11] = b((gpu_t - gpu_t.floor()) * 100.0);
        v[12] = temp_unit;
        v[13] = b(self.gpu_usage_pct);
        let gpu_p_i = self.gpu_power_w.floor();
        v[14] = b(gpu_p_i % 100.0);
        v[15] = b((self.gpu_power_w - gpu_p_i) * 100.0);
        v[32] = b(gpu_p_i / 100.0);
        let gpu_f_i = self.gpu_freq_mhz.floor();
        v[16] = b(gpu_f_i / 100.0);
        v[17] = b(gpu_f_i % 100.0);
        let fan_i = self.fan_rpm.floor();
        v[18] = b(fan_i / 100.0);
        v[19] = b(fan_i % 100.0);
        let pump_i = self.pump_rpm.floor();
        v[20] = b(pump_i / 100.0);
        v[21] = b(pump_i % 100.0);
        let now = chrono::Local::now();
        let yr = format!("{:04}", now.year());
        v[22] = yr[..2].parse().unwrap_or(20);
        v[23] = yr[2..].parse().unwrap_or(25);
        v[24] = now.month() as u8;
        v[25] = now.day() as u8;
        v[26] = now.hour() as u8;
        v[27] = now.minute() as u8;
        v[28] = now.second() as u8;
        let dotw = now.format("%u").to_string().parse::<u8>().unwrap_or(1);
        v[29] = dotw % 7;
        v[30] = self.ram_usage_pct;
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_b_truncates() {
        let vals: [(f32, u8); 6] = [
            (53.7, 53),
            (0.0, 0),
            (255.9, 255),
            (256.0, 255),
            (-1.0, 0),
            (42.999, 42),
        ];
        for (input, expected) in vals {
            assert_eq!(b(input), expected, "b({input}) should be {expected}");
        }
    }

    #[test]
    fn test_to_value_array_structure() {
        let sv = SensorValues {
            cpu_temp_c: 53.75,
            cpu_usage_pct: 42.6,
            cpu_power_w: 75.8,
            cpu_freq_mhz: 2400.0,
            cpu_voltage_v: 1.2,
            gpu_temp_c: 45.3,
            gpu_usage_pct: 68.9,
            gpu_power_w: 150.5,
            gpu_freq_mhz: 1800.0,
            fan_rpm: 1234.6,
            pump_rpm: 2800.0,
            ram_usage_pct: 67,
        };

        let v = sv.to_value_array(TEMP_UNIT_CELSIUS);

        assert_eq!(v.len(), 33, "value array must be exactly 33 bytes");

        assert_eq!(v[0], 53, "CPU temp int");
        assert_eq!(v[1], 75, "CPU temp frac*100");
        assert_eq!(v[2], 0, "CPU unit Celsius");

        assert_eq!(v[3], 42, "CPU usage (trunc, not round)");

        assert_eq!(v[4], 75, "CPU power % 100");
        assert_eq!(v[5], 80, "CPU power frac*100");
        assert_eq!(v[31], 0, "CPU power / 100 (hundreds)");

        assert_eq!(v[6], 24, "CPU freq / 100");
        assert_eq!(v[7], 0, "CPU freq % 100");

        assert_eq!(v[8], 1, "CPU voltage int");
        assert_eq!(v[9], 100, "CPU voltage int*100 (quirk)");

        assert_eq!(v[10], 45, "GPU temp int");
        assert_eq!(v[11], 29, "GPU temp frac*100 (45.3*100 = 29.999.. truncated to 29)");

        assert_eq!(v[13], 68, "GPU usage (trunc, not round)");

        assert_eq!(v[14], 50, "GPU power % 100");
        assert_eq!(v[15], 50, "GPU power frac*100");
        assert_eq!(v[32], 1, "GPU power / 100 (hundreds)");

        assert_eq!(v[16], 18, "GPU freq / 100");
        assert_eq!(v[17], 0, "GPU freq % 100");

        assert_eq!(v[18], 12, "Fan RPM / 100");
        assert_eq!(v[19], 34, "Fan RPM % 100 (trunc)");

        assert_eq!(v[20], 28, "Pump RPM / 100");
        assert_eq!(v[21], 0, "Pump RPM % 100");

        assert_eq!(v[30], 67, "RAM usage");
    }

    #[test]
    fn test_refresh_frame_structure() {
        let sv = SensorValues {
            cpu_temp_c: 53.0,
            cpu_usage_pct: 10.0,
            cpu_power_w: 75.0,
            cpu_freq_mhz: 2400.0,
            cpu_voltage_v: 1.2,
            gpu_temp_c: 45.0,
            gpu_usage_pct: 50.0,
            gpu_power_w: 100.0,
            gpu_freq_mhz: 1800.0,
            fan_rpm: 800.0,
            pump_rpm: 0.0,
            ram_usage_pct: 42,
        };

        let frame = build_refresh_frame(&sv, TEMP_UNIT_CELSIUS);

        assert_eq!(frame.len(), 65, "frame must be 65 bytes (report ID + 64-byte report)");

        assert_eq!(frame[0], REPORT_ID, "byte 0 = report ID");
        assert_eq!(frame[1], HEADER, "byte 1 = header 0x00");
        assert_eq!(frame[2], 53, "byte 2 = CPU temp int (values start here)");

        for i in 35..64 {
            assert_eq!(frame[i + 1], 0x00, "byte {} = padding 0x00", i + 1);
        }
    }

    #[test]
    fn test_shutdown_frame_structure() {
        let frame = build_shutdown_frame();

        assert_eq!(frame.len(), 65, "shutdown frame must be 65 bytes");

        assert_eq!(frame[0], REPORT_ID, "byte 0 = report ID");
        assert_eq!(frame[1], SHUTDOWN_MARKER, "byte 1 = shutdown marker 0x0F");

        for i in 2..64 {
            assert_eq!(frame[i + 1], 0x00, "byte {} = padding 0x00", i + 1);
        }
    }

    #[test]
    fn test_iunity_frame_known_values() {
        let frame = build_iunity_frame(45.6, 38.2);
        assert_eq!(frame, vec![0x55, 0xAA, 0x01, 0x01, 0x06, 4, 5, 6, 3, 8, 2, 0x23]);
    }

    #[test]
    fn test_iunity_frame_zero() {
        let frame = build_iunity_frame(0.0, 0.0);
        assert_eq!(frame, vec![0x55, 0xAA, 0x01, 0x01, 0x06, 0, 0, 0, 0, 0, 0, 0x07]);
    }

    #[test]
    fn test_iunity_frame_clamps() {
        let frame = build_iunity_frame(100.0, -5.0);
        assert_eq!(frame, vec![0x55, 0xAA, 0x01, 0x01, 0x06, 9, 9, 9, 0, 0, 0, 0x22]);
    }

    #[test]
    fn test_b_rounding_regression() {
        let sv = SensorValues {
            cpu_temp_c: 53.0,
            cpu_usage_pct: 42.6,
            cpu_power_w: 75.0,
            cpu_freq_mhz: 2400.0,
            cpu_voltage_v: 1.2,
            gpu_temp_c: 45.0,
            gpu_usage_pct: 68.9,
            gpu_power_w: 100.0,
            gpu_freq_mhz: 1800.0,
            fan_rpm: 800.0,
            pump_rpm: 2800.0,
            ram_usage_pct: 67,
        };
        let v = sv.to_value_array(TEMP_UNIT_CELSIUS);

        assert_eq!(v[3], 42, "CPU usage 42.6 must truncate to 42, not round to 43");
        assert_eq!(v[13], 68, "GPU usage 68.9 must truncate to 68, not round to 69");
    }
}
