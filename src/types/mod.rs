use crate::{get_data, get_data_size, get_element_id, EbmlError};
use compact_str::CompactString;
use std::io::{Cursor, Read, Seek};

pub struct EbmlElement {
    pub id: u64,
    pub size: VarInt,
    pub length: u64,
}

impl TryFrom<&mut Cursor<&[u8]>> for EbmlElement {
    type Error = EbmlError;

    fn try_from(cursor: &mut Cursor<&[u8]>) -> Result<Self, Self::Error> {
        let start = cursor.position();
        let id = match get_element_id(cursor) {
            Ok(v) => v,
            Err(_) => return Err(EbmlError::ElementIdAllOnes),
        };
        let size = match get_data_size(cursor) {
            Ok(v) => v,
            Err(_) => return Err(EbmlError::ElementIdAllOnes),
        };
        let end = cursor.position();
        Ok(EbmlElement {
            id,
            size,
            length: end - start,
        })
    }
}

impl EbmlElement {
  #[inline]
  pub fn get_data<'a>(&self, cursor: &mut Cursor<&'a [u8]>) -> Result<&'a [u8], EbmlError> {
    get_data(self.size.value, cursor)
  }

  pub fn get_child<'a>(&self, cursor: &mut Cursor<&'a [u8]>) -> Result<EbmlElement, EbmlError> {
    EbmlElement::try_from(&mut *cursor)
  }
}

#[derive(Debug, Clone, Default)]
pub struct EbmlHeader {
    version: Option<EbmlUnsignedInteger>,
    read_version: Option<EbmlUnsignedInteger>,
    max_id_length: Option<EbmlUnsignedInteger>,
    max_size_length: Option<EbmlUnsignedInteger>,
    doc_type: Option<EbmlString>,
    doc_type_version: Option<EbmlUnsignedInteger>,
    doc_type_read_version: Option<EbmlUnsignedInteger>,
    doc_type_extensions: Option<Vec<DocTypeExtension>>,
}

impl TryFrom<&mut Cursor<&[u8]>> for EbmlHeader {
    type Error = EbmlError;

    fn try_from(cursor: &mut Cursor<&[u8]>) -> Result<Self, Self::Error> {
        let mut header = EbmlHeader::default();

        let ebml = EbmlElement::try_from(&mut *cursor)?;
        if ebml.id != Ebml::ID {
            return Err(EbmlError::InvalidElement(format!(
                "Invalid element id: {:X}",
                ebml.id
            )));
        }

        while cursor.position() < ebml.size.value + ebml.length {
            let element = EbmlElement::try_from(&mut *cursor)?;
            match element.id {
                EbmlVersion::ID => {
                    let data = get_data(element.size.value, &mut *cursor)?;
                    header.version = Some(EbmlUnsignedInteger::new(data)?);
                }
                DocType::ID => {
                    let data = get_data(element.size.value, &mut *cursor)?;
                    header.doc_type = Some(EbmlString::new(data)?);
                }
                DocTypeVersion::ID => {
                    let data = get_data(element.size.value, &mut *cursor)?;
                    header.doc_type_version = Some(EbmlUnsignedInteger::new(data)?);
                }
                DocTypeReadVersion::ID => {
                    let data = get_data(element.size.value, &mut *cursor)?;
                    header.doc_type_read_version = Some(EbmlUnsignedInteger::new(data)?);
                }
                EbmlReadVersion::ID => {
                    let data = get_data(element.size.value, &mut *cursor)?;
                    header.read_version = Some(EbmlUnsignedInteger::new(data)?);
                }
                EbmlMaxIdLength::ID => {
                    let data = get_data(element.size.value, &mut *cursor)?;
                    header.max_id_length = Some(EbmlUnsignedInteger::new(data)?);
                }
                EbmlMaxSizeLength::ID => {
                    let data = get_data(element.size.value, &mut *cursor)?;
                    header.max_size_length = Some(EbmlUnsignedInteger::new(data)?);
                }
                DocTypeExtension::ID => {
                    let first_element = EbmlElement::try_from(&mut *cursor)?;
                    let first_data = get_data(first_element.size.value, &mut *cursor)?;
                    let second_element = EbmlElement::try_from(&mut *cursor)?;
                    let second_data = get_data(second_element.size.value, &mut *cursor)?;

                    let extension;
                    if first_element.id == DocTypeExtensionName::ID
                        && second_element.id == DocTypeExtensionVersion::ID
                    {
                        let name = EbmlString::new(first_data)?;
                        let version = EbmlUnsignedInteger::new(second_data)?;
                        extension = DocTypeExtension::new(name, version);
                    } else if first_element.id == DocTypeExtensionVersion::ID
                        && second_element.id == DocTypeExtensionName::ID
                    {
                        let name = EbmlString::new(second_data)?;
                        let version = EbmlUnsignedInteger::new(first_data)?;
                        extension = DocTypeExtension::new(name, version);
                    } else {
                        return Err(EbmlError::InvalidElement(format!(
                            "Unrecognized one or two element ids in ebml header: {:X}, {:X}",
                            first_element.id, second_element.id
                        )));
                    }

                    match header.doc_type_extensions {
                        Some(ref mut v) => v.push(extension),
                        None => {
                            header.doc_type_extensions = Some(vec![extension]);
                        }
                    }
                }
                _ => {
                    return Err(EbmlError::InvalidElement(format!(
                        "EBML Header contains invalid element ID {:X}",
                        element.id
                    )))
                }
            }
        }

        Ok(header)
    }
}

