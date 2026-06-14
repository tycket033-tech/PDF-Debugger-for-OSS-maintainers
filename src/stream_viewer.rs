use crate::content_ops::{analyze_content_stream, ContentAnalysis};
use crate::pdf_model::{ByteRange, ObjectRef, PdfStream};
use crate::stream_decode::{decode_stream, DecodeIssue, DecodeStep};
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct StreamView {
    pub reference: ObjectRef,
    pub declared_length: Option<usize>,
    pub actual_length: usize,
    pub raw_range: ByteRange,
    pub filters: Vec<String>,
    pub decoded_length: Option<usize>,
    pub decode_steps: Vec<DecodeStep>,
    pub decode_issues: Vec<DecodeIssue>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ContentStreamView {
    pub reference: ObjectRef,
    pub decoded_length: usize,
    pub filters: Vec<String>,
    pub decode_steps: Vec<DecodeStep>,
    pub analysis: ContentAnalysis,
}

#[derive(Clone, Debug)]
pub struct ContentStreamViewError {
    pub issues: Vec<DecodeIssue>,
    pub message: String,
}

pub fn inspect_stream(reference: ObjectRef, stream: &PdfStream) -> StreamView {
    let decode = decode_stream(stream);
    StreamView {
        reference,
        declared_length: stream.declared_length,
        actual_length: stream.actual_length,
        raw_range: stream.raw_range,
        filters: stream.filters.clone(),
        decoded_length: (!decode.has_issues()).then_some(decode.decoded_length),
        decode_steps: decode.steps,
        decode_issues: decode.issues,
    }
}

pub fn inspect_content_stream(
    reference: ObjectRef,
    stream: &PdfStream,
) -> Result<ContentStreamView, ContentStreamViewError> {
    ensure_page_content_stream_candidate(reference, stream)?;
    let decode = decode_stream(stream);
    if decode.has_issues() {
        let message = decode
            .issues
            .iter()
            .map(|issue| format!("/{}: {}", issue.filter, issue.message))
            .collect::<Vec<_>>()
            .join("; ");
        return Err(ContentStreamViewError {
            issues: decode.issues,
            message: if message.is_empty() {
                "Content stream analysis requires successfully decoded stream bytes".to_string()
            } else {
                message
            },
        });
    }

    let decoded_length = decode.decoded_length;
    let filters = decode.filters;
    let decode_steps = decode.steps;
    let analysis = analyze_content_stream(&decode.decoded);

    Ok(ContentStreamView {
        reference,
        decoded_length,
        filters,
        decode_steps,
        analysis,
    })
}

fn ensure_page_content_stream_candidate(
    reference: ObjectRef,
    stream: &PdfStream,
) -> Result<(), ContentStreamViewError> {
    let type_name = stream
        .dictionary
        .get("Type")
        .and_then(|value| value.as_name());
    let subtype_name = stream
        .dictionary
        .get("Subtype")
        .and_then(|value| value.as_name());

    if matches!(subtype_name, Some("Image")) {
        return Err(content_stream_type_error(format!(
            "Object {reference} is an image XObject stream, not a PDF page content stream."
        )));
    }

    if matches!(type_name, Some("XObject")) {
        return Err(content_stream_type_error(format!(
            "Object {reference} is an XObject stream ({}) rather than a page content stream.",
            subtype_name.unwrap_or("unknown subtype")
        )));
    }

    Ok(())
}

fn content_stream_type_error(message: String) -> ContentStreamViewError {
    ContentStreamViewError {
        issues: Vec::new(),
        message,
    }
}

pub fn hex_dump(bytes: &[u8]) -> String {
    let mut output = String::new();
    for (row, chunk) in bytes.chunks(16).enumerate() {
        let offset = row * 16;
        output.push_str(&format!("{offset:08x}  "));

        for index in 0..16 {
            if let Some(byte) = chunk.get(index) {
                output.push_str(&format!("{byte:02x} "));
            } else {
                output.push_str("   ");
            }
            if index == 7 {
                output.push(' ');
            }
        }

        output.push(' ');
        for byte in chunk {
            if byte.is_ascii_graphic() || *byte == b' ' {
                output.push(*byte as char);
            } else {
                output.push('.');
            }
        }
        output.push('\n');
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf_model::{PdfDictionary, PdfValue};

    #[test]
    fn renders_hex_dump() {
        let dump = hex_dump(b"BT /F1 12 Tf");

        assert!(dump.contains("00000000"));
        assert!(dump.contains("42 54 20 2f"));
        assert!(dump.contains("BT /F1 12 Tf"));
    }

    #[test]
    fn inspects_stream_without_embedding_bytes() {
        let mut dictionary = PdfDictionary::new();
        dictionary.insert("Length".to_string(), PdfValue::Number(5.0));
        let stream = PdfStream {
            dictionary,
            declared_length: Some(5),
            actual_length: 5,
            filters: Vec::new(),
            raw_range: ByteRange { start: 10, end: 15 },
            raw_bytes: b"Hello".to_vec(),
        };

        let view = inspect_stream(ObjectRef::new(4, 0), &stream);

        assert_eq!(view.reference, ObjectRef::new(4, 0));
        assert_eq!(view.declared_length, Some(5));
        assert_eq!(view.actual_length, 5);
        assert_eq!(view.decoded_length, Some(5));
        assert!(view.decode_steps.is_empty());
        assert!(view.decode_issues.is_empty());
    }

    #[test]
    fn inspects_decoded_content_stream() {
        let stream = PdfStream {
            dictionary: PdfDictionary::new(),
            declared_length: Some(23),
            actual_length: 23,
            filters: Vec::new(),
            raw_range: ByteRange { start: 0, end: 23 },
            raw_bytes: b"BT /F1 12 Tf (Hi) Tj ET".to_vec(),
        };

        let view = inspect_content_stream(ObjectRef::new(4, 0), &stream).unwrap();

        assert_eq!(view.reference, ObjectRef::new(4, 0));
        assert_eq!(view.decoded_length, 23);
        assert_eq!(
            view.analysis
                .operators
                .iter()
                .map(|operator| operator.name.as_str())
                .collect::<Vec<_>>(),
            vec!["BT", "Tf", "Tj", "ET"]
        );
        assert!(view.analysis.warnings.is_empty());
    }

    #[test]
    fn skips_image_xobject_for_content_analysis() {
        let mut dictionary = PdfDictionary::new();
        dictionary.insert("Type".to_string(), PdfValue::Name("XObject".to_string()));
        dictionary.insert("Subtype".to_string(), PdfValue::Name("Image".to_string()));
        dictionary.insert("Length".to_string(), PdfValue::Number(4.0));
        let stream = PdfStream {
            dictionary,
            declared_length: Some(4),
            actual_length: 4,
            filters: Vec::new(),
            raw_range: ByteRange { start: 10, end: 14 },
            raw_bytes: vec![0, 255, 128, 64],
        };

        let error = inspect_content_stream(ObjectRef::new(19, 0), &stream)
            .expect_err("image XObject should not be parsed as content operators");

        assert!(error.issues.is_empty());
        assert!(error.message.contains("image XObject stream"));
    }
}
