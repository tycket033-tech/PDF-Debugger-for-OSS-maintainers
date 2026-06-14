use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::pdf_model::{ObjectRef, ParsedPdf, PdfDictionary, PdfObject, PdfStream, PdfValue};
use crate::pdf_parser::{parse_file, parse_pdf_value};
use crate::stream_decode::encode_filter_chain;
use crate::{PdfDebuggerError, Result};

#[derive(Clone, Debug)]
pub enum PdfEditPathSegment {
    DictionaryKey(String),
    ArrayIndex(usize),
}

#[derive(Clone, Debug)]
pub struct PdfObjectEdit {
    pub reference: ObjectRef,
    pub path: Vec<PdfEditPathSegment>,
    pub value: PdfValue,
}

#[derive(Clone, Debug)]
pub struct PdfStreamEdit {
    pub reference: ObjectRef,
    pub decoded_text: String,
}

#[derive(Clone, Debug)]
pub struct PdfSaveAsResult {
    pub bytes_written: usize,
    pub object_count: usize,
}

pub fn parse_edit_value(input: &str) -> Result<PdfValue> {
    let value =
        parse_pdf_value(input.trim().as_bytes()).ok_or_else(|| PdfDebuggerError::Parse {
            offset: 0,
            message: "Could not parse edited PDF value.".to_string(),
        })?;
    ensure_editable_object_value(&value)?;
    Ok(value)
}

pub fn save_modified_pdf_as(
    input_path: &Path,
    output_path: &Path,
    edits: &[PdfObjectEdit],
    stream_edits: &[PdfStreamEdit],
) -> Result<PdfSaveAsResult> {
    let (bytes, object_count) = modified_pdf_bytes(input_path, edits, stream_edits)?;
    fs::write(output_path, &bytes)?;
    parse_file(output_path)?;
    Ok(PdfSaveAsResult {
        bytes_written: bytes.len(),
        object_count,
    })
}

pub fn save_modified_pdf_in_place(
    input_path: &Path,
    edits: &[PdfObjectEdit],
    stream_edits: &[PdfStreamEdit],
) -> Result<PdfSaveAsResult> {
    let (bytes, object_count) = modified_pdf_bytes(input_path, edits, stream_edits)?;
    let parent = input_path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = input_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("document.pdf");
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let temp_path = parent.join(format!(".{file_name}.pdf-debugger-{nonce}.tmp"));
    let backup_path = backup_path_for(input_path, nonce);

    let write_result = (|| -> Result<()> {
        fs::write(&temp_path, &bytes)?;
        parse_file(&temp_path)?;
        fs::rename(input_path, &backup_path)?;
        match fs::rename(&temp_path, input_path) {
            Ok(()) => {
                let _ = fs::remove_file(&backup_path);
                Ok(())
            }
            Err(error) => {
                let _ = fs::rename(&backup_path, input_path);
                Err(PdfDebuggerError::Io(error))
            }
        }
    })();

    if write_result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    write_result?;

    Ok(PdfSaveAsResult {
        bytes_written: bytes.len(),
        object_count,
    })
}

fn modified_pdf_bytes(
    input_path: &Path,
    edits: &[PdfObjectEdit],
    stream_edits: &[PdfStreamEdit],
) -> Result<(Vec<u8>, usize)> {
    let mut pdf = parse_file(input_path)?;
    for edit in edits {
        apply_object_edit(&mut pdf, edit)?;
    }
    for edit in stream_edits {
        apply_stream_edit(&mut pdf, edit)?;
    }
    let object_count = pdf.objects.len();
    let bytes = serialize_pdf(&pdf)?;
    Ok((bytes, object_count))
}

fn backup_path_for(input_path: &Path, nonce: u128) -> PathBuf {
    let parent = input_path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = input_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("document.pdf");
    parent.join(format!(".{file_name}.pdf-debugger-{nonce}.bak"))
}