#[derive(Debug, Clone)]
pub struct EbmlString {
    value: CompactString,
    /// The position of the end of the string (null terminator or end)
    end: usize,
    /// The entire length of the string
    full_end: usize,
}

impl EbmlString {
    /// Creates the string starting from the current position of the cursor up to the size
    pub fn new(data: &[u8]) -> Result<Self, EbmlError> {
        let result = {
            let mut end = data.len();
            for (ind, b) in data.iter().enumerate() {
                if (*b < 0x20 || *b > 0x7E) && *b != 0 {
                    return Err(EbmlError::InvalidString);
                } else if *b == 0 {
                    end = ind;
                    break;
                }
            }

            Ok(Self {
                value: CompactString::from_utf8(&data[..end])
                    .map_err(|_e| EbmlError::InvalidString)?,
                full_end: data.len(),
                end,
            })
        };
        result
    }
}

#[derive(Debug, Clone)]
pub struct EbmlUnsignedInteger {
    value: u64,
}

impl EbmlUnsignedInteger {
    pub fn new(data: &[u8]) -> Result<Self, EbmlError> {
        if data.len() > 8 {
            return Err(EbmlError::OverMaximumSize(8));
        }
        let mut bytes: [u8; 8] = [0u8; 8];
        for (i, b) in data.iter().rev().enumerate() {
            bytes[i] = *b;
        }

        bytes.reverse();

        Ok(Self {
            value: u64::from_be_bytes(bytes),
        })
    }
}

#[derive(Debug, Clone)]
pub struct DocType {
    size: VarInt,
    value: EbmlString,
}

impl DocType {
    // range: >0
    pub const ID: u64 = 0x4282;
    const MIN_OCCURS: u8 = 1;
    const MAX_OCCURS: u8 = 1;
    pub fn new(size: VarInt, value: EbmlString) -> Self {
        DocType { size, value }
    }
}

#[derive(Debug, Clone)]
pub struct DocTypeVersion {
    size: VarInt,
    value: EbmlUnsignedInteger,
}

impl DocTypeVersion {
    pub const ID: u64 = 0x4287;
    const DEFAULT: u64 = 1; // range: >0
    const MIN_OCCURS: u8 = 1;
    const MAX_OCCURS: u8 = 1;
    pub fn new(size: VarInt, value: EbmlUnsignedInteger) -> Self {
        DocTypeVersion { size, value }
    }
}

#[derive(Debug, Clone)]
pub struct DocTypeReadVersion {
    size: VarInt,
    value: EbmlUnsignedInteger,
}

impl DocTypeReadVersion {
    pub const ID: u64 = 0x4285;
    const DEFAULT: u64 = 1; // range: >0
    const MIN_OCCURS: u8 = 1;
    const MAX_OCCURS: u8 = 1;
    pub fn new(size: VarInt, value: EbmlUnsignedInteger) -> Self {
        DocTypeReadVersion { size, value }
    }
}

#[derive(Debug, Clone)]
pub struct DocTypeExtension {
    name: EbmlString,
    version: EbmlUnsignedInteger,
}

impl DocTypeExtension {
    pub const ID: u64 = 0x4281;
    const MIN_OCCURS: u8 = 0;

    pub fn new(name: EbmlString, version: EbmlUnsignedInteger) -> Self {
        Self { name, version }
    }
}

#[derive(Debug, Clone)]
pub struct DocTypeExtensionName {
    size: VarInt,
    value: EbmlString,
}

impl DocTypeExtensionName {
    pub const ID: u64 = 0x4283;
    //const LENGTH: greater than 1
    const MIN_OCCURS: u8 = 1;
    const MAX_OCCURS: u8 = 1;
    pub fn new(size: VarInt, value: EbmlString) -> Self {
        DocTypeExtensionName { size, value }
    }
}

#[derive(Debug, Clone)]
pub struct DocTypeExtensionVersion {
    size: VarInt,
    value: EbmlUnsignedInteger,
}

impl DocTypeExtensionVersion {
    pub const ID: u64 = 0x4284; // range not 0
    const MIN_OCCURS: u8 = 1;
    const MAX_OCCURS: u8 = 1;
    pub fn new(size: VarInt, value: EbmlUnsignedInteger) -> Self {
        DocTypeExtensionVersion { size, value }
    }
}

