#[allow(unused)]
mod types;
use std::io::{Cursor, Seek};
use thiserror::Error;

pub use types::*;

#[derive(Error, Debug)]
pub enum EbmlError {
    #[error("An error occurred during the IO operation: {0}")]
    IoError(#[from] std::io::Error),
    #[error("No more data available to be read")]
    NoData,
    #[error("Invalid varint length, reached end of data")]
    VarIntEndedEarly,
    #[error("No marker bit to determine var int length")]
    VarIntNoLength,
    #[error("Element ID used more octets than allowed")]
    InvalidElementIdSize,
    #[error("Var int is too large")]
    VarIntTooLarge,
    #[error("Element ID all Ones")]
    ElementIdAllOnes,
    #[error("Element ID all zeros")]
    ElementIdAllZeros,
    #[error("Var int length value invalid")]
    InvalidVarIntLength,
    #[error("Element IDs must be encoded in the shortest size possible")]
    ElementIdLongerThanNeeded,
    #[error("Unknown header element, id: {0:X}, size: {1:?}")]
    UnknownHeaderElement(u64, VarInt),
    #[error("The bytes are not a valid matroska string")]
    InvalidString,
    #[error("An element/data that must be sized had an unknown size: {0}")]
    MustBeSized(&'static str),
    #[error("Invalid Element: {0}")]
    InvalidElement(String),
    #[error("Over maximum size: {0}")]
    OverMaximumSize(usize),
    #[error("Couldn't Seek")]
    CouldntSeek,
}

#[inline]
pub fn get_element_id(cursor: &mut Cursor<&[u8]>) -> Result<u64, EbmlError> {
    let varint = VarInt::get_var_int(cursor)?;
    if varint.length > VarIntLength::Four {
        // TODO: Can be configured in the EBMLMaxIDLength header field
        return Err(EbmlError::InvalidElementIdSize);
    }

    if varint.value == 0 {
        return Err(EbmlError::ElementIdAllZeros);
    }

    if varint.all_ones() {
        return Err(EbmlError::ElementIdAllOnes);
    }

    if varint.is_shortest_valid_element_id_length() {
        Ok(varint.raw_value)
    } else {
        Err(EbmlError::ElementIdLongerThanNeeded)
    }
}

#[inline]
pub fn get_data_size(cursor: &mut Cursor<&[u8]>) -> Result<VarInt, EbmlError> {
    // 1-8 unless EBMLMaxSizeLength
    VarInt::get_var_int(cursor)
    // can have all bits set to zero unless the element ID mandates otherwise
    // if all zeros (aka empty element) and there's a default, default should be returned
    // if all bits are one, the size of the element is unknown
    // spec: Only a Master Element is allowed to be of unknown size, and it can only be so if the unknownsizeallowed attribute of its EBML Schema is set to true
}

#[inline]
pub fn get_data<'a>(size: u64, cursor: &mut Cursor<&'a [u8]>) -> Result<&'a [u8], EbmlError> {
  let start = cursor.position() as usize;
  let end = start + size as usize;
  let data = &cursor.get_ref()[start..end];
  cursor
      .seek_relative(data.len() as i64)
      .map_err(|_| EbmlError::CouldntSeek)?;
  Ok(data)
}



#[cfg(test)]
mod tests {
    use crate::VarIntLength;

    #[test]
    fn test_length_order() {
        assert!(VarIntLength::One < VarIntLength::Two);
    }

    mod varint {

        use std::io::Cursor;

        use crate::{EbmlError, VarInt};

        #[test]
        fn test_2_byte() {
            let data = [0b01000010, 0b00000001];
            let mut c = Cursor::new(&data[..]);
            let vi = VarInt::get_var_int(&mut c).unwrap();
            assert_eq!(vi.value, 513);
        }