fn apply_stream_edit(pdf: &mut ParsedPdf, edit: &PdfStreamEdit) -> Result<()> {
    let object = pdf
        .objects
        .get_mut(&edit.reference)
        .ok_or(PdfDebuggerError::ObjectNotFound {
            reference: edit.reference,
        })?;
    let Some(stream) = object.stream.as_mut() else {
        return Err(PdfDebuggerError::StreamNotFound {
            reference: edit.reference,
        });
    };

    let encoded =
        encode_filter_chain(edit.decoded_text.as_bytes(), &stream.filters).map_err(|message| {
            PdfDebuggerError::StreamDecode {
                reference: edit.reference,
                message,
            }
        })?;
    stream.raw_bytes = encoded;
    stream.actual_length = stream.raw_bytes.len();
    stream.declared_length = Some(stream.raw_bytes.len());
    stream.dictionary.insert(
        "Length".to_string(),
        PdfValue::Number(stream.raw_bytes.len() as f64),
    );
    object.value = PdfValue::Dictionary(stream.dictionary.clone());
    Ok(())
}

fn apply_object_edit(pdf: &mut ParsedPdf, edit: &PdfObjectEdit) -> Result<()> {
    ensure_editable_object_value(&edit.value)?;
    let object = pdf
        .objects
        .get_mut(&edit.reference)
        .ok_or(PdfDebuggerError::ObjectNotFound {
            reference: edit.reference,
        })?;

    if edit.path.is_empty() {
        ensure_editable_object_value(&object.value)?;
        set_object_value(object, edit.value.clone());
        return Ok(());
    }

    let target = object_edit_root_mut(object);
    let result = apply_value_edit(target, &edit.path, edit.value.clone());
    if result.is_ok() {
        sync_stream_dictionary_from_value(object);
    }
    result
}

fn object_edit_root_mut(object: &mut PdfObject) -> &mut PdfValue {
    if let Some(stream) = object.stream.as_mut() {
        object.value = PdfValue::Dictionary(stream.dictionary.clone());
    }
    &mut object.value
}

fn set_object_value(object: &mut PdfObject, value: PdfValue) {
    object.value = value;
    sync_stream_dictionary_from_value(object);
}

fn sync_stream_dictionary_from_value(object: &mut PdfObject) {
    if let Some(stream) = object.stream.as_mut() {
        if let PdfValue::Dictionary(dictionary) = &object.value {
            update_stream_dictionary(stream, dictionary.clone());
        }
    }
}

fn ensure_editable_object_value(value: &PdfValue) -> Result<()> {
    if matches!(
        value,
        PdfValue::Number(_) | PdfValue::String(_) | PdfValue::Name(_)
    ) {
        return Ok(());
    }

    Err(PdfDebuggerError::Parse {
        offset: 0,
        message: "Only Number, String, and Name values can be edited in Object Inspector."
            .to_string(),
    })
}

fn apply_value_edit(
    current: &mut PdfValue,
    path: &[PdfEditPathSegment],
    replacement: PdfValue,
) -> Result<()> {
    let Some((head, tail)) = path.split_first() else {
        *current = replacement;
        return Ok(());
    };

    match head {
        PdfEditPathSegment::DictionaryKey(key) => {
            let PdfValue::Dictionary(dictionary) = current else {
                return Err(PdfDebuggerError::Parse {
                    offset: 0,
                    message: format!("Cannot edit /{key}; parent is not a dictionary."),
                });
            };
            let child = dictionary
                .get_mut(key)
                .ok_or_else(|| PdfDebuggerError::Parse {
                    offset: 0,
                    message: format!("Dictionary key /{key} was not found."),
                })?;
            if tail.is_empty() {
                ensure_editable_object_value(child)?;
                ensure_editable_object_value(&replacement)?;
                *child = replacement;
                return Ok(());
            }
            apply_value_edit(child, tail, replacement)
        }
        PdfEditPathSegment::ArrayIndex(index) => {
            let PdfValue::Array(values) = current else {
                return Err(PdfDebuggerError::Parse {
                    offset: 0,
                    message: format!("Cannot edit [{index}]; parent is not an array."),
                });
            };
            if *index >= values.len() {
                return Err(PdfDebuggerError::Parse {
                    offset: 0,
                    message: format!("Array index [{index}] is out of range."),
                });
            }
            if tail.is_empty() {
                ensure_editable_object_value(&values[*index])?;
                ensure_editable_object_value(&replacement)?;
                values[*index] = replacement;
                return Ok(());
            }
            apply_value_edit(&mut values[*index], tail, replacement)
        }
    }
}

fn update_stream_dictionary(stream: &mut PdfStream, dictionary: PdfDictionary) {
    stream.declared_length = dictionary.get("Length").and_then(PdfValue::as_usize);
    stream.filters = filter_names_from_dictionary(&dictionary);
    stream.dictionary = dictionary;
}

