use crate::{MN1, MN2};

#[derive(PartialEq, Debug)]
pub enum ReadStatus {
    InProgress,
    Finished,
    Failed,
}

enum State {
    WaitingForFirstMagicNumber,
    WaitingForSecondMagicNumber,
    Reading,
    Finished,
    Failed,
}

/// State machine representing ongoing read from device
/// * Waits for magic numbers
/// * Allows for breaks in transmission
/// * Fails after arbitrary number of read retries
pub struct ReadStateMachine<'a> {
    buffer: &'a mut [u8],
    index: usize,
    state: State,
    retries: usize,
}

impl<'a> ReadStateMachine<'a> {
    pub fn new(buffer: &'a mut [u8], retries: usize) -> Self {
        Self {
            buffer,
            index: 0,
            state: State::WaitingForFirstMagicNumber,
            retries,
        }
    }

    fn retry(&mut self) {
        if self.retries == 0 {
            self.state = State::Failed;
        } else {
            self.retries -= 1;
        }
    }

    fn magic_number_read(&mut self) {
        self.index = 2;
        self.buffer[0] = MN1;
        self.buffer[1] = MN2;
        self.state = State::Reading;
    }

    fn byte_read(&mut self, byte: u8) {
        self.buffer[self.index] = byte;
        self.index += 1;
        if self.index == self.buffer.len() {
            self.state = State::Finished;
        }
    }

    pub fn update<E>(&mut self, read_result: Result<u8, nb::Error<E>>) -> ReadStatus {
        match self.state {
            State::WaitingForFirstMagicNumber => match read_result {
                Ok(byte) if byte == MN1 => self.state = State::WaitingForSecondMagicNumber,
                _ => self.retry(),
            },
            State::WaitingForSecondMagicNumber => match read_result {
                Ok(byte) if byte == MN2 => self.magic_number_read(),
                Ok(_) => self.state = State::WaitingForFirstMagicNumber,
                _ => self.retry(),
            },
            State::Reading => match read_result {
                Ok(byte) => self.byte_read(byte),
                _ => self.retry(),
            },
            _ => {}
        };

        match self.state {
            State::WaitingForFirstMagicNumber
            | State::WaitingForSecondMagicNumber
            | State::Reading => ReadStatus::InProgress,
            State::Finished => ReadStatus::Finished,
            State::Failed => ReadStatus::Failed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_fsm(buffer: &mut [u8], retries: usize) -> ReadStateMachine<'_> {
        ReadStateMachine {
            buffer,
            index: 0,
            state: State::WaitingForFirstMagicNumber,
            retries,
        }
    }

    fn read_sequence(fsm: &mut ReadStateMachine, sequence: &[Result<u8, nb::Error<()>>]) {
        sequence.iter().for_each(|byte| {
            fsm.update(*byte);
        })
    }

    #[test]
    fn read_is_in_progress_until_finished() {
        let mut buffer = [0u8; 2];
        let mut fsm = create_test_fsm(&mut buffer, 0);
        assert_eq!(ReadStatus::InProgress, fsm.update::<()>(Ok(MN1)));
    }

    #[test]
    fn read_is_finished_when_required_number_of_bytes_read() {
        let mut buffer = [0u8; 4];
        let mut fsm = create_test_fsm(&mut buffer, 0);

        read_sequence(&mut fsm, &[Ok(MN1), Ok(MN2), Ok(0x11)]);

        assert_eq!(ReadStatus::Finished, fsm.update::<()>(Ok(0x33)));
        assert_eq!([MN1, MN2, 0x11, 0x33], buffer);
    }

    #[test]
    fn ignores_everything_until_magic_number_is_read() {
        let mut buffer = [0u8; 4];
        let mut fsm = create_test_fsm(&mut buffer, 5);

        read_sequence(
            &mut fsm,
            &[
                Ok(0x00),
                Ok(0x00),
                Ok(0x00),
                Ok(0x00),
                Ok(0x00),
                Ok(MN1),
                Ok(MN2),
                Ok(0x11),
            ],
        );

        assert_eq!(fsm.update::<()>(Ok(0x33)), ReadStatus::Finished);
        assert_eq!([MN1, MN2, 0x11, 0x33], buffer);
    }

    #[test]
    fn if_second_magic_number_is_not_received_just_after_the_first_one_reset() {
        let mut buffer = [0u8; 4];
        let mut fsm = create_test_fsm(&mut buffer, 2);

        read_sequence(
            &mut fsm,
            &[Ok(MN1), Ok(0x00), Ok(0x00), Ok(MN1), Ok(MN2), Ok(0x11)],
        );

        assert_eq!(fsm.update::<()>(Ok(0x33)), ReadStatus::Finished);
        assert_eq!(buffer, [MN1, MN2, 0x11, 0x33]);
    }

    #[test]
    fn if_magic_number_is_not_received_fail_after_n_retries() {
        let mut buffer = [0u8; 4];
        let mut fsm = create_test_fsm(&mut buffer, 3);

        read_sequence(&mut fsm, &[Ok(0x00), Err(nb::Error::WouldBlock), Ok(0x00)]);

        assert_eq!(fsm.update::<()>(Ok(0x33)), ReadStatus::Failed);
    }

    #[test]
    fn if_second_magic_number_is_not_received_fail_after_n_retries() {
        let mut buffer = [0u8; 4];
        let mut fsm = create_test_fsm(&mut buffer, 2);

        read_sequence(
            &mut fsm,
            &[
                Ok(MN1),
                Err(nb::Error::WouldBlock),
                Err(nb::Error::Other(())),
            ],
        );

        assert_eq!(
            fsm.update::<()>(Err(nb::Error::WouldBlock)),
            ReadStatus::Failed
        );
    }

    #[test]
    fn read_may_be_interrupted() {
        let mut buffer = [0u8; 4];
        let mut fsm = create_test_fsm(&mut buffer, 3);

        read_sequence(
            &mut fsm,
            &[
                Ok(MN1),
                Ok(MN2),
                Err(nb::Error::WouldBlock),
                Err(nb::Error::Other(())),
                Ok(0x11),
            ],
        );

        assert_eq!(ReadStatus::Finished, fsm.update::<()>(Ok(0x33)));
        assert_eq!([MN1, MN2, 0x11, 0x33], buffer);
    }

    #[test]
    fn if_interrupted_for_more_than_n_retries_fail() {
        let mut buffer = [0u8; 4];
        let mut fsm = create_test_fsm(&mut buffer, 2);

        read_sequence(
            &mut fsm,
            &[
                Ok(MN1),
                Ok(MN2),
                Err(nb::Error::WouldBlock),
                Err(nb::Error::WouldBlock),
            ],
        );

        assert_eq!(
            ReadStatus::Failed,
            fsm.update::<()>(Err(nb::Error::WouldBlock))
        );
    }
}