#[derive(Debug, Clone)]
pub struct EbmlVersion {
    size: VarInt,
    value: EbmlUnsignedInteger,
}

impl EbmlVersion {
    pub const ID: u64 = 0x4286;
    const DEFAULT: u64 = 1; // range: >0
    const MIN_OCCURS: u8 = 1;
    const MAX_OCCURS: u8 = 1;
    pub fn new(size: VarInt, value: EbmlUnsignedInteger) -> Self {
        EbmlVersion { size, value }
    }
}

#[derive(Debug, Clone)]
pub struct EbmlReadVersion {
    size: VarInt,
    value: EbmlUnsignedInteger,
}

impl EbmlReadVersion {
    pub const ID: u64 = 0x42F7;
    const DEFAULT: u8 = 1; // Range: ==1
    const MIN_OCCURS: u8 = 1;
    const MAX_OCCURS: u8 = 1;
    pub fn new(size: VarInt, value: EbmlUnsignedInteger) -> Self {
        EbmlReadVersion { size, value }
    }
}

#[derive(Debug, Clone)]
pub struct EbmlMaxIdLength {
    size: VarInt,
    value: EbmlUnsignedInteger,
}

impl EbmlMaxIdLength {
    pub const ID: u64 = 0x42F2;
    const DEFAULT: u8 = 4; // range: >=4
    const MIN_OCCURS: u8 = 1;
    const MAX_OCCURS: u8 = 1;
    pub fn new(size: VarInt, value: EbmlUnsignedInteger) -> Self {
        EbmlMaxIdLength { size, value }
    }
}

#[derive(Debug, Clone)]
pub struct EbmlMaxSizeLength {
    size: VarInt,
    value: EbmlUnsignedInteger,
}

impl EbmlMaxSizeLength {
    pub const ID: u64 = 0x42F3;
    const DEFAULT: u8 = 8; // range: >0
    const MIN_OCCURS: u8 = 1;
    const MAX_OCCURS: u8 = 1;
    pub fn new(size: VarInt, value: EbmlUnsignedInteger) -> Self {
        EbmlMaxSizeLength { size, value }
    }
}

pub struct Ebml {
    size: VarInt,
}

impl Ebml {
    pub const ID: u64 = 0x1A45DFA3;
}

#[derive(Debug, Clone)]
pub struct EbmlBinary<'a> {
    index: u64,
    size: u64,
    data: &'a [u8],
}

impl<'a> EbmlBinary<'a> {
    pub fn new(size: &VarInt, cursor: &mut Cursor<&'a [u8]>) -> Result<Self, EbmlError> {
        let index = cursor.position();
        let data_ref = &cursor.get_ref()[index as usize..];
        if data_ref.len() < size.value as usize {
            return Err(EbmlError::NoData);
        }
        let data = &data_ref[..size.value as usize];
        Ok(Self {
            size: size.value,
            index,
            data,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Crc32<'a> {
    size: VarInt,
    binary: EbmlBinary<'a>,
}

impl<'a> Crc32<'a> {
    const ID: u64 = 0xBF;
    // length: 4
    // type: binary
    const MIN_OCCURS: u8 = 0; // within parent
    const MAX_OCCURS: u8 = 1;
    pub fn new(size: VarInt, binary: EbmlBinary<'a>) -> Self {
        Crc32 { size, binary }
    }
}

#[derive(Debug, Clone)]
pub struct Void {
    size: VarInt,
}

impl Void {
    const ID: u64 = 0xEC;
    const MIN_OCCURS: u8 = 0;
    pub fn new(size: VarInt) -> Self {
        Void { size }
    }
}

#[derive(Debug, Eq, PartialEq, PartialOrd, Ord, Clone)]
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

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct VarInt {
    pub length: VarIntLength,
    bytes: [u8; 8],
    pub value: u64,
    pub raw_value: u64, // The value before masking the marker bit
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

    /// Check if the varint is the most compact form possible without losing data
    /// Specifically for ELEMENT ID
    pub fn is_shortest_valid_element_id_length(&self) -> bool {
        match self.length {
            VarIntLength::One => true,
            VarIntLength::Two => self.value > VarIntLength::One.maximum_value(),
            VarIntLength::Three => self.value > VarIntLength::Two.maximum_value(),
            VarIntLength::Four => self.value > VarIntLength::Three.maximum_value(),
            VarIntLength::Five => self.value > VarIntLength::Four.maximum_value(),
            VarIntLength::Six => self.value > VarIntLength::Five.maximum_value(),
            VarIntLength::Seven => self.value > VarIntLength::Six.maximum_value(),
            VarIntLength::Eight => self.value > VarIntLength::Seven.maximum_value(),
        }
    }
}