fn filter_names_from_dictionary(dictionary: &PdfDictionary) -> Vec<String> {
    match dictionary.get("Filter") {
        Some(PdfValue::Name(name)) => vec![name.clone()],
        Some(PdfValue::Array(filters)) => filters
            .iter()
            .filter_map(|value| value.as_name().map(str::to_string))
            .collect(),
        _ => Vec::new(),
    }
}

pub fn serialize_pdf(pdf: &ParsedPdf) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n");

    let mut offsets = Vec::new();
    offsets.push(0usize);
    for object in pdf.objects.values() {
        offsets.push(bytes.len());
        bytes.extend_from_slice(
            format!(
                "{} {} obj\n",
                object.reference.object, object.reference.generation
            )
            .as_bytes(),
        );
        write_object_body(&mut bytes, object);
        bytes.extend_from_slice(b"\nendobj\n");
    }

    let xref_offset = bytes.len();
    bytes.extend_from_slice(format!("xref\n0 {}\n", offsets.len()).as_bytes());
    bytes.extend_from_slice(b"0000000000 65535 f \n");
    for offset in offsets.iter().skip(1) {
        bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }

    let trailer = writable_trailer(pdf, offsets.len());
    bytes.extend_from_slice(b"trailer\n");
    write_pdf_value(&mut bytes, &trailer);
    bytes.extend_from_slice(format!("\nstartxref\n{xref_offset}\n%%EOF\n").as_bytes());
    Ok(bytes)
}

fn write_object_body(bytes: &mut Vec<u8>, object: &PdfObject) {
    if let Some(stream) = &object.stream {
        let mut dictionary = stream.dictionary.clone();
        dictionary
            .entry("Length".to_string())
            .or_insert_with(|| PdfValue::Number(stream.raw_bytes.len() as f64));
        write_pdf_value(bytes, &PdfValue::Dictionary(dictionary));
        bytes.extend_from_slice(b"\nstream\n");
        bytes.extend_from_slice(&stream.raw_bytes);
        bytes.extend_from_slice(b"\nendstream");
    } else {
        write_pdf_value(bytes, &object.value);
    }
}

fn writable_trailer(pdf: &ParsedPdf, size: usize) -> PdfValue {
    let mut trailer = match pdf.trailer.as_ref().and_then(PdfValue::as_dictionary) {
        Some(dictionary) => dictionary.clone(),
        None => PdfDictionary::new(),
    };
    trailer.insert("Size".to_string(), PdfValue::Number(size as f64));
    trailer.remove("Prev");
    trailer.remove("XRefStm");
    PdfValue::Dictionary(trailer)
}

fn write_pdf_value(bytes: &mut Vec<u8>, value: &PdfValue) {
    match value {
        PdfValue::Null => bytes.extend_from_slice(b"null"),
        PdfValue::Boolean(value) => {
            bytes.extend_from_slice(if *value { b"true" } else { b"false" })
        }
        PdfValue::Number(value) => write_number(bytes, *value),
        PdfValue::Name(name) => {
            bytes.push(b'/');
            bytes.extend_from_slice(escape_pdf_name(name).as_bytes());
        }
        PdfValue::String(value) => {
            bytes.push(b'(');
            bytes.extend_from_slice(escape_literal_string(value).as_bytes());
            bytes.push(b')');
        }
        PdfValue::HexString(value) => {
            bytes.push(b'<');
            bytes.extend_from_slice(value.as_bytes());
            bytes.push(b'>');
        }
        PdfValue::Array(values) => {
            bytes.push(b'[');
            for (index, item) in values.iter().enumerate() {
                if index > 0 {
                    bytes.push(b' ');
                }
                write_pdf_value(bytes, item);
            }
            bytes.push(b']');
        }
        PdfValue::Dictionary(dictionary) => {
            bytes.extend_from_slice(b"<<");
            for (key, item) in dictionary {
                bytes.push(b' ');
                bytes.push(b'/');
                bytes.extend_from_slice(escape_pdf_name(key).as_bytes());
                bytes.push(b' ');
                write_pdf_value(bytes, item);
            }
            bytes.extend_from_slice(b" >>");
        }
        PdfValue::Reference(reference) => {
            bytes.extend_from_slice(reference.to_string().as_bytes());
        }
        PdfValue::Raw(value) => bytes.extend_from_slice(value.as_bytes()),
    }
}

