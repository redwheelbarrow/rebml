

use std::io::{Cursor, Seek};

use bytes::Buf;
use compact_str::CompactString;

use crate::{get_data_size, get_element_id, EbmlError, VarInt};

pub struct DataWrapper<'a>(&'a [u8]);

impl<'a> std::fmt::Debug for DataWrapper<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("DataWrapper")
            //.field(&self.0)
            .finish()
    }
}

#[derive(Debug)]
pub struct EbmlString<'a> {
    value: CompactString,
    /// This includes the full range of the string, even if there is a null terminator somewhere
    data: DataWrapper<'a>,
    /// The position of the end of the string (null terminator or end)
    end: usize,
}

impl<'a> EbmlString<'a> {
    /// Creates the string starting from the current position of the cursor up to the size
    pub fn new(size: &VarInt, cursor: &mut Cursor<&'a [u8]>) -> Result<Self, EbmlError> {
        let result = {
            let absolute_start = cursor.position() as usize;
            let absolute_end = size.value as usize + absolute_start;
            let data = &cursor.get_ref()[absolute_start..absolute_end];
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
                data: DataWrapper(data),
                end,
            })
        };
        let _ = cursor.advance(size.value as usize);
        result
    }

    pub fn get_str(&'a self) -> &'a CompactString {
        &self.value
    }
}

#[derive(Debug)]
pub struct EbmlUnsignedInteger<'a> {
    value: u64,
    data: DataWrapper<'a>,
}

impl<'a> EbmlUnsignedInteger<'a> {
    pub fn new(size: &VarInt, cursor: &mut Cursor<&'a [u8]>) -> Result<Self, EbmlError> {
        let absolute_start = cursor.position() as usize;
        let absolute_end = size.value as usize + absolute_start;
        let data = &cursor.get_ref()[absolute_start..absolute_end];
        cursor.advance(data.len());
        let mut bytes: [u8; 8] = [0u8; 8];
        for (i, b) in data.iter().rev().enumerate() {
            bytes[i] = *b;
        }

        bytes.reverse(); // could just rotate??

        let value = u64::from_be_bytes(bytes);
        Ok(Self {
            value,
            data: DataWrapper(data),
        })
    }
}

pub enum EbmlElementId {
    // Crc32,
    // Void,
    Ebml,
    EbmlVersion,
    EbmlReadVersion,
    EbmlMaxIdLength,
    EbmlMaxSizeLength,
    DocType,
    DocTypeVersion,
    DocTypeReadVersion,
    DocTypeExtension,
    DocTypeExtensionName,
    DocTypeExtensionVersion,
    Crc32,
    Void,
    Unknown(u64),
}

impl From<u64> for EbmlElementId {
    fn from(value: u64) -> Self {
        match value {
            Ebml::ID => EbmlElementId::Ebml,
            EbmlVersion::ID => EbmlElementId::EbmlVersion,
            EbmlReadVersion::ID => EbmlElementId::EbmlReadVersion,
            EbmlMaxIdLength::ID => EbmlElementId::EbmlMaxIdLength,
            EbmlMaxSizeLength::ID => EbmlElementId::EbmlMaxSizeLength,
            DocType::ID => EbmlElementId::DocType,
            DocTypeVersion::ID => EbmlElementId::DocTypeVersion,
            DocTypeReadVersion::ID => EbmlElementId::DocTypeReadVersion,
            DocTypeExtension::ID => EbmlElementId::DocTypeExtension,
            DocTypeExtensionName::ID => EbmlElementId::DocTypeExtensionName,
            DocTypeExtensionVersion::ID => EbmlElementId::DocTypeExtensionVersion,
            Crc32::ID => EbmlElementId::Crc32,
            Void::ID => EbmlElementId::Void,
            _ => EbmlElementId::Unknown(value),
        }
    }
}

impl From<EbmlElementId> for u64 {
    fn from(value: EbmlElementId) -> Self {
        match value {
            EbmlElementId::Ebml => Ebml::ID,
            EbmlElementId::EbmlVersion => EbmlVersion::ID,
            EbmlElementId::EbmlReadVersion => EbmlReadVersion::ID,
            EbmlElementId::EbmlMaxIdLength => EbmlMaxIdLength::ID,
            EbmlElementId::EbmlMaxSizeLength => EbmlMaxSizeLength::ID,
            EbmlElementId::DocType => DocType::ID,
            EbmlElementId::DocTypeVersion => DocTypeVersion::ID,
            EbmlElementId::DocTypeReadVersion => DocTypeReadVersion::ID,
            EbmlElementId::Unknown(v) => v,
            EbmlElementId::DocTypeExtension => DocTypeExtension::ID,
            EbmlElementId::DocTypeExtensionName => DocTypeExtensionName::ID,
            EbmlElementId::DocTypeExtensionVersion => DocTypeExtensionVersion::ID,
            EbmlElementId::Crc32 => Crc32::ID,
            EbmlElementId::Void => Void::ID,
        }
    }
}

