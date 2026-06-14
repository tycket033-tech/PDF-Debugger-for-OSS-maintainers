#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PdfStringEncoding {
    Utf16Be,
    Utf16Le,
    PdfDocEncoding,
    Utf8,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecodedPdfString {
    pub text: String,
    pub encoding: PdfStringEncoding,
    pub warnings: Vec<String>,
}

pub fn decode_pdf_string(bytes: &[u8]) -> DecodedPdfString {
    if bytes.starts_with(&[0xfe, 0xff]) {
        return decode_utf16_units(&bytes[2..], true, PdfStringEncoding::Utf16Be);
    }
    if bytes.starts_with(&[0xff, 0xfe]) {
        return decode_utf16_units(&bytes[2..], false, PdfStringEncoding::Utf16Le);
    }
    if let Ok(text) = std::str::from_utf8(bytes) {
        if !contains_control_noise(text) {
            return DecodedPdfString {
                text: text.to_string(),
                encoding: PdfStringEncoding::Utf8,
                warnings: Vec::new(),
            };
        }
    }

    DecodedPdfString {
        text: bytes
            .iter()
            .map(|byte| pdf_doc_encoding_char(*byte))
            .collect::<String>(),
        encoding: PdfStringEncoding::PdfDocEncoding,
        warnings: Vec::new(),
    }
}

pub fn decode_pdf_hex_string(hex: &str) -> DecodedPdfString {
    let mut warnings = Vec::new();
    let mut nibbles = hex
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .collect::<Vec<_>>();
    if nibbles.len() % 2 == 1 {
        nibbles.push(b'0');
        warnings.push("Odd-length PDF hex string was padded with a trailing 0 nibble.".to_string());
    }

    let mut bytes = Vec::with_capacity(nibbles.len() / 2);
    let mut index = 0;
    while index + 1 < nibbles.len() {
        match (hex_value(nibbles[index]), hex_value(nibbles[index + 1])) {
            (Some(high), Some(low)) => bytes.push((high << 4) | low),
            _ => warnings.push(format!(
                "Invalid hex pair at byte offset {} was skipped.",
                index
            )),
        }
        index += 2;
    }

    let mut decoded = decode_pdf_string(&bytes);
    decoded.warnings.splice(0..0, warnings);
    decoded
}

