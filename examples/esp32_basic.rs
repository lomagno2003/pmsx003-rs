//! Basic ESP32 example using embedded-io
//! 
//! This example shows how to use the PMS5003 sensor with ESP32
//! using the embedded-io traits for maximum portability.

#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_hal::{
    clock::ClockControl,
    delay::Delay,
    gpio::Io,
    peripherals::Peripherals,
    prelude::*,
    system::SystemControl,
    uart::{config::Config, Uart},
};
use esp_println::println;
use pmsx003::PmsX003Sensor;

#[entry]
fn main() -> ! {
    let peripherals = Peripherals::take();
    let system = SystemControl::new(peripherals.SYSTEM);
    let clocks = ClockControl::boot_defaults(system.clock_control).freeze();

    let io = Io::new(peripherals.GPIO, peripherals.IO_MUX);
    let delay = Delay::new(&clocks);

    // Configure UART for PMS5003 communication
    // PMS5003 uses 9600 baud, 8N1
    let config = Config {
        baudrate: 9600,
        data_bits: esp_hal::uart::config::DataBits::DataBits8,
        parity: esp_hal::uart::config::Parity::ParityNone,
        stop_bits: esp_hal::uart::config::StopBits::STOP1,
        clock_source: esp_hal::uart::config::ClockSource::default(),
    };

    let uart = Uart::new_with_config(
        peripherals.UART1,
        config,
        Some(io.pins.gpio17), // TX pin - connects to PMS5003 RXD
        Some(io.pins.gpio16), // RX pin - connects to PMS5003 TXD
        &clocks,
    ).unwrap();

    // Create sensor instance - esp-hal's Uart implements embedded-io traits
    let mut sensor = PmsX003Sensor::new(uart);

    println!("PMS5003 sensor initialized");
    println!("Starting continuous readings...");

    loop {
        match sensor.read() {
            Ok(frame) => {
                println!("PM1.0: {} μg/m³", frame.pm1_0);
                println!("PM2.5: {} μg/m³", frame.pm2_5);
                println!("PM10:  {} μg/m³", frame.pm10);
                println!("---");
            }
            Err(e) => {
                println!("Error reading sensor: {:?}", e);
            }
        }

        delay.delay_ms(2000u32); // Read every 2 seconds
    }
}