use nom::bytes::complete::take;
use nom::combinator::{map, map_res};
use nom::multi::count;
use nom::number::complete::{le_u32, le_u64, le_u8, *};
use nom::{bytes::complete::tag, IResult};

/// GGUF metadata value type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GGUfMetadataValueType {
    /// The value is a 8-bit unsigned integer.
    Uint8 = 0,
    /// The value is a 8-bit signed integer.
    Int8 = 1,
    /// The value is a 16-bit unsigned little-endian integer.
    Uint16 = 2,
    /// The value is a 16-bit signed little-endian integer.
    Int16 = 3,
    /// The value is a 32-bit unsigned little-endian integer.
    Uint32 = 4,
    /// The value is a 32-bit signed little-endian integer.
    Int32 = 5,
    /// The value is a 32-bit IEEE754 floating point number.
    Float32 = 6,
    /// The value is a boolean.
    Bool = 7,
    /// The value is a UTF-8 non-null-terminated string, with length prepended.
    String = 8,
    /// The value is an array of other values, with the length and type prepended.
    Array = 9,
    /// The value is a 64-bit unsigned little-endian integer.
    Uint64 = 10,
    /// The value is a 64-bit signed little-endian integer.
    Int64 = 11,
    /// The value is a 64-bit IEEE754 floating point number.
    Float64 = 12,
}

impl TryFrom<u32> for GGUfMetadataValueType {
    type Error = String;

    fn try_from(item: u32) -> Result<Self, Self::Error> {
        Ok(match item {
            0 => GGUfMetadataValueType::Uint8,
            1 => GGUfMetadataValueType::Int8,
            2 => GGUfMetadataValueType::Uint16,
            3 => GGUfMetadataValueType::Int16,
            4 => GGUfMetadataValueType::Uint32,
            5 => GGUfMetadataValueType::Int32,
            6 => GGUfMetadataValueType::Float32,
            7 => GGUfMetadataValueType::Bool,
            8 => GGUfMetadataValueType::String,
            9 => GGUfMetadataValueType::Array,
            10 => GGUfMetadataValueType::Uint64,
            11 => GGUfMetadataValueType::Int64,
            12 => GGUfMetadataValueType::Float64,
            _ => return Err(format!("invalid metadata type 0x{:x}", item)),
        })
    }
}

/// GGUF metadata value
#[derive(Debug, PartialEq)]
pub enum GGUFMetadataValue {
    Uint8(u8),
    Int8(i8),
    Uint16(u16),
    Int16(i16),
    Uint32(u32),
    Int32(i32),
    Float32(f32),
    Uint64(u64),
    Int64(i64),
    Float64(f64),
    Bool(bool),
    String(String),
    Array(Vec<GGUFMetadataValue>),
}

/// GGUF metadata
#[derive(Debug, PartialEq)]
pub struct GGUFMetadata {
    pub key: String,
    pub value_type: GGUfMetadataValueType,
    pub value: GGUFMetadataValue,
}

/// GGUF header
#[derive(Debug, PartialEq)]
pub struct GGUFHeader {
    pub version: u32,
    pub tensor_count: u64,
    pub metadata: Vec<GGUFMetadata>,
}

impl GGUFHeader {
    pub fn read(data: &[u8]) -> Result<GGUFHeader, String> {
        let (_, header) = parse_gguf_header(data).expect("failed to parse");
        Ok(header)
    }
}

fn gguf_string(i: &[u8]) -> IResult<&[u8], String> {
    let (i, len) = le_u64(i)?;
    let (i, data) = map_res(take(len), std::str::from_utf8)(i)?;
    Ok((i, data.to_string()))
}

fn magic(input: &[u8]) -> IResult<&[u8], &[u8]> {
    tag("GGUF")(input)
}

fn parse_gguf_metadata_value_type(i: &[u8]) -> IResult<&[u8], GGUfMetadataValueType> {
    map_res(le_u32, GGUfMetadataValueType::try_from)(i)
}