fn decode_utf16_units(
    bytes: &[u8],
    big_endian: bool,
    encoding: PdfStringEncoding,
) -> DecodedPdfString {
    let mut warnings = Vec::new();
    if bytes.len() % 2 == 1 {
        warnings.push(
            "UTF-16 PDF string has an odd byte length; the last byte was ignored.".to_string(),
        );
    }
    let units = bytes
        .chunks_exact(2)
        .map(|chunk| {
            if big_endian {
                u16::from_be_bytes([chunk[0], chunk[1]])
            } else {
                u16::from_le_bytes([chunk[0], chunk[1]])
            }
        })
        .collect::<Vec<_>>();
    let text = char::decode_utf16(units)
        .map(|item| match item {
            Ok(character) => character,
            Err(_) => {
                warnings.push("UTF-16 PDF string contained an invalid surrogate pair.".to_string());
                '\u{fffd}'
            }
        })
        .collect::<String>();

    DecodedPdfString {
        text,
        encoding,
        warnings,
    }
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn contains_control_noise(text: &str) -> bool {
    text.chars()
        .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
}

fn pdf_doc_encoding_char(byte: u8) -> char {
    match byte {
        0x00 => '\u{0000}',
        0x01 => '\u{0001}',
        0x02 => '\u{0002}',
        0x03 => '\u{0003}',
        0x04 => '\u{0004}',
        0x05 => '\u{0005}',
        0x06 => '\u{0006}',
        0x07 => '\u{0007}',
        0x08 => '\u{0008}',
        0x09 => '\t',
        0x0a => '\n',
        0x0b => '\u{000b}',
        0x0c => '\u{000c}',
        0x0d => '\r',
        0x0e => '\u{000e}',
        0x0f => '\u{000f}',
        0x10 => '\u{0010}',
        0x11 => '\u{0011}',
        0x12 => '\u{0012}',
        0x13 => '\u{0013}',
        0x14 => '\u{0014}',
        0x15 => '\u{0015}',
        0x16 => '\u{0016}',
        0x17 => '\u{0017}',
        0x18 => '\u{02d8}',
        0x19 => '\u{02c7}',
        0x1a => '\u{02c6}',
        0x1b => '\u{02d9}',
        0x1c => '\u{02dd}',
        0x1d => '\u{02db}',
        0x1e => '\u{02da}',
        0x1f => '\u{02dc}',
        0x20..=0x7e => byte as char,
        0x7f => '\u{007f}',
        0x80 => '\u{2022}',
        0x81 => '\u{2020}',
        0x82 => '\u{2021}',
        0x83 => '\u{2026}',
        0x84 => '\u{2014}',
        0x85 => '\u{2013}',
        0x86 => '\u{0192}',
        0x87 => '\u{2044}',
        0x88 => '\u{2039}',
        0x89 => '\u{203a}',
        0x8a => '\u{2212}',
        0x8b => '\u{2030}',
        0x8c => '\u{201e}',
        0x8d => '\u{201c}',
        0x8e => '\u{201d}',
        0x8f => '\u{2018}',
        0x90 => '\u{2019}',
        0x91 => '\u{201a}',
        0x92 => '\u{2122}',
        0x93 => '\u{fb01}',
        0x94 => '\u{fb02}',
        0x95 => '\u{0141}',
        0x96 => '\u{0152}',
        0x97 => '\u{0160}',
        0x98 => '\u{0178}',
        0x99 => '\u{017d}',
        0x9a => '\u{0131}',
        0x9b => '\u{0142}',
        0x9c => '\u{0153}',
        0x9d => '\u{0161}',
        0x9e => '\u{017e}',
        0x9f => '\u{fffd}',
        0xa0 => '\u{20ac}',
        0xa1 => '¡',
        0xa2 => '¢',
        0xa3 => '£',
        0xa4 => '¤',
        0xa5 => '¥',
        0xa6 => '¦',
        0xa7 => '§',
        0xa8 => '¨',
        0xa9 => '©',
        0xaa => 'ª',
        0xab => '«',
        0xac => '¬',
        0xad => '\u{00ad}',
        0xae => '®',
        0xaf => '¯',
        0xb0 => '°',
        0xb1 => '±',
        0xb2 => '²',
        0xb3 => '³',
        0xb4 => '´',
        0xb5 => 'µ',
        0xb6 => '¶',
        0xb7 => '·',
        0xb8 => '¸',
        0xb9 => '¹',
        0xba => 'º',
        0xbb => '»',
        0xbc => '¼',
        0xbd => '½',
        0xbe => '¾',
        0xbf => '¿',
        0xc0 => 'À',
        0xc1 => 'Á',
        0xc2 => 'Â',
        0xc3 => 'Ã',
        0xc4 => 'Ä',
        0xc5 => 'Å',
        0xc6 => 'Æ',
        0xc7 => 'Ç',
        0xc8 => 'È',
        0xc9 => 'É',
        0xca => 'Ê',
        0xcb => 'Ë',
        0xcc => 'Ì',
        0xcd => 'Í',
        0xce => 'Î',
        0xcf => 'Ï',
        0xd0 => 'Ð',
        0xd1 => 'Ñ',
        0xd2 => 'Ò',
        0xd3 => 'Ó',
        0xd4 => 'Ô',
        0xd5 => 'Õ',
        0xd6 => 'Ö',
        0xd7 => '×',
        0xd8 => 'Ø',
        0xd9 => 'Ù',
        0xda => 'Ú',
        0xdb => 'Û',
        0xdc => 'Ü',
        0xdd => 'Ý',
        0xde => 'Þ',
        0xdf => 'ß',
        0xe0 => 'à',
        0xe1 => 'á',
        0xe2 => 'â',
        0xe3 => 'ã',
        0xe4 => 'ä',
        0xe5 => 'å',
        0xe6 => 'æ',
        0xe7 => 'ç',
        0xe8 => 'è',
        0xe9 => 'é',
        0xea => 'ê',
        0xeb => 'ë',
        0xec => 'ì',
        0xed => 'í',
        0xee => 'î',
        0xef => 'ï',
        0xf0 => 'ð',
        0xf1 => 'ñ',
        0xf2 => 'ò',
        0xf3 => 'ó',
        0xf4 => 'ô',
        0xf5 => 'õ',
        0xf6 => 'ö',
        0xf7 => '÷',
        0xf8 => 'ø',
        0xf9 => 'ù',
        0xfa => 'ú',
        0xfb => 'û',
        0xfc => 'ü',
        0xfd => 'ý',
        0xfe => 'þ',
        0xff => 'ÿ',
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_utf16be_bom_pdf_string() {
        let decoded = decode_pdf_hex_string(
            "FEFF004D006900630072006F0073006F0066007400200057006F0072006400200032003000310036",
        );
        assert_eq!(decoded.text, "Microsoft Word 2016");
        assert_eq!(decoded.encoding, PdfStringEncoding::Utf16Be);
    }

    #[test]
    fn decodes_pdf_doc_encoding() {
        let decoded = decode_pdf_string(&[0x48, 0x8d, 0x69]);
        assert_eq!(decoded.text, "H“i");
        assert_eq!(decoded.encoding, PdfStringEncoding::PdfDocEncoding);
    }
}
