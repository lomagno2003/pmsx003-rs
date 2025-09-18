#![no_std]

use embedded_io::{Read, Write, ErrorType, ReadExactError};

const CMD_FRAME_SIZE: usize = 7;
const OUTPUT_FRAME_SIZE: usize = 32;
const RESPONSE_FRAME_SIZE: usize = 8;
const CHECKSUM_SIZE: usize = 2;

type Response = [u8; RESPONSE_FRAME_SIZE];

pub const MN1: u8 = 0x42;
pub const MN2: u8 = 0x4D;
const PASSIVE_MODE_RESPONSE: Response = [MN1, MN1, 0x00, 0x04, 0xE1, 0x00, 0x01, 0x74];
const ACTIVE_MODE_RESPONSE: Response = [MN1, MN2, 0x00, 0x04, 0xE1, 0x01, 0x01, 0x75];
const SLEEP_RESPONSE: Response = [MN1, MN2, 0x00, 0x04, 0xE4, 0x00, 0x01, 0x77];

#[derive(Debug)]
pub enum Error<E> {
    Read(ReadExactError<E>),
    Write(E),
    ChecksumError,
    IncorrectResponse,
    NoResponse,
}

/// Sensor interface
pub struct PmsX003Sensor<UART> {
    uart: UART,
}

impl<UART> PmsX003Sensor<UART>
where
    UART: Read + Write + ErrorType,
{
    /// Creates a new sensor instance
    /// * `uart` - UART implementing embedded-io Read + Write traits
    pub fn new(uart: UART) -> Self {
        Self { uart }
    }

    fn read_from_device<T: AsMut<[u8]>>(&mut self, mut buffer: T) -> Result<T, Error<UART::Error>> {
        let buf = buffer.as_mut();
        
        // Find the magic numbers (0x42, 0x4D) at the start of a frame
        let mut temp_buf = [0u8; 1];
        loop {
            // Read first magic number
            loop {
                match self.uart.read_exact(&mut temp_buf) {
                    Ok(()) => {
                        if temp_buf[0] == MN1 {
                            break;
                        }
                    }
                    Err(e) => return Err(Error::Read(e)),
                }
            }
            
            // Read second magic number
            match self.uart.read_exact(&mut temp_buf) {
                Ok(()) => {
                    if temp_buf[0] == MN2 {
                        // Found both magic numbers, set them in buffer and read the rest
                        buf[0] = MN1;
                        buf[1] = MN2;
                        match self.uart.read_exact(&mut buf[2..]) {
                            Ok(()) => break,
                            Err(e) => return Err(Error::Read(e)),
                        }
                    }
                    // If second byte wasn't MN2, continue looking for MN1
                }
                Err(e) => return Err(Error::Read(e)),
            }
        }
        
        Ok(buffer)
    }

    /// Reads sensor status. Blocks until status is available.
    pub fn read(&mut self) -> Result<OutputFrame, Error<UART::Error>> {
        OutputFrame::from_buffer(&self.read_from_device([0_u8; OUTPUT_FRAME_SIZE])?)
    }

    /// Sleep mode. May fail because of incorrect response because of race condition between response and air quality status
    pub fn sleep(&mut self) -> Result<(), Error<UART::Error>> {
        self.send_cmd(&create_command(0xe4, 0))?;
        self.receive_response(SLEEP_RESPONSE)
    }

    pub fn wake(&mut self) -> Result<(), Error<UART::Error>> {
        self.send_cmd(&create_command(0xe4, 1))
    }

    /// Passive mode - sensor reports air quality on request
    pub fn passive(&mut self) -> Result<(), Error<UART::Error>> {
        self.send_cmd(&create_command(0xe1, 0))?;
        self.receive_response(PASSIVE_MODE_RESPONSE)
    }

    /// Active mode - sensor reports air quality continuously
    pub fn active(&mut self) -> Result<(), Error<UART::Error>> {
        self.send_cmd(&create_command(0xe1, 1))?;
        self.receive_response(ACTIVE_MODE_RESPONSE)
    }

    /// Requests status in passive mode
    pub fn request(&mut self) -> Result<(), Error<UART::Error>> {
        self.send_cmd(&create_command(0xe2, 0))
    }

    fn send_cmd(&mut self, cmd: &[u8]) -> Result<(), Error<UART::Error>> {
        match self.uart.write_all(cmd) {
            Ok(()) => Ok(()),
            Err(_) => Err(Error::NoResponse), // Simplify for now
        }
    }

    fn receive_response(&mut self, expected_response: Response) -> Result<(), Error<UART::Error>> {
        if self.read_from_device([0u8; RESPONSE_FRAME_SIZE])? != expected_response {
            Err(Error::IncorrectResponse)
        } else {
            Ok(())
        }
    }
}