        #[test]
        fn test_back_to_back() {
            let data = [0b01000010, 0b00000001, 0b10000001];
            let mut c = Cursor::new(&data[..]);
            let vi = VarInt::get_var_int(&mut c).unwrap();
            assert_eq!(vi.value, 513);
            let vi = VarInt::get_var_int(&mut c).unwrap();
            assert_eq!(vi.value, 1);
        }

        #[test]
        fn test_larger_context() {
            let data = [
                0b01000010, 0b00000001, 0b10000001, 0b10000001, 0b10000001, 0b10000001, 0b10000001,
                0b10000001,
            ];
            let mut c = Cursor::new(&data[..]);
            let vi = VarInt::get_var_int(&mut c).unwrap();
            assert_eq!(vi.value, 513);
        }

        #[test]
        fn test_reserved_space() {
            let data = [0b00000001, 0b0, 0b0, 0b0, 0b0, 0b0, 0b0, 0b00000011];
            let mut c = Cursor::new(&data[..]);
            let vi = VarInt::get_var_int(&mut c).unwrap();
            assert_eq!(vi.value, 3);
        }

        #[test]
        fn test_simple() {
            let data = [0b10000010];
            let mut c = Cursor::new(&data[..]);
            let vi = VarInt::get_var_int(&mut c).unwrap();
            assert_eq!(vi.value, 2);
        }

        #[test]
        fn test_get_var_int_incorrect() {
            let data = [0b01000010];
            let mut c = Cursor::new(&data[..]);
            match VarInt::get_var_int(&mut c) {
                Ok(_) => panic!("Should have returned an error"),
                Err(e) => match e {
                    EbmlError::VarIntEndedEarly => {}
                    _ => panic!("Incorrect error: {:#?}", e),
                },
            }
        }
    }

    mod element_id {
        use crate::{get_element_id, EbmlError};
        use std::io::Cursor;

        #[test]
        fn test_basic() {
            let data = [0b00011000, 0b0, 0b0, 0b0];
            let mut c = Cursor::new(&data[..]);
            let id = get_element_id(&mut c).unwrap();
            assert_eq!(id, 402653184);
        }

        #[test]
        fn test_invalid_ids() {
            let data = [u8::MAX];
            let mut c = Cursor::new(&data[..]);
            match get_element_id(&mut c) {
                Ok(_) => panic!("Should have returned an error"),
                Err(e) => match e {
                    EbmlError::ElementIdAllOnes => {}
                    _ => panic!("Incorrect error: {:#?}", e),
                },
            }

            let data = [0b10000000];
            let mut c = Cursor::new(&data[..]);
            match get_element_id(&mut c) {
                Ok(_) => panic!("Should have returned an error"),
                Err(e) => match e {
                    EbmlError::ElementIdAllZeros => {}
                    _ => panic!("Incorrect error: {:#?}", e),
                },
            }
        }

        #[test]
        fn test_too_large() {
            let data = [0b00001000, 0b0, 0b0, 0b0, 0b1];
            let mut c = Cursor::new(&data[..]);
            match get_element_id(&mut c) {
                Ok(_) => panic!("Should have returned an error"),
                Err(e) => match e {
                    EbmlError::InvalidElementIdSize => {}
                    _ => panic!("Incorrect error: {:#?}", e),
                },
            }
        }

        #[test]
        fn test_all_ones() {
            let data = [0b01111111, 0b11111111];
            let mut c = Cursor::new(&data[..]);
            match get_element_id(&mut c) {
                Ok(_) => panic!("Should have returned an error"),
                Err(e) => match e {
                    EbmlError::ElementIdAllOnes => {}
                    _ => panic!("Incorrect error: {:#?}", e),
                },
            }
        }

        #[test]
        fn test_all_zeros() {
            let data = [0b01000000, 0b0];
            let mut c = Cursor::new(&data[..]);
            match get_element_id(&mut c) {
                Ok(_) => panic!("Should have returned an error"),
                Err(e) => match e {
                    EbmlError::ElementIdAllZeros => {}
                    _ => panic!("Incorrect error: {:#?}", e),
                },
            }
        }
    }
}