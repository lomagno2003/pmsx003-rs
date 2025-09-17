# pmsx003-rs

[![Crates.io](https://img.shields.io/crates/v/pmsx003.svg)](https://crates.io/crates/pmsx003)
[![Documentation](https://docs.rs/pmsx003/badge.svg)](https://docs.rs/pmsx003)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A `no_std` Rust driver for Plantower PMS X003 series air quality sensors, including PMS5003, PMS7003, and other compatible models.

## Features

- **`no_std` compatible** - Perfect for embedded systems
- **embedded-hal v1.0.0** - Uses the latest embedded HAL traits
- **Zero dependencies** - No external parsing libraries, uses built-in Rust byte manipulation
- **Flexible serial interface** - Supports both combined and separate TX/RX serial interfaces
- **Comprehensive sensor control** - Active/passive modes, sleep/wake, and data reading
- **Robust parsing** - Built-in checksum validation and error handling
- **Multiple sensor support** - Works with PMS5003, PMS7003, and other PMS X003 series sensors

## Supported Sensors

This driver supports the Plantower PMS X003 series of air quality sensors:

- **PMS5003** - Measures PM1.0, PM2.5, PM10 with particle counting
- **PMS7003** - Similar to PMS5003 with additional features
- **PMS3003** - Basic PM2.5 and PM10 measurements
- Other compatible PMS X003 series sensors

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
pmsx003 = "1.0.0"
embedded-hal = "1.0.0"
embedded-hal-nb = "1.0.0"
nb = "1.0.0"
```

### Basic Example

```rust
use pmsx003::{PmsX003Sensor, OutputFrame};
use embedded_hal_nb::serial::{Read, Write};

// Create sensor with a combined serial interface
let mut sensor = PmsX003Sensor::new(serial);

// Read air quality data
match sensor.read() {
    Ok(frame) => {
        println!("PM1.0: {} μg/m³", frame.pm1_0);
        println!("PM2.5: {} μg/m³", frame.pm2_5);
        println!("PM10:  {} μg/m³", frame.pm10);
    }
    Err(e) => println!("Error reading sensor: {:?}", e),
}
```

### Separate TX/RX Interface

```rust
// Create sensor with separate TX and RX interfaces
let mut sensor = PmsX003Sensor::new_tx_rx(tx, rx);

// Same usage as above
let frame = sensor.read()?;
```

### Sensor Control

```rust
// Put sensor in passive mode (request data manually)
sensor.passive()?;
sensor.request()?;
let frame = sensor.read()?;

// Put sensor in active mode (continuous data)
sensor.active()?;
loop {
    let frame = sensor.read()?;
    // Process data...
}

// Power management
sensor.sleep()?;  // Put sensor to sleep
sensor.wake()?;   // Wake up sensor
```

## Data Structure

The `OutputFrame` struct contains all sensor measurements:

```rust
pub struct OutputFrame {
    pub pm1_0: u16,        // PM1.0 concentration (μg/m³)
    pub pm2_5: u16,        // PM2.5 concentration (μg/m³)
    pub pm10: u16,         // PM10 concentration (μg/m³)
    pub pm1_0_atm: u16,    // PM1.0 atmospheric environment (μg/m³)
    pub pm2_5_atm: u16,    // PM2.5 atmospheric environment (μg/m³)
    pub pm10_atm: u16,     // PM10 atmospheric environment (μg/m³)
    pub beyond_0_3: u16,   // Particles > 0.3μm per 0.1L air
    pub beyond_0_5: u16,   // Particles > 0.5μm per 0.1L air
    pub beyond_1_0: u16,   // Particles > 1.0μm per 0.1L air
    pub beyond_2_5: u16,   // Particles > 2.5μm per 0.1L air
    pub beyond_5_0: u16,   // Particles > 5.0μm per 0.1L air
    pub beyond_10_0: u16,  // Particles > 10.0μm per 0.1L air
    // ... other fields
}
```

## Error Handling

The driver provides comprehensive error handling:

```rust
use pmsx003::Error;

match sensor.read() {
    Ok(frame) => { /* Process data */ }
    Err(Error::ChecksumError) => println!("Data corruption detected"),
    Err(Error::ReadFailed) => println!("Serial read failed"),
    Err(Error::SendFailed) => println!("Serial write failed"),
    Err(Error::IncorrectResponse) => println!("Unexpected sensor response"),
    Err(Error::NoResponse) => println!("Sensor not responding"),
}
```

## Platform Examples

### ESP32 with esp-hal

```rust
use esp_hal::{
    gpio::Io,
    uart::{config::Config, Uart},
    prelude::*,
};
use pmsx003::PmsX003Sensor;

let io = Io::new(peripherals.GPIO, peripherals.IO_MUX);
let uart = Uart::new_with_config(
    peripherals.UART1,
    Config::default(),
    Some(io.pins.gpio4),  // TX
    Some(io.pins.gpio5),  // RX
    &clocks,
)?;

let mut sensor = PmsX003Sensor::new(uart);
let frame = sensor.read()?;
```

### STM32 with stm32-hal

```rust
use stm32f4xx_hal::{
    pac,
    prelude::*,
    serial::{config::Config, Serial},
};
use pmsx003::PmsX003Sensor;

let dp = pac::Peripherals::take().unwrap();
let gpioa = dp.GPIOA.split();
let rcc = dp.RCC.constrain();
let clocks = rcc.cfgr.freeze();

let tx = gpioa.pa2.into_alternate();
let rx = gpioa.pa3.into_alternate();

let serial = Serial::new(
    dp.USART2,
    (tx, rx),
    Config::default().baudrate(9600.bps()),
    &clocks,
)?;

let mut sensor = PmsX003Sensor::new(serial);
let frame = sensor.read()?;
```

## Wiring

Connect your PMS X003 sensor to your microcontroller:

| PMS X003 Pin | Function | MCU Pin |
|--------------|----------|---------|
| VCC          | Power    | 5V      |
| GND          | Ground   | GND     |
| TXD          | Data out | RX      |
| RXD          | Data in  | TX      |
| SET          | Sleep    | GPIO (optional) |
| RESET        | Reset    | GPIO (optional) |

**Note:** The sensor's TXD connects to your MCU's RX, and sensor's RXD connects to your MCU's TX.

## Communication Protocol

The driver handles the PMS X003 communication protocol automatically:

- **Baud rate:** 9600 bps
- **Data format:** 8N1 (8 data bits, no parity, 1 stop bit)
- **Frame format:** 32-byte data frames with checksum validation
- **Commands:** Sleep/wake, active/passive mode control

## Migration from v0.x

If you're upgrading from an older version:

1. **Struct name changed:** `Pms7003Sensor` → `PmsX003Sensor`
2. **embedded-hal version:** Now requires embedded-hal v1.0.0
3. **Dependencies:** `scroll` dependency removed (zero external deps)

```rust
// Old (v0.x)
use pmsx003::Pms7003Sensor;
let sensor = Pms7003Sensor::new(serial);

// New (v1.x)
use pmsx003::PmsX003Sensor;
let sensor = PmsX003Sensor::new(serial);
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Plantower for creating the PMS X003 series sensors
- The embedded Rust community for the excellent embedded-hal ecosystem