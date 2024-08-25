
pub mod types;
use std::io::{Cursor, Read};
use thiserror::Error;

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
}

#[derive(Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum VarIntLength {
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
}

#[derive(Debug, Eq, PartialEq)]
pub struct VarInt {
    length: VarIntLength,
    bytes: [u8; 8],
    value: u64,
    raw_value: u64, // The value before masking the marker bit
}

impl VarIntLength {
    fn new(num_bytes: usize) -> Result<Self, EbmlError> {
        match num_bytes {
            1 => Ok(VarIntLength::One),
            2 => Ok(VarIntLength::Two),
            3 => Ok(VarIntLength::Three),
            4 => Ok(VarIntLength::Four),
            5 => Ok(VarIntLength::Five),
            6 => Ok(VarIntLength::Six),
            7 => Ok(VarIntLength::Seven),
            8 => Ok(VarIntLength::Eight),
            _ => Err(EbmlError::InvalidVarIntLength),
        }
    }

    fn maximum_value(&self) -> u64 {
        match self {
            VarIntLength::One => 127,
            VarIntLength::Two => 16383,
            VarIntLength::Three => 2097151,
            VarIntLength::Four => 268435455,
            VarIntLength::Five => 34359738367,
            VarIntLength::Six => 4398046511103,
            VarIntLength::Seven => 562949953421311,
            VarIntLength::Eight => 72057594037927935,
        }
    }
}

impl VarInt {
    #[inline]
    pub fn get_var_int(cursor: &mut Cursor<&[u8]>) -> Result<VarInt, EbmlError> {
        let (num_bytes, masked_first_byte, first_byte) = Self::get_var_int_length(cursor)?;
        if num_bytes > 8 || num_bytes == 0 {
            Err(EbmlError::InvalidVarIntLength)
        } else {
            let varint = Self::get_var_int_value(cursor, masked_first_byte, num_bytes)?;
            let mut raw_value = varint.clone();
            raw_value[8 - num_bytes] = first_byte;
            Ok(VarInt {
                length: VarIntLength::new(num_bytes)?,
                bytes: varint,
                raw_value: u64::from_be_bytes(raw_value),
                value: u64::from_be_bytes(varint),
            })
        }
    }

    /// Get the size of the varint and the value of the first byte with the market bit removed
    #[inline]
    fn get_var_int_length(cursor: &mut Cursor<&[u8]>) -> Result<(usize, u8, u8), EbmlError> {
        let mut bytes: [u8; 1] = [0; 1];

        if cursor.read(&mut bytes[..])? == 1 {
            let zeros = bytes[0].leading_zeros() as usize;
            if zeros == 8 {
                return Err(EbmlError::VarIntNoLength);
            }
            let num_bytes = zeros + 1;
            let shift = 8 - num_bytes;
            let masked_value = bytes[0] ^ 1u8 << shift; // Zero the marker bit

            Ok((num_bytes, masked_value, bytes[0]))
        } else {
            Err(EbmlError::NoData)
        }
    }

    #[inline]
    fn get_var_int_value(
        cursor: &mut Cursor<&[u8]>,
        first_byte: u8,
        num_bytes: usize,
    ) -> Result<[u8; 8], EbmlError> {
        let mut bytes: [u8; 8] = [0; 8];
        let first_index = 8 - num_bytes;
        bytes[first_index] = first_byte; // Put the first byte at the beginning of the big endian number in the array

        if num_bytes > 1 {
            // Read the number of bytes indicated by byte 0 into the end of the array (since it's big endian)
            let expected_read_amount = num_bytes - 1;
            if cursor.read(&mut bytes[first_index + 1..])? < expected_read_amount {
                return Err(EbmlError::VarIntEndedEarly);
            }
        }

        Ok(bytes)
    }

    #[inline]
    pub fn all_ones(&self) -> bool {
        self.value == self.length.maximum_value()
    }
}

