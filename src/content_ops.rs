use crate::pdf_model::ByteRange;
use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentTokenKind {
    Number,
    Name,
    String,
    HexString,
    Keyword,
    Comment,
    ArrayStart,
    ArrayEnd,
    DictionaryStart,
    DictionaryEnd,
    Other,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ContentToken {
    pub kind: ContentTokenKind,
    pub lexeme: String,
    pub byte_range: ByteRange,
    #[serde(skip_serializing)]
    pub raw_bytes: Vec<u8>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ContentOperator {
    pub name: String,
    pub operands: Vec<ContentToken>,
    pub byte_range: ByteRange,
    pub known: bool,
    pub explanation: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ContentWarning {
    pub rule_id: String,
    pub operator: Option<String>,
    pub byte_range: Option<ByteRange>,
    pub message: String,
    pub suggested_next_step: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ContentAnalysis {
    pub tokens: Vec<ContentToken>,
    pub operators: Vec<ContentOperator>,
    pub warnings: Vec<ContentWarning>,
}

pub fn analyze_content_stream(bytes: &[u8]) -> ContentAnalysis {
    let tokens = tokenize_content_stream(bytes);
    let operators = group_operators(&tokens);
    let warnings = analyze_operator_warnings(&operators);

    ContentAnalysis {
        tokens,
        operators,
        warnings,
    }
}

pub fn tokenize_content_stream(bytes: &[u8]) -> Vec<ContentToken> {
    let mut tokens = Vec::new();
    let mut pos = 0;

    while pos < bytes.len() {
        pos = skip_whitespace(bytes, pos);
        if pos >= bytes.len() {
            break;
        }

        let start = pos;
        match bytes[pos] {
            b'%' => {
                pos += 1;
                while pos < bytes.len() && !matches!(bytes[pos], b'\n' | b'\r') {
                    pos += 1;
                }
                tokens.push(token(
                    ContentTokenKind::Comment,
                    &bytes[start..pos],
                    start,
                    pos,
                ));
            }
            b'/' => {
                pos += 1;
                let name_start = pos;
                while pos < bytes.len() && !is_delimiter(bytes[pos]) {
                    pos += 1;
                }
                tokens.push(ContentToken {
                    kind: ContentTokenKind::Name,
                    lexeme: decode_name(&bytes[name_start..pos]),
                    byte_range: ByteRange { start, end: pos },
                    raw_bytes: bytes[start..pos].to_vec(),
                });
            }
            b'(' => {
                pos = parse_literal_string(bytes, pos);
                tokens.push(token(
                    ContentTokenKind::String,
                    &bytes[start..pos],
                    start,
                    pos,
                ));
            }
            b'<' if bytes.get(pos + 1) == Some(&b'<') => {
                pos += 2;
                tokens.push(token(
                    ContentTokenKind::DictionaryStart,
                    &bytes[start..pos],
                    start,
                    pos,
                ));
            }
            b'>' if bytes.get(pos + 1) == Some(&b'>') => {
                pos += 2;
                tokens.push(token(
                    ContentTokenKind::DictionaryEnd,
                    &bytes[start..pos],
                    start,
                    pos,
                ));
            }
            b'<' => {
                pos += 1;
                while pos < bytes.len() && bytes[pos] != b'>' {
                    pos += 1;
                }
                if pos < bytes.len() {
                    pos += 1;
                }
                tokens.push(token(
                    ContentTokenKind::HexString,
                    &bytes[start..pos],
                    start,
                    pos,
                ));
            }
            b'[' => {
                pos += 1;
                tokens.push(token(
                    ContentTokenKind::ArrayStart,
                    &bytes[start..pos],
                    start,
                    pos,
                ));
            }
            b']' => {
                pos += 1;
                tokens.push(token(
                    ContentTokenKind::ArrayEnd,
                    &bytes[start..pos],
                    start,
                    pos,
                ));
            }
            _ => {
                while pos < bytes.len() && !is_delimiter(bytes[pos]) {
                    pos += 1;
                }
                let kind = classify_atom(&bytes[start..pos]);
                tokens.push(token(kind, &bytes[start..pos], start, pos));
            }
        }
    }

    tokens
}

pub fn font_names_used(analysis: &ContentAnalysis) -> BTreeSet<String> {
    analysis
        .operators
        .iter()
        .filter(|operator| operator.name == "Tf")
        .filter_map(|operator| {
            operator
                .operands
                .iter()
                .find(|token| token.kind == ContentTokenKind::Name)
                .map(|token| token.lexeme.clone())
        })
        .collect()
}

pub fn xobject_names_used(analysis: &ContentAnalysis) -> BTreeSet<String> {
    analysis
        .operators
        .iter()
        .filter(|operator| operator.name == "Do")
        .filter_map(|operator| {
            operator
                .operands
                .iter()
                .rev()
                .find(|token| token.kind == ContentTokenKind::Name)
                .map(|token| token.lexeme.clone())
        })
        .collect()
}

fn group_operators(tokens: &[ContentToken]) -> Vec<ContentOperator> {
    let mut operators = Vec::new();
    let mut operands = Vec::new();
    let mut array_depth = 0usize;
    let mut dictionary_depth = 0usize;
    let mut skip_until_inline_image_end = false;

    for token in tokens {
        match token.kind {
            ContentTokenKind::Comment => continue,
            ContentTokenKind::ArrayStart => {
                array_depth += 1;
                operands.push(token.clone());
                continue;
            }
            ContentTokenKind::ArrayEnd => {
                array_depth = array_depth.saturating_sub(1);
                operands.push(token.clone());
                continue;
            }
            ContentTokenKind::DictionaryStart => {
                dictionary_depth += 1;
                operands.push(token.clone());
                continue;
            }
            ContentTokenKind::DictionaryEnd => {
                dictionary_depth = dictionary_depth.saturating_sub(1);
                operands.push(token.clone());
                continue;
            }
            _ => {}
        }

        if skip_until_inline_image_end {
            if token.kind == ContentTokenKind::Keyword && token.lexeme == "EI" {
                operators.push(content_operator(token, Vec::new()));
                skip_until_inline_image_end = false;
            }
            continue;
        }

        if token.kind == ContentTokenKind::Keyword
            && array_depth == 0
            && dictionary_depth == 0
            && !is_keyword_operand(&token.lexeme)
        {
            let operator = content_operator(token, std::mem::take(&mut operands));
            if operator.name == "ID" {
                skip_until_inline_image_end = true;
            }
            operators.push(operator);
        } else {
            operands.push(token.clone());
        }
    }

    operators
}

fn content_operator(token: &ContentToken, operands: Vec<ContentToken>) -> ContentOperator {
    let known = is_known_operator(&token.lexeme);
    ContentOperator {
        name: token.lexeme.clone(),
        operands,
        byte_range: token.byte_range,
        known,
        explanation: operator_explanation(&token.lexeme).to_string(),
    }
}

fn analyze_operator_warnings(operators: &[ContentOperator]) -> Vec<ContentWarning> {
    let mut warnings = Vec::new();
    let mut in_text_object = false;
    let mut graphics_state_depth = 0usize;

    for operator in operators {
        if !operator.known {
            warnings.push(ContentWarning {
                rule_id: "content.unknown_operator".to_string(),
                operator: Some(operator.name.clone()),
                byte_range: Some(operator.byte_range),
                message: format!(
                    "Unknown or unsupported content stream operator `{}` at content byte range {}..{}",
                    operator.name, operator.byte_range.start, operator.byte_range.end
                ),
                suggested_next_step: "Check whether the content stream contains a typo, unsupported extension, or corrupted operator bytes".to_string(),
            });
        }

        match operator.name.as_str() {
            "BT" => {
                if in_text_object {
                    warnings.push(ContentWarning {
                        rule_id: "content.nested_text_object".to_string(),
                        operator: Some(operator.name.clone()),
                        byte_range: Some(operator.byte_range),
                        message: "Nested BT operator found before the previous text object ended".to_string(),
                        suggested_next_step: "Verify that every BT text object is closed with ET before starting another".to_string(),
                    });
                }
                in_text_object = true;
            }
            "ET" => {
                if !in_text_object {
                    warnings.push(ContentWarning {
                        rule_id: "content.unmatched_text_end".to_string(),
                        operator: Some(operator.name.clone()),
                        byte_range: Some(operator.byte_range),
                        message: "ET operator found without a matching BT".to_string(),
                        suggested_next_step:
                            "Check content generation around text object boundaries".to_string(),
                    });
                }
                in_text_object = false;
            }
            "q" => graphics_state_depth += 1,
            "Q" => {
                if graphics_state_depth == 0 {
                    warnings.push(ContentWarning {
                        rule_id: "content.unmatched_graphics_restore".to_string(),
                        operator: Some(operator.name.clone()),
                        byte_range: Some(operator.byte_range),
                        message: "Q operator restores a graphics state that was not saved with q"
                            .to_string(),
                        suggested_next_step: "Check q/Q balancing in the page content stream"
                            .to_string(),
                    });
                } else {
                    graphics_state_depth -= 1;
                }
            }
            _ => {}
        }
    }

    if in_text_object {
        warnings.push(ContentWarning {
            rule_id: "content.unclosed_text_object".to_string(),
            operator: None,
            byte_range: operators.last().map(|operator| operator.byte_range),
            message: "Content stream ended while a BT text object was still open".to_string(),
            suggested_next_step:
                "Add a matching ET or inspect whether the content stream was truncated".to_string(),
        });
    }

    if graphics_state_depth > 0 {
        warnings.push(ContentWarning {
            rule_id: "content.unbalanced_graphics_state".to_string(),
            operator: None,
            byte_range: operators.last().map(|operator| operator.byte_range),
            message: format!(
                "Content stream ended with {} unmatched q graphics state save operator(s)",
                graphics_state_depth
            ),
            suggested_next_step:
                "Add matching Q operators or inspect whether the content stream was truncated"
                    .to_string(),
        });
    }

    warnings
}

fn token(kind: ContentTokenKind, bytes: &[u8], start: usize, end: usize) -> ContentToken {
    ContentToken {
        kind,
        lexeme: String::from_utf8_lossy(bytes).into_owned(),
        byte_range: ByteRange { start, end },
        raw_bytes: bytes.to_vec(),
    }
}

fn parse_literal_string(bytes: &[u8], mut pos: usize) -> usize {
    pos += 1;
    let mut depth = 1usize;

    while pos < bytes.len() {
        match bytes[pos] {
            b'\\' => {
                pos += 1;
                if pos < bytes.len() {
                    pos += 1;
                }
            }
            b'(' => {
                depth += 1;
                pos += 1;
            }
            b')' => {
                depth = depth.saturating_sub(1);
                pos += 1;
                if depth == 0 {
                    return pos;
                }
            }
            _ => pos += 1,
        }
    }

    pos
}

fn classify_atom(bytes: &[u8]) -> ContentTokenKind {
    if is_number(bytes) {
        ContentTokenKind::Number
    } else if bytes
        .iter()
        .all(|byte| byte.is_ascii_graphic() && !is_delimiter(*byte))
    {
        ContentTokenKind::Keyword
    } else {
        ContentTokenKind::Other
    }
}

fn is_number(bytes: &[u8]) -> bool {
    std::str::from_utf8(bytes)
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .is_some()
}

fn is_keyword_operand(keyword: &str) -> bool {
    matches!(keyword, "true" | "false" | "null")
}

fn is_known_operator(operator: &str) -> bool {
    matches!(
        operator,
        "BT" | "ET"
            | "Tc"
            | "Tw"
            | "Tz"
            | "TL"
            | "Tf"
            | "Tr"
            | "Ts"
            | "Td"
            | "TD"
            | "Tm"
            | "T*"
            | "Tj"
            | "TJ"
            | "'"
            | "\""
            | "q"
            | "Q"
            | "cm"
            | "w"
            | "J"
            | "j"
            | "M"
            | "d"
            | "ri"
            | "i"
            | "gs"
            | "m"
            | "l"
            | "c"
            | "v"
            | "y"
            | "h"
            | "re"
            | "S"
            | "s"
            | "f"
            | "F"
            | "f*"
            | "B"
            | "B*"
            | "b"
            | "b*"
            | "n"
            | "W"
            | "W*"
            | "CS"
            | "cs"
            | "SC"
            | "SCN"
            | "sc"
            | "scn"
            | "G"
            | "g"
            | "RG"
            | "rg"
            | "K"
            | "k"
            | "sh"
            | "Do"
            | "BI"
            | "ID"
            | "EI"
            | "MP"
            | "DP"
            | "BMC"
            | "BDC"
            | "EMC"
            | "BX"
            | "EX"
            | "d0"
            | "d1"
    )
}

fn operator_explanation(operator: &str) -> &'static str {
    match operator {
        "BT" => "Begin a text object",
        "ET" => "End a text object",
        "Tf" => "Set text font and size",
        "Td" | "TD" | "Tm" | "T*" => "Move or set text position",
        "Tj" | "TJ" | "'" | "\"" => "Show text",
        "q" => "Save graphics state",
        "Q" => "Restore graphics state",
        "cm" => "Concatenate the current transformation matrix",
        "Do" => "Paint an external object",
        "m" | "l" | "c" | "v" | "y" | "h" | "re" => "Construct a path",
        "S" | "s" | "f" | "F" | "f*" | "B" | "B*" | "b" | "b*" | "n" => {
            "Paint or end the current path"
        }
        "CS" | "cs" | "SC" | "SCN" | "sc" | "scn" | "G" | "g" | "RG" | "rg" | "K" | "k" => {
            "Set color or color space"
        }
        _ if is_known_operator(operator) => "Known PDF content stream operator",
        _ => "Unknown PDF content stream operator",
    }
}

fn skip_whitespace(bytes: &[u8], mut pos: usize) -> usize {
    while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
        pos += 1;
    }
    pos
}

fn is_delimiter(byte: u8) -> bool {
    byte.is_ascii_whitespace()
        || matches!(
            byte,
            b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%'
        )
}

fn decode_name(bytes: &[u8]) -> String {
    let mut output = Vec::new();
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] == b'#' && index + 2 < bytes.len() {
            if let Ok(hex) = std::str::from_utf8(&bytes[index + 1..index + 3]) {
                if let Ok(value) = u8::from_str_radix(hex, 16) {
                    output.push(value);
                    index += 3;
                    continue;
                }
            }
        }
        output.push(bytes[index]);
        index += 1;
    }

    String::from_utf8_lossy(&output).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_operators_and_resources() {
        let analysis = analyze_content_stream(b"BT /F1 12 Tf (Hi) Tj ET /Im1 Do");

        assert_eq!(
            analysis
                .operators
                .iter()
                .map(|operator| operator.name.as_str())
                .collect::<Vec<_>>(),
            vec!["BT", "Tf", "Tj", "ET", "Do"]
        );
        assert_eq!(
            font_names_used(&analysis),
            BTreeSet::from(["F1".to_string()])
        );
        assert_eq!(
            xobject_names_used(&analysis),
            BTreeSet::from(["Im1".to_string()])
        );
        assert!(analysis.warnings.is_empty());
    }

    #[test]
    fn warns_for_unknown_operator() {
        let analysis = analyze_content_stream(b"BT /F1 12 Tf ZZ ET");

        assert!(analysis
            .warnings
            .iter()
            .any(|warning| warning.rule_id == "content.unknown_operator"));
    }

    #[test]
    fn warns_for_unclosed_text_object() {
        let analysis = analyze_content_stream(b"BT /F1 12 Tf (Hi) Tj");

        assert!(analysis
            .warnings
            .iter()
            .any(|warning| warning.rule_id == "content.unclosed_text_object"));
    }
}
