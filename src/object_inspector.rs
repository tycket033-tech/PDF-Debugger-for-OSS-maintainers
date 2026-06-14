use crate::pdf_model::{ByteRange, ObjectRef, PdfObject, PdfValue};
use crate::stream_decode::{decode_stream, DecodeIssue};
use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Serialize)]
pub struct ObjectInspection {
    pub reference: ObjectRef,
    pub object_type: String,
    pub value_summary: String,
    pub raw_range: ByteRange,
    pub raw_length: usize,
    pub dictionary_keys: Vec<String>,
    pub references: Vec<ObjectRef>,
    pub stream: Option<StreamInspection>,
}

#[derive(Clone, Debug, Serialize)]
pub struct StreamInspection {
    pub declared_length: Option<usize>,
    pub actual_length: usize,
    pub raw_range: ByteRange,
    pub filters: Vec<String>,
    pub decoded_length: Option<usize>,
    pub decode_issues: Vec<DecodeIssue>,
}

pub fn inspect_object(object: &PdfObject) -> ObjectInspection {
    inspect_object_with_options(object, true)
}

pub fn inspect_object_shallow(object: &PdfObject) -> ObjectInspection {
    inspect_object_with_options(object, false)
}

fn inspect_object_with_options(object: &PdfObject, decode_streams: bool) -> ObjectInspection {
    let dictionary_keys = object
        .dictionary()
        .map(|dictionary| dictionary.keys().cloned().collect())
        .unwrap_or_default();
    let references = collect_object_references(object);

    let stream = object.stream.as_ref().map(|stream| {
        let decode = decode_streams.then(|| decode_stream(stream));
        StreamInspection {
            declared_length: stream.declared_length,
            actual_length: stream.actual_length,
            raw_range: stream.raw_range,
            filters: stream.filters.clone(),
            decoded_length: decode
                .as_ref()
                .and_then(|decode| (!decode.has_issues()).then_some(decode.decoded_length)),
            decode_issues: decode.map(|decode| decode.issues).unwrap_or_default(),
        }
    });

    ObjectInspection {
        reference: object.reference,
        object_type: object_type(object),
        value_summary: object.value.summary(),
        raw_range: object.raw_range,
        raw_length: if object.raw_bytes.is_empty() {
            object.raw_range.len()
        } else {
            object.raw_bytes.len()
        },
        dictionary_keys,
        references,
        stream,
    }
}

fn collect_object_references(object: &PdfObject) -> Vec<ObjectRef> {
    let mut references = BTreeSet::new();
    collect_references(&object.value, &mut references);
    if let Some(stream) = &object.stream {
        for value in stream.dictionary.values() {
            collect_references(value, &mut references);
        }
    }
    references.into_iter().collect()
}

fn collect_references(value: &PdfValue, references: &mut BTreeSet<ObjectRef>) {
    match value {
        PdfValue::Reference(reference) => {
            references.insert(*reference);
        }
        PdfValue::Array(values) => {
            for value in values {
                collect_references(value, references);
            }
        }
        PdfValue::Dictionary(dictionary) => {
            for value in dictionary.values() {
                collect_references(value, references);
            }
        }
        _ => {}
    }
}

fn object_type(object: &PdfObject) -> String {
    if object.stream.is_some() {
        return "stream".to_string();
    }

    match &object.value {
        PdfValue::Null => "null",
        PdfValue::Boolean(_) => "boolean",
        PdfValue::Number(_) => "number",
        PdfValue::Name(_) => "name",
        PdfValue::String(_) => "string",
        PdfValue::HexString(_) => "hex_string",
        PdfValue::Array(_) => "array",
        PdfValue::Dictionary(_) => "dictionary",
        PdfValue::Reference(_) => "reference",
        PdfValue::Raw(_) => "raw",
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf_parser::parse_bytes;

    #[test]
    fn inspects_dictionary_object() {
        let pdf = parse_bytes(minimal_pdf().as_bytes(), "inspect.pdf");
        let object = pdf.object(ObjectRef::new(1, 0)).unwrap();
        let inspection = inspect_object(object);

        assert_eq!(inspection.object_type, "dictionary");
        assert_eq!(inspection.reference, ObjectRef::new(1, 0));
        assert!(inspection.dictionary_keys.contains(&"Pages".to_string()));
        assert!(inspection.dictionary_keys.contains(&"Type".to_string()));
        assert_eq!(inspection.references, vec![ObjectRef::new(2, 0)]);
        assert!(inspection.stream.is_none());
    }

    #[test]
    fn inspects_stream_object_with_decoded_length() {
        let pdf = parse_bytes(minimal_pdf().as_bytes(), "inspect.pdf");
        let object = pdf.object(ObjectRef::new(4, 0)).unwrap();
        let inspection = inspect_object(object);
        let stream = inspection.stream.unwrap();

        assert_eq!(inspection.object_type, "stream");
        assert!(inspection.references.is_empty());
        assert_eq!(stream.declared_length, Some(23));
        assert_eq!(stream.actual_length, 23);
        assert_eq!(stream.decoded_length, Some(23));
        assert!(stream.filters.is_empty());
        assert!(stream.decode_issues.is_empty());
    }

    fn minimal_pdf() -> String {
        let content = b"BT /F1 12 Tf (Hi) Tj ET";
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"%PDF-1.4\n");
        let object_1 = push(
            &mut bytes,
            b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n",
        );
        let object_2 = push(
            &mut bytes,
            b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 200 200] >>\nendobj\n",
        );
        let object_3 = push(
            &mut bytes,
            b"3 0 obj\n<< /Type /Page /Parent 2 0 R /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>\nendobj\n",
        );
        let mut stream = Vec::new();
        stream.extend_from_slice(
            format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes(),
        );
        stream.extend_from_slice(content);
        stream.extend_from_slice(b"\nendstream\nendobj\n");
        let object_4 = push(&mut bytes, &stream);
        let object_5 = push(
            &mut bytes,
            b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n",
        );
        let xref = bytes.len();
        bytes.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
        for offset in [object_1, object_2, object_3, object_4, object_5] {
            bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        bytes.extend_from_slice(
            format!("trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes(),
        );
        String::from_utf8(bytes).unwrap()
    }

    fn push(bytes: &mut Vec<u8>, value: &[u8]) -> usize {
        let offset = bytes.len();
        bytes.extend_from_slice(value);
        offset
    }
}
