use pmsx003::PmsX003Sensor;
use embedded_hal_nb::serial::{ErrorType, Read, Write, Error as SerialError, ErrorKind};

struct RxMock {}
struct TxMock {}

#[derive(Debug)]
struct MockError;

impl SerialError for MockError {
    fn kind(&self) -> ErrorKind {
        ErrorKind::Other
    }
}

impl ErrorType for RxMock {
    type Error = MockError;
}

impl Read for RxMock {
    fn read(&mut self) -> nb::Result<u8, Self::Error> {
        Err(nb::Error::Other(MockError))
    }
}

impl ErrorType for TxMock {
    type Error = MockError;
}

impl Write for TxMock {
    fn write(&mut self, _: u8) -> nb::Result<(), Self::Error> {
        Ok(())
    }

    fn flush(&mut self) -> nb::Result<(), Self::Error> {
        Ok(())
    }
}

#[test]
fn crate_instance_using_separate_rx_tx() {
    let tx = TxMock {};
    let rx = RxMock {};

    let mut pms = PmsX003Sensor::new_tx_rx(tx, rx);
    let _ = pms.sleep();
}