fn create_command(cmd: u8, data: u16) -> [u8; CMD_FRAME_SIZE] {
    let mut buffer = [0_u8; CMD_FRAME_SIZE];
    let mut offset = 0usize;

    // Write magic numbers and command
    buffer[offset] = MN1;
    offset += 1;
    buffer[offset] = MN2;
    offset += 1;
    buffer[offset] = cmd;
    offset += 1;

    // Write data as big-endian u16
    let data_bytes = data.to_be_bytes();
    buffer[offset..offset + 2].copy_from_slice(&data_bytes);
    offset += 2;

    // Calculate checksum
    let checksum = buffer
        .iter()
        .take(CMD_FRAME_SIZE - CHECKSUM_SIZE)
        .map(|b| *b as u16)
        .sum::<u16>();

    // Write checksum as big-endian u16
    let checksum_bytes = checksum.to_be_bytes();
    buffer[offset..offset + 2].copy_from_slice(&checksum_bytes);

    buffer
}

/// Contains data reported by the sensor
#[derive(Default, Debug)]
pub struct OutputFrame {
    pub start1: u8,
    pub start2: u8,
    pub frame_length: u16,
    pub pm1_0: u16,
    pub pm2_5: u16,
    pub pm10: u16,
    pub pm1_0_atm: u16,
    pub pm2_5_atm: u16,
    pub pm10_atm: u16,
    pub beyond_0_3: u16,
    pub beyond_0_5: u16,
    pub beyond_1_0: u16,
    pub beyond_2_5: u16,
    pub beyond_5_0: u16,
    pub beyond_10_0: u16,
    pub reserved: u16,
    pub check: u16,
}

impl OutputFrame {
    pub fn from_buffer<E>(buffer: &[u8; OUTPUT_FRAME_SIZE]) -> Result<Self, Error<E>> {
        let sum: usize = buffer
            .iter()
            .take(OUTPUT_FRAME_SIZE - CHECKSUM_SIZE)
            .map(|e| *e as usize)
            .sum();

        let mut frame = OutputFrame::default();
        let mut offset = 0usize;

        // Read u8 values
        frame.start1 = buffer[offset];
        offset += 1;
        frame.start2 = buffer[offset];
        offset += 1;

        // Read u16 values as big-endian
        frame.frame_length = u16::from_be_bytes([buffer[offset], buffer[offset + 1]]);
        offset += 2;
        frame.pm1_0 = u16::from_be_bytes([buffer[offset], buffer[offset + 1]]);
        offset += 2;
        frame.pm2_5 = u16::from_be_bytes([buffer[offset], buffer[offset + 1]]);
        offset += 2;
        frame.pm10 = u16::from_be_bytes([buffer[offset], buffer[offset + 1]]);
        offset += 2;
        frame.pm1_0_atm = u16::from_be_bytes([buffer[offset], buffer[offset + 1]]);
        offset += 2;
        frame.pm2_5_atm = u16::from_be_bytes([buffer[offset], buffer[offset + 1]]);
        offset += 2;
        frame.pm10_atm = u16::from_be_bytes([buffer[offset], buffer[offset + 1]]);
        offset += 2;
        frame.beyond_0_3 = u16::from_be_bytes([buffer[offset], buffer[offset + 1]]);
        offset += 2;
        frame.beyond_0_5 = u16::from_be_bytes([buffer[offset], buffer[offset + 1]]);
        offset += 2;
        frame.beyond_1_0 = u16::from_be_bytes([buffer[offset], buffer[offset + 1]]);
        offset += 2;
        frame.beyond_2_5 = u16::from_be_bytes([buffer[offset], buffer[offset + 1]]);
        offset += 2;
        frame.beyond_5_0 = u16::from_be_bytes([buffer[offset], buffer[offset + 1]]);
        offset += 2;
        frame.beyond_10_0 = u16::from_be_bytes([buffer[offset], buffer[offset + 1]]);
        offset += 2;
        frame.reserved = u16::from_be_bytes([buffer[offset], buffer[offset + 1]]);
        offset += 2;
        frame.check = u16::from_be_bytes([buffer[offset], buffer[offset + 1]]);

        if sum != frame.check as usize {
            return Err(Error::ChecksumError);
        }

        Ok(frame)
    }
}