fn write_number(bytes: &mut Vec<u8>, value: f64) {
    if value.fract() == 0.0 && value >= i64::MIN as f64 && value <= i64::MAX as f64 {
        bytes.extend_from_slice(format!("{}", value as i64).as_bytes());
    } else {
        bytes.extend_from_slice(value.to_string().as_bytes());
    }
}

fn escape_literal_string(value: &str) -> String {
    value
        .chars()
        .flat_map(|character| match character {
            '(' => "\\(".chars().collect::<Vec<_>>(),
            ')' => "\\)".chars().collect::<Vec<_>>(),
            '\\' => "\\\\".chars().collect::<Vec<_>>(),
            '\n' => "\\n".chars().collect::<Vec<_>>(),
            '\r' => "\\r".chars().collect::<Vec<_>>(),
            '\t' => "\\t".chars().collect::<Vec<_>>(),
            other => vec![other],
        })
        .collect()
}

fn escape_pdf_name(value: &str) -> String {
    value
        .chars()
        .flat_map(|character| {
            if character.is_ascii_whitespace()
                || matches!(
                    character,
                    '(' | ')' | '<' | '>' | '[' | ']' | '{' | '}' | '/' | '%'
                )
            {
                format!("#{:02X}", character as u32).chars().collect()
            } else {
                vec![character]
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf_parser::parse_bytes;
    use crate::stream_decode::decode_stream;

    #[test]
    fn applies_dictionary_value_edit_and_serializes_pdf() {
        let bytes = b"%PDF-1.7
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj
2 0 obj
<< /Type /Pages /Count 0 /Kids [] >>
endobj
xref
0 3
0000000000 65535 f 
0000000009 00000 n 
0000000060 00000 n 
trailer
<< /Size 3 /Root 1 0 R >>
startxref
111
%%EOF
";
        let mut pdf = parse_bytes(bytes, "edit.pdf");
        apply_object_edit(
            &mut pdf,
            &PdfObjectEdit {
                reference: ObjectRef::new(2, 0),
                path: vec![PdfEditPathSegment::DictionaryKey("Count".to_string())],
                value: PdfValue::Number(1.0),
            },
        )
        .expect("apply edit");
        let output = serialize_pdf(&pdf).expect("serialize");
        let parsed = parse_bytes(&output, "modified.pdf");
        let count = parsed
            .object(ObjectRef::new(2, 0))
            .and_then(PdfObject::dictionary)
            .and_then(|dictionary| dictionary.get("Count"))
            .and_then(PdfValue::as_usize);
        assert_eq!(count, Some(1));
    }

    #[test]
    fn rejects_object_edit_to_non_scalar_value() {
        assert!(parse_edit_value("[0 0 10 10]").is_err());
        assert!(parse_edit_value("<< /K /V >>").is_err());
        assert!(parse_edit_value("true").is_err());
        assert!(parse_edit_value("null").is_err());
        assert!(parse_edit_value("3 0 R").is_err());
        assert!(parse_edit_value("65").is_ok());
        assert!(parse_edit_value("/FlateDecode").is_ok());
        assert!(parse_edit_value("(Text)").is_ok());
    }

    #[test]
    fn rejects_object_edit_when_existing_value_is_not_editable() {
        let bytes = b"%PDF-1.7
1 0 obj
<< /Type /Catalog /BBox [0 0 10 10] /Enabled true >>
endobj
xref
0 2
0000000000 65535 f
0000000009 00000 n
trailer
<< /Size 2 /Root 1 0 R >>
startxref
76
%%EOF
";
        let mut pdf = parse_bytes(bytes, "edit-reject.pdf");
        let array_result = apply_object_edit(
            &mut pdf,
            &PdfObjectEdit {
                reference: ObjectRef::new(1, 0),
                path: vec![PdfEditPathSegment::DictionaryKey("BBox".to_string())],
                value: PdfValue::Number(4.0),
            },
        );
        assert!(array_result.is_err());

        let boolean_result = apply_object_edit(
            &mut pdf,
            &PdfObjectEdit {
                reference: ObjectRef::new(1, 0),
                path: vec![PdfEditPathSegment::DictionaryKey("Enabled".to_string())],
                value: PdfValue::Name("False".to_string()),
            },
        );
        assert!(boolean_result.is_err());
    }

    #[test]
    fn applies_flate_content_stream_edit_and_updates_length() {
        let raw = encode_filter_chain(b"BT (Old) Tj ET", &["FlateDecode".to_string()])
            .expect("encode fixture stream");
        let mut bytes = b"%PDF-1.7\n".to_vec();
        let object_offset = bytes.len();
        bytes.extend_from_slice(
            format!(
                "1 0 obj\n<< /Length {} /Filter /FlateDecode >>\nstream\n",
                raw.len()
            )
            .as_bytes(),
        );
        bytes.extend_from_slice(&raw);
        bytes.extend_from_slice(b"\nendstream\nendobj\n");
        let xref = bytes.len();
        bytes.extend_from_slice(
            format!(
                "xref\n0 2\n0000000000 65535 f \n{object_offset:010} 00000 n \ntrailer\n<< /Size 2 >>\nstartxref\n{xref}\n%%EOF\n"
            )
            .as_bytes(),
        );
        let mut pdf = parse_bytes(&bytes, "stream-edit.pdf");
        apply_stream_edit(
            &mut pdf,
            &PdfStreamEdit {
                reference: ObjectRef::new(1, 0),
                decoded_text: "BT (New) Tj ET".to_string(),
            },
        )
        .expect("apply stream edit");
        let output = serialize_pdf(&pdf).expect("serialize");
        let parsed = parse_bytes(&output, "modified-stream.pdf");
        let stream = parsed.stream(ObjectRef::new(1, 0)).expect("edited stream");
        let decoded = decode_stream(stream);

        assert!(!decoded.has_issues());
        assert_eq!(decoded.decoded, b"BT (New) Tj ET");
        assert_eq!(stream.declared_length, Some(stream.raw_bytes.len()));
    }

    #[test]
    fn preserves_explicit_stream_dictionary_length_edit() {
        let bytes = b"%PDF-1.7
1 0 obj
<< /Length 5 >>
stream
abcde
endstream
endobj
xref
0 2
0000000000 65535 f
0000000009 00000 n
trailer
<< /Size 2 >>
startxref
60
%%EOF
";
        let mut pdf = parse_bytes(bytes, "stream-length-edit.pdf");
        apply_object_edit(
            &mut pdf,
            &PdfObjectEdit {
                reference: ObjectRef::new(1, 0),
                path: vec![PdfEditPathSegment::DictionaryKey("Length".to_string())],
                value: PdfValue::Number(80.0),
            },
        )
        .expect("apply stream dictionary length edit");

        let output = serialize_pdf(&pdf).expect("serialize");
        let parsed = parse_bytes(&output, "modified-stream-length.pdf");
        let stream = parsed.stream(ObjectRef::new(1, 0)).expect("stream");

        assert_eq!(stream.declared_length, Some(80));
        assert_eq!(stream.actual_length, 5);
    }

    #[test]
    fn saves_modified_pdf_in_place_and_reopens_path() {
        let mut path = std::env::temp_dir();
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        path.push(format!("pdf-debugger-in-place-{nonce}.pdf"));
        let bytes = b"%PDF-1.7
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj
2 0 obj
<< /Type /Pages /Count 0 /Kids [] >>
endobj
xref
0 3
0000000000 65535 f
0000000009 00000 n
0000000060 00000 n
trailer
<< /Size 3 /Root 1 0 R >>
startxref
111
%%EOF
";
        fs::write(&path, bytes).expect("write source pdf");

        let result = save_modified_pdf_in_place(
            &path,
            &[PdfObjectEdit {
                reference: ObjectRef::new(2, 0),
                path: vec![PdfEditPathSegment::DictionaryKey("Count".to_string())],
                value: PdfValue::Number(3.0),
            }],
            &[],
        )
        .expect("save in place");

        assert!(result.bytes_written > 0);
        let parsed = parse_file(&path).expect("reopen saved pdf");
        let count = parsed
            .object(ObjectRef::new(2, 0))
            .and_then(PdfObject::dictionary)
            .and_then(|dictionary| dictionary.get("Count"))
            .and_then(PdfValue::as_usize);
        assert_eq!(count, Some(3));

        let _ = fs::remove_file(&path);
    }
}
