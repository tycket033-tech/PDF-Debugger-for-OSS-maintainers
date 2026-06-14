use crate::pdf_model::{PdfDictionary, PdfStream, PdfValue};
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use serde::Serialize;
use std::io::{Read, Write};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DecodeIssueKind {
    Failed,
    Unsupported,
}

#[derive(Clone, Debug, Serialize)]
pub struct DecodeIssue {
    pub filter: String,
    pub kind: DecodeIssueKind,
    pub message: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DecodeStepStatus {
    Decoded,
    Passthrough,
    Failed,
    Unsupported,
}

#[derive(Clone, Debug, Serialize)]
pub struct DecodeStep {
    pub filter: String,
    pub status: DecodeStepStatus,
    pub message: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct StreamDecodeResult {
    pub filters: Vec<String>,
    #[serde(skip_serializing)]
    pub decoded: Vec<u8>,
    pub decoded_length: usize,
    pub steps: Vec<DecodeStep>,
    pub issues: Vec<DecodeIssue>,
}

impl StreamDecodeResult {
    pub fn has_issues(&self) -> bool {
        !self.issues.is_empty()
    }
}

pub fn decode_stream(stream: &PdfStream) -> StreamDecodeResult {
    decode_stream_with_limit(stream, None)
}

pub fn decode_stream_with_limit(
    stream: &PdfStream,
    max_decoded_length: Option<usize>,
) -> StreamDecodeResult {
    decode_filter_chain_with_limit(
        stream.raw_bytes.clone(),
        &stream.filters,
        max_decoded_length,
    )
}

pub fn decode_filter_chain(raw_bytes: Vec<u8>, filters: &[String]) -> StreamDecodeResult {
    decode_filter_chain_with_limit(raw_bytes, filters, None)
}

pub fn encode_filter_chain(decoded_bytes: &[u8], filters: &[String]) -> Result<Vec<u8>, String> {
    if filters.is_empty() {
        return Ok(decoded_bytes.to_vec());
    }

    if filters.len() == 1 && normalize_filter_name(&filters[0]) == "FlateDecode" {
        return flate_encode(decoded_bytes);
    }

    Err(format!(
        "content stream editing can only re-encode unfiltered or single /FlateDecode streams; current filters: {}",
        filters
            .iter()
            .map(|filter| format!("/{filter}"))
            .collect::<Vec<_>>()
            .join(" -> ")
    ))
}

pub fn decode_filter_chain_with_limit(
    raw_bytes: Vec<u8>,
    filters: &[String],
    max_decoded_length: Option<usize>,
) -> StreamDecodeResult {
    let mut decoded = raw_bytes;
    let mut steps = Vec::new();
    let mut issues = Vec::new();

    if let Some(limit) = max_decoded_length {
        if decoded.len() > limit {
            issues.push(DecodeIssue {
                filter: "raw".to_string(),
                kind: DecodeIssueKind::Failed,
                message: format!("raw stream bytes exceed decoded output limit of {limit} bytes"),
            });
            steps.push(DecodeStep {
                filter: "raw".to_string(),
                status: DecodeStepStatus::Failed,
                message: format!("raw stream bytes exceed decoded output limit of {limit} bytes"),
            });
            return StreamDecodeResult {
                filters: filters.to_vec(),
                decoded,
                decoded_length: 0,
                steps,
                issues,
            };
        }
    }

    for filter in filters {
        match normalize_filter_name(filter).as_str() {
            "FlateDecode" => match flate_decode(&decoded, max_decoded_length) {
                Ok(next) => {
                    decoded = next;
                    steps.push(DecodeStep {
                        filter: filter.clone(),
                        status: DecodeStepStatus::Decoded,
                        message: "decoded with zlib/deflate".to_string(),
                    });
                }
                Err(message) => {
                    issues.push(DecodeIssue {
                        filter: filter.clone(),
                        kind: DecodeIssueKind::Failed,
                        message: message.clone(),
                    });
                    steps.push(DecodeStep {
                        filter: filter.clone(),
                        status: DecodeStepStatus::Failed,
                        message,
                    });
                    break;
                }
            },
            "ASCIIHexDecode" => match ascii_hex_decode(&decoded, max_decoded_length) {
                Ok(next) => {
                    decoded = next;
                    steps.push(DecodeStep {
                        filter: filter.clone(),
                        status: DecodeStepStatus::Decoded,
                        message: "decoded ASCII hexadecimal data".to_string(),
                    });
                }
                Err(message) => {
                    issues.push(DecodeIssue {
                        filter: filter.clone(),
                        kind: DecodeIssueKind::Failed,
                        message: message.clone(),
                    });
                    steps.push(DecodeStep {
                        filter: filter.clone(),
                        status: DecodeStepStatus::Failed,
                        message,
                    });
                    break;
                }
            },
            "ASCII85Decode" => match ascii85_decode(&decoded, max_decoded_length) {
                Ok(next) => {
                    decoded = next;
                    steps.push(DecodeStep {
                        filter: filter.clone(),
                        status: DecodeStepStatus::Decoded,
                        message: "decoded ASCII85 data".to_string(),
                    });
                }
                Err(message) => {
                    issues.push(DecodeIssue {
                        filter: filter.clone(),
                        kind: DecodeIssueKind::Failed,
                        message: message.clone(),
                    });
                    steps.push(DecodeStep {
                        filter: filter.clone(),
                        status: DecodeStepStatus::Failed,
                        message,
                    });
                    break;
                }
            },
            "RunLengthDecode" => match run_length_decode(&decoded, max_decoded_length) {
                Ok(next) => {
                    decoded = next;
                    steps.push(DecodeStep {
                        filter: filter.clone(),
                        status: DecodeStepStatus::Decoded,
                        message: "decoded PDF run-length data".to_string(),
                    });
                }
                Err(message) => {
                    issues.push(DecodeIssue {
                        filter: filter.clone(),
                        kind: DecodeIssueKind::Failed,
                        message: message.clone(),
                    });
                    steps.push(DecodeStep {
                        filter: filter.clone(),
                        status: DecodeStepStatus::Failed,
                        message,
                    });
                    break;
                }
            },
            "DCTDecode" => {
                if let Some(limit) = max_decoded_length {
                    if decoded.len() > limit {
                        let message = format!(
                            "DCT/JPEG stream exceeds decoded output limit of {limit} bytes"
                        );
                        issues.push(DecodeIssue {
                            filter: filter.clone(),
                            kind: DecodeIssueKind::Failed,
                            message: message.clone(),
                        });
                        steps.push(DecodeStep {
                            filter: filter.clone(),
                            status: DecodeStepStatus::Failed,
                            message,
                        });
                        break;
                    }
                }
                steps.push(DecodeStep {
                    filter: filter.clone(),
                    status: DecodeStepStatus::Passthrough,
                    message: "DCT/JPEG image data preserved for preview or export".to_string(),
                });
            }
            _ => {
                let message = format!("unsupported stream filter /{filter}");
                issues.push(DecodeIssue {
                    filter: filter.clone(),
                    kind: DecodeIssueKind::Unsupported,
                    message: message.clone(),
                });
                steps.push(DecodeStep {
                    filter: filter.clone(),
                    status: DecodeStepStatus::Unsupported,
                    message,
                });
                break;
            }
        }
    }

    let decoded_length = decoded.len();
    StreamDecodeResult {
        filters: filters.to_vec(),
        decoded,
        decoded_length,
        steps,
        issues,
    }
}

pub fn filter_names_from_dictionary(dictionary: &PdfDictionary) -> Vec<String> {
    match dictionary.get("Filter") {
        Some(PdfValue::Name(name)) => vec![name.clone()],
        Some(PdfValue::Array(filters)) => filters
            .iter()
            .filter_map(|value| value.as_name().map(ToOwned::to_owned))
            .collect(),
        _ => Vec::new(),
    }
}

fn normalize_filter_name(filter: &str) -> String {
    match filter {
        "Fl" => "FlateDecode",
        "AHx" => "ASCIIHexDecode",
        "A85" => "ASCII85Decode",
        "RL" => "RunLengthDecode",
        "DCT" => "DCTDecode",
        other => other,
    }
    .to_string()
}

fn flate_decode(input: &[u8], max_output_length: Option<usize>) -> Result<Vec<u8>, String> {
    let mut decoder = ZlibDecoder::new(input);
    let mut output = Vec::new();
    if let Some(limit) = max_output_length {
        let max_read = limit.saturating_add(1) as u64;
        decoder
            .take(max_read)
            .read_to_end(&mut output)
            .map_err(|error| error.to_string())?;
        ensure_output_limit(output.len(), limit)?;
    } else {
        decoder
            .read_to_end(&mut output)
            .map_err(|error| error.to_string())?;
    }
    Ok(output)
}

fn flate_encode(input: &[u8]) -> Result<Vec<u8>, String> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(input)
        .map_err(|error| format!("FlateEncode write failed: {error}"))?;
    encoder
        .finish()
        .map_err(|error| format!("FlateEncode finish failed: {error}"))
}

fn ascii_hex_decode(input: &[u8], max_output_length: Option<usize>) -> Result<Vec<u8>, String> {
    let mut nibbles = Vec::new();
    for byte in input.iter().copied() {
        if byte == b'>' {
            break;
        }
        if byte.is_ascii_whitespace() {
            continue;
        }
        let nibble = match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            b'A'..=b'F' => byte - b'A' + 10,
            _ => return Err(format!("invalid ASCIIHex byte 0x{byte:02X}")),
        };
        nibbles.push(nibble);
    }