fn parse_gguf_metadata_value(
    value_type: GGUfMetadataValueType,
) -> impl FnMut(&[u8]) -> IResult<&[u8], GGUFMetadataValue> {
    move |i: &[u8]| {
        // parse all metadata value type
        match value_type {
            GGUfMetadataValueType::Uint8 => map(le_u8, GGUFMetadataValue::Uint8)(i),
            GGUfMetadataValueType::Int8 => map(le_i8, GGUFMetadataValue::Int8)(i),
            GGUfMetadataValueType::Uint16 => map(le_u16, GGUFMetadataValue::Uint16)(i),
            GGUfMetadataValueType::Int16 => map(le_i16, GGUFMetadataValue::Int16)(i),
            GGUfMetadataValueType::Uint32 => map(le_u32, GGUFMetadataValue::Uint32)(i),
            GGUfMetadataValueType::Int32 => map(le_i32, GGUFMetadataValue::Int32)(i),
            GGUfMetadataValueType::Float32 => map(le_f32, GGUFMetadataValue::Float32)(i),
            GGUfMetadataValueType::Uint64 => map(le_u64, GGUFMetadataValue::Uint64)(i),
            GGUfMetadataValueType::Int64 => map(le_i64, GGUFMetadataValue::Int64)(i),
            GGUfMetadataValueType::Float64 => map(le_f64, GGUFMetadataValue::Float64)(i),
            GGUfMetadataValueType::Bool => map_res(le_u8, |b| {
                if b == 0 {
                    Ok(GGUFMetadataValue::Bool(false))
                } else if b == 1 {
                    Ok(GGUFMetadataValue::Bool(true))
                } else {
                    Err("invalid bool value".to_string())
                }
            })(i),
            GGUfMetadataValueType::String => map(gguf_string, GGUFMetadataValue::String)(i),
            GGUfMetadataValueType::Array => {
                let (i, value_type) = parse_gguf_metadata_value_type(i)?;
                let (i, len) = le_u64(i)?;
                let (i, v) = count(parse_gguf_metadata_value(value_type), len as usize)(i)?;
                Ok((i, GGUFMetadataValue::Array(v)))
            }
        }
    }
}

fn parse_gguf_metadata(i: &[u8]) -> IResult<&[u8], GGUFMetadata> {
    let (i, key) = gguf_string(i)?;
    let (i, value_type) = parse_gguf_metadata_value_type(i)?;
    let (i, value) = parse_gguf_metadata_value(value_type)(i)?;
    Ok((
        i,
        GGUFMetadata {
            key,
            value_type,
            value,
        },
    ))
}