#[derive(Debug)]
pub struct DocType<'a> {
    size: VarInt,
    value: EbmlString<'a>,
}

impl<'a> DocType<'a> {
    // range: >0
    const ID: u64 = 0x4282;
    const MIN_OCCURS: u8 = 1;
    const MAX_OCCURS: u8 = 1;
    pub fn new(size: VarInt, value: EbmlString<'a>) -> Self {
        DocType { size, value }
    }
}

#[derive(Debug)]
pub struct DocTypeVersion<'a> {
    size: VarInt,
    value: EbmlUnsignedInteger<'a>,
}

impl<'a> DocTypeVersion<'a> {
    const ID: u64 = 0x4287;
    const DEFAULT: u64 = 1; // range: >0
    const MIN_OCCURS: u8 = 1;
    const MAX_OCCURS: u8 = 1;
    pub fn new(size: VarInt, value: EbmlUnsignedInteger<'a>) -> Self {
        DocTypeVersion { size, value }
    }
}

#[derive(Debug)]
pub struct DocTypeReadVersion<'a> {
    size: VarInt,
    value: EbmlUnsignedInteger<'a>,
}

impl<'a> DocTypeReadVersion<'a> {
    const ID: u64 = 0x4285;
    const DEFAULT: u64 = 1; // range: >0
    const MIN_OCCURS: u8 = 1;
    const MAX_OCCURS: u8 = 1;
    pub fn new(size: VarInt, value: EbmlUnsignedInteger<'a>) -> Self {
        DocTypeReadVersion { size, value }
    }
}

trait MasterElement {}
impl MasterElement for DocTypeExtension {}

#[derive(Debug)]
pub struct DocTypeExtension {
    size: VarInt,
    index: usize,
}

impl<'a> DocTypeExtension {
    const ID: u64 = 0x4281;
    const MIN_OCCURS: u8 = 0;
    pub fn new(size: VarInt, index: usize) -> Self {
        DocTypeExtension { size, index }
    }
}

#[derive(Debug)]
pub struct DocTypeExtensionName<'a> {
    size: VarInt,
    value: EbmlString<'a>,
}

impl<'a> DocTypeExtensionName<'a> {
    const ID: u64 = 0x4283;
    //const LENGTH: greater than 1
    const MIN_OCCURS: u8 = 1;
    const MAX_OCCURS: u8 = 1;
    pub fn new(size: VarInt, value: EbmlString<'a>) -> Self {
        DocTypeExtensionName { size, value }
    }
}

#[derive(Debug)]
pub struct DocTypeExtensionVersion<'a> {
    size: VarInt,
    value: EbmlUnsignedInteger<'a>,
}

impl<'a> DocTypeExtensionVersion<'a> {
    const ID: u64 = 0x4284; // range not 0
    const MIN_OCCURS: u8 = 1;
    const MAX_OCCURS: u8 = 1;
    pub fn new(size: VarInt, value: EbmlUnsignedInteger<'a>) -> Self {
        DocTypeExtensionVersion { size, value }
    }
}

#[derive(Debug)]
pub struct EbmlVersion<'a> {
    size: VarInt,
    value: EbmlUnsignedInteger<'a>,
}

impl<'a> EbmlVersion<'a> {
    const ID: u64 = 0x4286;
    const DEFAULT: u64 = 1; // range: >0
    const MIN_OCCURS: u8 = 1;
    const MAX_OCCURS: u8 = 1;
    pub fn new(size: VarInt, value: EbmlUnsignedInteger<'a>) -> Self {
        EbmlVersion { size, value }
    }
}

#[derive(Debug)]
pub struct EbmlReadVersion<'a> {
    size: VarInt,
    value: EbmlUnsignedInteger<'a>,
}

impl<'a> EbmlReadVersion<'a> {
    const ID: u64 = 0x42F7;
    const DEFAULT: u8 = 1; // Range: ==1
    const MIN_OCCURS: u8 = 1;
    const MAX_OCCURS: u8 = 1;
    pub fn new(size: VarInt, value: EbmlUnsignedInteger<'a>) -> Self {
        EbmlReadVersion { size, value }
    }
}