    if nibbles.len() % 2 == 1 {
        nibbles.push(0);
    }

    let output = nibbles
        .chunks(2)
        .map(|pair| (pair[0] << 4) | pair[1])
        .collect::<Vec<_>>();
    if let Some(limit) = max_output_length {
        ensure_output_limit(output.len(), limit)?;
    }
    Ok(output)
}

fn ascii85_decode(input: &[u8], max_output_length: Option<usize>) -> Result<Vec<u8>, String> {
    let mut data = input
        .iter()
        .copied()
        .filter(|byte| !byte.is_ascii_whitespace())
        .collect::<Vec<_>>();

    if data.starts_with(b"<~") {
        data.drain(..2);
    }

    let mut output = Vec::new();
    let mut group = Vec::with_capacity(5);
    let mut index = 0;

    while index < data.len() {
        let byte = data[index];
        if byte == b'~' {
            break;
        }
        if byte == b'z' {
            if !group.is_empty() {
                return Err("'z' short form appeared inside an ASCII85 group".to_string());
            }
            output.extend_from_slice(&[0, 0, 0, 0]);
            if let Some(limit) = max_output_length {
                ensure_output_limit(output.len(), limit)?;
            }
            index += 1;
            continue;
        }
        if !(b'!'..=b'u').contains(&byte) {
            return Err(format!("invalid ASCII85 byte 0x{byte:02X}"));
        }

        group.push(byte - b'!');
        if group.len() == 5 {
            append_ascii85_group(&mut output, &group, 4);
            if let Some(limit) = max_output_length {
                ensure_output_limit(output.len(), limit)?;
            }
            group.clear();
        }
        index += 1;
    }

    if !group.is_empty() {
        if group.len() == 1 {
            return Err("ASCII85 final group has only one digit".to_string());
        }
        let output_bytes = group.len() - 1;
        while group.len() < 5 {
            group.push(b'u' - b'!');
        }
        append_ascii85_group(&mut output, &group, output_bytes);
        if let Some(limit) = max_output_length {
            ensure_output_limit(output.len(), limit)?;
        }
    }

    Ok(output)
}