fn parse_gguf_header(i: &[u8]) -> IResult<&[u8], GGUFHeader> {
    let (i, _) = magic(i)?;
    let (i, version) = le_u32(i)?;
    let (i, tensor_count) = le_u64(i)?;
    let (i, metadata_count) = le_u64(i)?;
    let mut metadata = Vec::new();
    let mut i = i;
    for _ in 0..metadata_count {
        let (i2, m) = parse_gguf_metadata(i)?;
        metadata.push(m);
        i = i2;
    }
    Ok((
        i,
        GGUFHeader {
            version,
            tensor_count,
            metadata,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_magic() {
        let data = &[0x47, 0x47, 0x55, 0x46];
        let result = magic(data);
        assert_eq!(result, Ok((&[][..], &data[..])));
    }

    #[test]
    fn parse_header() -> Result<(), Box<dyn std::error::Error>> {
        // data hex dump
        let data = &[
            0x47, 0x47, 0x55, 0x46, 0x02, 0x00, 0x00, 0x00, 0x23, 0x01, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x14, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x67, 0x65, 0x6e, 0x65, 0x72, 0x61, 0x6c, 0x2e, 0x61, 0x72,
            0x63, 0x68, 0x69, 0x74, 0x65, 0x63, 0x74, 0x75, 0x72, 0x65, 0x08, 0x00, 0x00, 0x00,
            0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x6c, 0x6c, 0x61, 0x6d, 0x61, 0x0c,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x67, 0x65, 0x6e, 0x65, 0x72, 0x61, 0x6c,
            0x2e, 0x6e, 0x61, 0x6d, 0x65, 0x08, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x4c, 0x4c, 0x61, 0x4d, 0x41, 0x20, 0x76, 0x32, 0x14, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x6c, 0x6c, 0x61, 0x6d, 0x61, 0x2e, 0x63, 0x6f, 0x6e,
            0x74, 0x65, 0x78, 0x74, 0x5f, 0x6c, 0x65, 0x6e, 0x67, 0x74, 0x68, 0x04, 0x00, 0x00,
            0x00, 0x00, 0x10, 0x00, 0x00, 0x16, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x6c,
            0x6c, 0x61, 0x6d, 0x61, 0x2e, 0x65, 0x6d, 0x62, 0x65, 0x64, 0x64, 0x69, 0x6e, 0x67,
            0x5f, 0x6c, 0x65, 0x6e, 0x67, 0x74, 0x68, 0x04, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00,
            0x00, 0x11, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x6c, 0x6c, 0x61, 0x6d, 0x61,
            0x2e, 0x62, 0x6c, 0x6f, 0x63, 0x6b, 0x5f, 0x63, 0x6f, 0x75, 0x6e, 0x74, 0x04, 0x00,
            0x00, 0x00, 0x20, 0x00, 0x00, 0x00, 0x19, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x6c, 0x6c, 0x61, 0x6d, 0x61, 0x2e, 0x66, 0x65, 0x65, 0x64, 0x5f, 0x66, 0x6f, 0x72,
            0x77, 0x61, 0x72, 0x64, 0x5f, 0x6c, 0x65, 0x6e, 0x67, 0x74, 0x68, 0x04, 0x00, 0x00,
            0x00, 0x00, 0x2b, 0x00, 0x00, 0x1a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x6c,
            0x6c, 0x61, 0x6d, 0x61, 0x2e, 0x72, 0x6f, 0x70, 0x65, 0x2e, 0x64, 0x69, 0x6d, 0x65,
            0x6e, 0x73, 0x69, 0x6f, 0x6e, 0x5f, 0x63, 0x6f, 0x75, 0x6e, 0x74, 0x04, 0x00, 0x00,
            0x00, 0x80, 0x00, 0x00, 0x00, 0x1a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x6c,
            0x6c, 0x61, 0x6d, 0x61, 0x2e, 0x61, 0x74, 0x74, 0x65, 0x6e, 0x74, 0x69, 0x6f, 0x6e,
            0x2e, 0x68, 0x65, 0x61, 0x64, 0x5f, 0x63, 0x6f, 0x75, 0x6e, 0x74, 0x04, 0x00, 0x00,
            0x00, 0x20, 0x00, 0x00, 0x00, 0x1d, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x6c,
            0x6c, 0x61, 0x6d, 0x61, 0x2e, 0x61, 0x74, 0x74, 0x65, 0x6e, 0x74, 0x69, 0x6f, 0x6e,
            0x2e, 0x68, 0x65, 0x61, 0x64, 0x5f, 0x63, 0x6f, 0x75, 0x6e, 0x74, 0x5f, 0x6b, 0x76,
            0x04, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00, 0x26, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x6c, 0x6c, 0x61, 0x6d, 0x61, 0x2e, 0x61, 0x74, 0x74, 0x65, 0x6e, 0x74,
            0x69, 0x6f, 0x6e, 0x2e, 0x6c, 0x61, 0x79, 0x65, 0x72, 0x5f, 0x6e, 0x6f, 0x72, 0x6d,
            0x5f, 0x72, 0x6d, 0x73, 0x5f, 0x65, 0x70, 0x73, 0x69, 0x6c, 0x6f, 0x6e, 0x06, 0x00,
            0x00, 0x00, 0xac, 0xc5, 0x27, 0x37, 0x11, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x67, 0x65, 0x6e, 0x65, 0x72, 0x61, 0x6c, 0x2e, 0x66, 0x69, 0x6c, 0x65, 0x5f, 0x74,
            0x79, 0x70, 0x65, 0x04, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x14, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x74, 0x6f, 0x6b, 0x65, 0x6e, 0x69, 0x7a, 0x65, 0x72,
            0x2e, 0x67, 0x67, 0x6d, 0x6c, 0x2e, 0x6d, 0x6f, 0x64, 0x65, 0x6c, 0x08, 0x00, 0x00,
            0x00, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x6c, 0x6c, 0x61, 0x6d, 0x61,
            0x15, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x74, 0x6f, 0x6b, 0x65, 0x6e, 0x69,
            0x7a, 0x65, 0x72, 0x2e, 0x67, 0x67, 0x6d, 0x6c, 0x2e, 0x74, 0x6f, 0x6b, 0x65, 0x6e,
            0x73, 0x09, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x7d, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x3c, 0x75, 0x6e,
            0x6b, 0x3e, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x3c, 0x73, 0x3e, 0x04,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x3c, 0x2f, 0x73, 0x3e, 0x06, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x3c, 0x30, 0x78, 0x30, 0x30, 0x3e, 0x06, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x3c, 0x30, 0x78, 0x30, 0x31, 0x3e, 0x06, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x3c, 0x30, 0x78, 0x30, 0x32, 0x3e, 0x06, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x3c, 0x30, 0x78, 0x30, 0x33, 0x3e, 0x06, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x3c, 0x30, 0x78, 0x30, 0x34, 0x3e, 0x06, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x3c, 0x30, 0x78, 0x30, 0x35, 0x3e, 0x06, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x3c, 0x30, 0x78, 0x30, 0x36, 0x3e, 0x06, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x3c, 0x30, 0x78, 0x30, 0x37, 0x3e, 0x06, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x3c, 0x30, 0x78, 0x30, 0x38, 0x3e, 0x06, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x3c, 0x30, 0x78, 0x30, 0x39, 0x3e, 0x06, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x3c, 0x30, 0x78, 0x30, 0x41, 0x3e, 0x06, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x3c, 0x30, 0x78, 0x30, 0x42, 0x3e, 0x06,
        ];

        let (_, result) = parse_gguf_header(data)?;
        assert_eq!(
            result,
            GGUFHeader {
                version: 2,
                tensor_count: 291,
                metadata: vec![
                    GGUFMetadata {
                        key: "general.architecture".to_string(),
                        value_type: GGUfMetadataValueType::String,
                        value: GGUFMetadataValue::String("llama".to_string()),
                    },
                    GGUFMetadata {
                        key: "general.name".to_string(),
                        value_type: GGUfMetadataValueType::String,
                        value: GGUFMetadataValue::String("LLaMA v2".to_string()),
                    },
                    GGUFMetadata {
                        key: "llama.context_length".to_string(),
                        value_type: GGUfMetadataValueType::Uint32,
                        value: GGUFMetadataValue::Uint32(4096)
                    },
                ]
            }
        );
        Ok(())
    }
}