#[derive(Debug)]
pub struct EbmlMaxIdLength<'a> {
    size: VarInt,
    value: EbmlUnsignedInteger<'a>,
}

impl<'a> EbmlMaxIdLength<'a> {
    const ID: u64 = 0x42F2;
    const DEFAULT: u8 = 4; // range: >=4
    const MIN_OCCURS: u8 = 1;
    const MAX_OCCURS: u8 = 1;
    pub fn new(size: VarInt, value: EbmlUnsignedInteger<'a>) -> Self {
        EbmlMaxIdLength { size, value }
    }
}

#[derive(Debug)]
pub struct EbmlMaxSizeLength<'a> {
    size: VarInt,
    value: EbmlUnsignedInteger<'a>,
}

impl<'a> EbmlMaxSizeLength<'a> {
    const ID: u64 = 0x42F3;
    const DEFAULT: u8 = 8; // range: >0
    const MIN_OCCURS: u8 = 1;
    const MAX_OCCURS: u8 = 1;
    pub fn new(size: VarInt, value: EbmlUnsignedInteger<'a>) -> Self {
        EbmlMaxSizeLength { size, value }
    }
}

pub struct Ebml {
    size: VarInt,
}

impl Ebml {
    const ID: u64 = 0x1A45DFA3;
}

#[derive(Debug)]
pub struct EbmlBinary<'a> {
    data: DataWrapper<'a>,
    index: u64,
    size: u64,
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
            data: DataWrapper(data),
        })
    }
}

#[derive(Debug)]
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

#[derive(Debug)]
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

pub enum EbmlGlobalElement<'a> {
    Crc32(Crc32<'a>),
    Void(Void),
}

