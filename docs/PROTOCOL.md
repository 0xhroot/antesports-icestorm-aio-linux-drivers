# ANTESPORTS Monitor USB HID Protocol

## Device Identification

| Protocol | Vendor ID | Product ID | Description |
|----------|-----------|------------|-------------|
| Classic   | `0x5131`  | `0x2007`   | Vevor / HT / ANT Esports LCD coolers |
| iUnity   | `0x2022`  | `0x0522`   | Antec Vortex View / Flux Pro |

## Classic Protocol (0x5131:0x2007)

64-byte HID output report, sent to `hidraw` device.

### Frame Structure

```
Byte  Offset  Value      Description
------ ------ ---------- -------------
 0      0     0x00       Report ID
 1      1     0x00       Header / sync byte
 2-34   2-34  <values>   33 data bytes (value array starts here)
35-64  35-64  0x00       Padding
```

Total on wire: 65 bytes (report ID + 64-byte report).

**Note:** Values start at byte[2], not byte[4] as claimed by earlier
protocol descriptions. The `0x01 0x02` opcode/subopcode bytes found in some
references appear to be artifacts of the original Windows .NET app's CyUSB
abstraction layer — the device firmware reads bytes[2..34] directly as the
33-element value array.

### Encoding Rules

All byte values are computed using **truncation** (C# `(byte)` cast / `int()`), **not rounding**.
The reference implementation uses `b(x) = clamp(trunc(x), 0, 255)`.

### Data Byte Layout (33 bytes, at positions 2..34 in the frame)

| Index | Field             | Encoding                          |
|-------|-------------------|-----------------------------------|
| 0     | CPU temp integer  | `trunc(temp)`                     |
| 1     | CPU temp decimal  | `trunc((temp - trunc(temp)) * 100)` |
| 2     | CPU temp unit     | `0` = Celsius, `1` = Fahrenheit   |
| 3     | CPU usage         | `trunc(percent)`                  |
| 4     | CPU power low     | `trunc(power) % 100`              |
| 5     | CPU power decimal | `trunc((power - trunc(power)) * 100)` |
| 6     | CPU freq hundreds | `trunc(trunc(freq) / 100)`        |
| 7     | CPU freq units    | `trunc(freq) % 100`               |
| 8     | CPU voltage int   | `trunc(voltage)`                  |
| 9     | CPU voltage frac  | `trunc(voltage) * 100` (no fractional part — proven quirk) |
| 10    | GPU temp integer  | `trunc(temp)`                     |
| 11    | GPU temp decimal  | `trunc((temp - trunc(temp)) * 100)` |
| 12    | GPU temp unit     | `0` = Celsius, `1` = Fahrenheit   |
| 13    | GPU usage         | `trunc(percent)`                  |
| 14    | GPU power low     | `trunc(power) % 100`              |
| 15    | GPU power decimal | `trunc((power - trunc(power)) * 100)` |
| 16    | GPU freq hundreds | `trunc(trunc(freq) / 100)`        |
| 17    | GPU freq units    | `trunc(freq) % 100`               |
| 18    | Fan RPM hundreds  | `trunc(trunc(rpm) / 100)`         |
| 19    | Fan RPM units     | `trunc(rpm) % 100`                |
| 20    | Pump RPM hundreds | `trunc(trunc(rpm) / 100)`         |
| 21    | Pump RPM units    | `trunc(rpm) % 100`                |
| 22    | Year high         | `YYYY / 100` (first 2 digits)     |
| 23    | Year low          | `YYYY % 100` (last 2 digits)      |
| 24    | Month             | 1-12                              |
| 25    | Day               | 1-31                              |
| 26    | Hour              | 0-23                              |
| 27    | Minute            | 0-59                              |
| 28    | Second            | 0-59                              |
| 29    | Day of week       | 0=Sun, 6=Sat (ISO `%u` % 7)      |
| 30    | RAM usage         | Percent (0-100)                   |
| 31    | CPU power high    | `trunc(power) / 100` (hundreds)   |
| 32    | GPU power high    | `trunc(power) / 100` (hundreds)   |

### Reconstruction (firmware side)
```
CPU_temp  = [0]  + [1]/100
CPU_power = [31]*100 + [4] + [5]/100
CPU_freq  = [6]*100 + [7]
GPU_power = [32]*100 + [14] + [15]/100
GPU_freq  = [16]*100 + [17]
Fan_RPM   = [18]*100 + [19]
Pump_RPM  = [20]*100 + [21]
```

### Quirk: CPU voltage encoding
The original .NET app computes `v[9] = trunc(voltage) * 100` (it stores the integer
part multiplied by 100, never the fractional part). This is a confirmed copy-paste
bug from the temperature/power encoding code and is reproduced verbatim for
firmware compatibility. The fractional digits of CPU voltage are not available
on the LCD.

### Shutdown Frame

```
Byte 0: 0x00  (report ID)
Byte 1: 0x0F  (shutdown marker — replaces header byte)
Bytes 2-64: 0x00 (padding)
```

Sets header byte to `0x0F` to signal the device to clear the LCD display.
(All zeros is indistinguishable from "no data", so `0x0F` provides a unique
sentinel that won't collide with any normal sensor values.)

## iUnity Protocol (0x2022:0x0522)

12-byte packet with checksum, used by Antec Vortex View / Flux Pro.

### Frame Structure

```
Byte  Value  Description
----- ------ -------------
 0    0x55   Magic header byte 1
 1    0xAA   Magic header byte 2
 2    0x01   Command
 3    0x01   Sub-command
 4    0x06   Payload length (6 bytes follow)
 5     CPU1  CPU temp tens digit
 6     CPU2  CPU temp units digit
 7     CPU3  CPU temp tenths digit
 8     GPU1  GPU temp tens digit
 9     GPU2  GPU temp units digit
10     GPU3  GPU temp tenths digit
11     CHK   Checksum (sum of bytes 0-10, modulo 256)
```

Temperature digits are ASCII digits (0-9) converted to integer by subtracting `b'0'`.

Example: CPU 45.6°C, GPU 38.2°C → `55 AA 01 01 06 04 05 06 03 08 02 <sum>`

## References

- [coldwelderx/cooler-lcd-linux](https://github.com/coldwelderx/cooler-lcd-linux) — Original reverse engineering for 0x5131:0x2007
- [MoiSieurAlex/HID_LCD_Test](https://github.com/MoiSieurAlex/HID_LCD_Test) — iUnity protocol research for 0x2022:0x0522