fn append_ascii85_group(output: &mut Vec<u8>, group: &[u8], output_bytes: usize) {
    let mut value = 0u32;
    for digit in group {
        value = value * 85 + *digit as u32;
    }
    let bytes = value.to_be_bytes();
    output.extend_from_slice(&bytes[..output_bytes]);
}

fn run_length_decode(input: &[u8], max_output_length: Option<usize>) -> Result<Vec<u8>, String> {
    let mut output = Vec::new();
    let mut index = 0;

    while index < input.len() {
        let length = input[index];
        index += 1;

        match length {
            128 => return Ok(output),
            0..=127 => {
                let run = length as usize + 1;
                if index + run > input.len() {
                    return Err("RunLengthDecode literal run exceeds stream length".to_string());
                }
                output.extend_from_slice(&input[index..index + run]);
                if let Some(limit) = max_output_length {
                    ensure_output_limit(output.len(), limit)?;
                }
                index += run;
            }
            129..=255 => {
                if index >= input.len() {
                    return Err("RunLengthDecode repeat run is missing its byte".to_string());
                }
                let repeat = 257usize - length as usize;
                output.extend(std::iter::repeat(input[index]).take(repeat));
                if let Some(limit) = max_output_length {
                    ensure_output_limit(output.len(), limit)?;
                }
                index += 1;
            }
        }
    }

    Ok(output)
}

fn ensure_output_limit(length: usize, limit: usize) -> Result<(), String> {
    if length > limit {
        Err(format!(
            "decoded stream exceeds output limit of {limit} bytes"
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_ascii_hex() {
        let decoded = ascii_hex_decode(b"48656c6c6f>", None).unwrap();
        assert_eq!(decoded, b"Hello");
    }

    #[test]
    fn decodes_ascii85() {
        let decoded = ascii85_decode(b"<~87cURD]i,\"Ebo7~>", None).unwrap();
        assert_eq!(decoded, b"Hello World");
    }

    #[test]
    fn decodes_run_length() {
        let decoded = run_length_decode(&[2, b'a', b'b', b'c', 254, b'Z', 128], None).unwrap();
        assert_eq!(decoded, b"abcZZZ");
    }
}