pub enum Element<'a> {
    Global(EbmlGlobalElement<'a>),
    Header(EbmlHeaderElement<'a>),
    Unknown(u64),
}

pub enum EbmlHeaderElement<'a> {
    Ebml(Ebml),
    EbmlVersion(EbmlVersion<'a>),
    EbmlReadVersion(EbmlReadVersion<'a>),
    EbmlMaxIdLength(EbmlMaxIdLength<'a>),
    EbmlMaxSizeLength(EbmlMaxSizeLength<'a>),
    DocType(DocType<'a>),
    DocTypeVersion(DocTypeVersion<'a>),
    DocTypeReadVersion(DocTypeReadVersion<'a>),
    DocTypeExtension(DocTypeExtension),
    DocTypeExtensionName(DocTypeExtensionName<'a>),
    DocTypeExtensionVersion(DocTypeExtensionVersion<'a>),
}

////////////////////////// parser
pub trait EbmlParser<'a> {
    type Output;
    fn next(
        &self,
        id: u64,
        size: VarInt,
        cursor: &mut Cursor<&'a [u8]>,
    ) -> Result<Self::Output, EbmlError>;
}

pub struct EbmlIterator<'a, T: EbmlParser<'a>> {
    parser: T,
    cursor: &'a mut Cursor<&'a [u8]>,
}

impl<'a, T: EbmlParser<'a>> EbmlIterator<'a, T> {
    pub fn new(parser: T, cursor: &'a mut Cursor<&'a [u8]>) -> Self {
        EbmlIterator { parser, cursor }
    }
}

impl<'a, T: EbmlParser<'a>> Iterator for EbmlIterator<'a, T> {
    type Item = Result<T::Output, EbmlError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cursor.has_remaining() {
            let id = match get_element_id(self.cursor) {
                Ok(v) => v,
                Err(e) => return Some(Err(e)),
            };
            let size = match get_data_size(self.cursor) {
                Ok(v) => v,
                Err(e) => return Some(Err(e)),
            };

            Some(self.parser.next(id, size, self.cursor))
        } else {
            None
        }
    }
}

pub struct BaseEbmlParser {}

impl BaseEbmlParser {
    pub fn new() -> Self {
        BaseEbmlParser {}
    }
}

impl<'a> EbmlParser<'a> for BaseEbmlParser {
    type Output = Element<'a>;
    fn next(
        &self,
        id: u64,
        size: VarInt,
        cursor: &mut Cursor<&'a [u8]>,
    ) -> Result<Self::Output, EbmlError> {
        // TODO: during WRITING, the size might not be known, and if unknownsizeallowed is true, this is allowed. But once it is saved, it must be set
        let header_element = match EbmlElementId::from(id) {
            EbmlElementId::Ebml => Element::Header(EbmlHeaderElement::Ebml(Ebml { size })),
            EbmlElementId::EbmlVersion => {
                let value = EbmlUnsignedInteger::new(&size, cursor)?;
                let version = EbmlVersion::new(size, value);
                Element::Header(EbmlHeaderElement::EbmlVersion(version))
            }
            EbmlElementId::EbmlReadVersion => {
                let value = EbmlUnsignedInteger::new(&size, cursor)?;
                let version = EbmlReadVersion::new(size, value);
                Element::Header(EbmlHeaderElement::EbmlReadVersion(version))
            }
            EbmlElementId::EbmlMaxIdLength => {
                let value = EbmlUnsignedInteger::new(&size, cursor)?;
                Element::Header(EbmlHeaderElement::EbmlMaxIdLength(EbmlMaxIdLength::new(
                    size, value,
                )))
            }
            EbmlElementId::EbmlMaxSizeLength => {
                let value = EbmlUnsignedInteger::new(&size, cursor)?;
                Element::Header(EbmlHeaderElement::EbmlMaxSizeLength(
                    EbmlMaxSizeLength::new(size, value),
                ))
            }
            EbmlElementId::DocType => {
                let value = EbmlString::new(&size, cursor)?;
                Element::Header(EbmlHeaderElement::DocType(DocType::new(size, value)))
            }
            EbmlElementId::DocTypeVersion => {
                let value = EbmlUnsignedInteger::new(&size, cursor)?;
                Element::Header(EbmlHeaderElement::DocTypeVersion(DocTypeVersion::new(
                    size, value,
                )))
            }
            EbmlElementId::DocTypeReadVersion => {
                let value = EbmlUnsignedInteger::new(&size, cursor)?;
                Element::Header(EbmlHeaderElement::DocTypeReadVersion(
                    DocTypeReadVersion::new(size, value),
                ))
            }
            EbmlElementId::DocTypeExtension => {
                Element::Header(EbmlHeaderElement::DocTypeExtension(DocTypeExtension::new(
                    size,
                    cursor.position() as usize, // Don't advance since it's a master element
                )))
            }
            EbmlElementId::DocTypeExtensionName => {
                let value = EbmlString::new(&size, cursor)?;
                Element::Header(EbmlHeaderElement::DocTypeExtensionName(
                    DocTypeExtensionName::new(size, value),
                ))
            }
            EbmlElementId::DocTypeExtensionVersion => {
                let value = EbmlUnsignedInteger::new(&size, cursor)?;
                Element::Header(EbmlHeaderElement::DocTypeExtensionVersion(
                    DocTypeExtensionVersion::new(size, value),
                ))
            }
            EbmlElementId::Crc32 => {
                let binary = EbmlBinary::new(&size, cursor)?;
                Element::Global(EbmlGlobalElement::Crc32(Crc32::new(size, binary)))
            }
            EbmlElementId::Void => {
                cursor.advance(size.value as usize);
                Element::Global(EbmlGlobalElement::Void(Void::new(size)))
            }
            EbmlElementId::Unknown(_) => return Err(EbmlError::UnknownHeaderElement(id, size)),
        };
        Ok(header_element)
    }
}

pub struct Chapter<'a> {
    data: &'a [u8],
}

/// Example for matroska
pub enum MatroskaElement<'a> {
    Chapter(Chapter<'a>),
    EbmlElement(Element<'a>),
}

pub struct MatroskaParser {
    base_parser: BaseEbmlParser,
}

impl MatroskaParser {
    pub fn new(base_parser: BaseEbmlParser) -> Self {
        MatroskaParser { base_parser }
    }
}

impl<'a> EbmlParser<'a> for MatroskaParser {
    type Output = MatroskaElement<'a>;

    fn next(
        &self,
        id: u64,
        size: VarInt,
        cursor: &mut Cursor<&'a [u8]>,
    ) -> Result<Self::Output, EbmlError> {
        // TODO: during WRITING, the size might not be known, and if unknownsizeallowed is true, this is allowed
        let header_element = match id {
            // ID_EBML_HEADER => MatroskaElement::Chapter(Chapter {
            //     data: cursor.get_ref(),
            // }),
            _ => {
                let x = self.base_parser.next(id, size, cursor)?;
                MatroskaElement::EbmlElement(x)
            }
        };
        Ok(header_element)
    }
}