/// Check if the varint is the most compact form possible without losing data
/// Specifically for ELEMENT ID
fn is_shortest_valid_element_id_length(varint: &VarInt) -> bool {
    match varint.length {
        VarIntLength::One => true,
        VarIntLength::Two => varint.value > VarIntLength::One.maximum_value(),
        VarIntLength::Three => varint.value > VarIntLength::Two.maximum_value(),
        VarIntLength::Four => varint.value > VarIntLength::Three.maximum_value(),
        VarIntLength::Five => varint.value > VarIntLength::Four.maximum_value(),
        VarIntLength::Six => varint.value > VarIntLength::Five.maximum_value(),
        VarIntLength::Seven => varint.value > VarIntLength::Six.maximum_value(),
        VarIntLength::Eight => varint.value > VarIntLength::Seven.maximum_value(),
    }
}

#[inline]
pub fn get_element_id(cursor: &mut Cursor<&[u8]>) -> Result<u64, EbmlError> {
    let varint = VarInt::get_var_int(cursor)?;
    if varint.length > VarIntLength::Four {
        // TODO: Can be configured in the EBMLMaxIDLength header field
        // BUT this only applies to the body - it's almost like there needs to be a parser for the header separate from the body.
        // Also, it's a 'guarantee' that the header and body are separated, so why parse them in a single loop?
        return Err(EbmlError::InvalidElementIdSize);
    }

    if varint.value == 0 {
        return Err(EbmlError::ElementIdAllZeros);
    }

    if varint.all_ones() {
        return Err(EbmlError::ElementIdAllOnes);
    }

    if is_shortest_valid_element_id_length(&varint) {
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
    //
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Read};

    use crate::{
        get_data_size, get_element_id,
        types::{BaseEbmlParser, EbmlHeaderElement, EbmlIterator, EbmlParser, MatroskaParser},
        VarInt, VarIntLength,
    };

    #[test]
    fn test() {
        use std::fs::File;

        use memmap2::Mmap;

        let file = File::open("test1.mkv").unwrap();

        let mmap = unsafe { Mmap::map(&file).unwrap() };

        let data = &mmap[..];
        let mut cursor = Cursor::new(data);

        
        {
          let parser = MatroskaParser::new(BaseEbmlParser::new());
            let iterator = EbmlIterator::new(parser, &mut cursor);
            for item in iterator {
                if let Err(e) = item {
                    println!("{e:#?}");
                    break;
                }
                match item.unwrap() {
                    crate::types::MatroskaElement::Chapter(chapter) => todo!(),
                    crate::types::MatroskaElement::EbmlElement(header_element) => {
                        match header_element {
                            crate::types::Element::Global(g) => match g {
                                crate::types::EbmlGlobalElement::Crc32(c) => {
                                    println!("Crc32: {:#?}", c)
                                }
                                crate::types::EbmlGlobalElement::Void(v) => {
                                    println!("Void: {:#?}", v)
                                }
                            },
                            crate::types::Element::Header(h) => match h {
                                EbmlHeaderElement::Ebml(_) => println!("head"),
                                EbmlHeaderElement::DocType(s) => println!("type: {:#?}", s),
                                EbmlHeaderElement::EbmlVersion(v) => println!("version: {:#?}", v),
                                EbmlHeaderElement::EbmlReadVersion(r) => {
                                    println!("read version: {:#?}", r)
                                }
                                EbmlHeaderElement::EbmlMaxIdLength(v) => {
                                    println!("EbmlMaxIdLength: {:#?}", v)
                                }
                                EbmlHeaderElement::EbmlMaxSizeLength(v) => {
                                    println!("EbmlMaxSizeLength: {:#?}", v)
                                }
                                EbmlHeaderElement::DocTypeVersion(v) => {
                                    println!("DocTypeVersion: {:#?}", v)
                                }
                                EbmlHeaderElement::DocTypeReadVersion(v) => {
                                    println!("DocTypeReadVersion: {:#?}", v)
                                }
                                EbmlHeaderElement::DocTypeExtension(v) => {
                                    println!("DocTypeExtension: {:#?}", v)
                                }
                                EbmlHeaderElement::DocTypeExtensionName(v) => {
                                    println!("DocTypeExtensionName: {:#?}", v)
                                }
                                EbmlHeaderElement::DocTypeExtensionVersion(v) => {
                                    println!("DocTypeExtensionVersion: {:#?}", v)
                                }
                            },
                            crate::types::Element::Unknown(id) => println!("Unknown: {id:X}"),
                        }
                    }
                }
            }
        }
    }

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
