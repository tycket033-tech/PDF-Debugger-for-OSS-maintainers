use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

use crate::pdf_string::decode_pdf_hex_string;

pub type PdfDictionary = BTreeMap<String, PdfValue>;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize)]
pub struct ObjectRef {
    pub object: u32,
    pub generation: u16,
}

impl ObjectRef {
    pub fn new(object: u32, generation: u16) -> Self {
        Self { object, generation }
    }
}

impl fmt::Display for ObjectRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} R", self.object, self.generation)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct ByteRange {
    pub start: usize,
    pub end: usize,
}

impl ByteRange {
    pub fn len(self) -> usize {
        self.end.saturating_sub(self.start)
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", content = "value")]
pub enum PdfValue {
    Null,
    Boolean(bool),
    Number(f64),
    Name(String),
    String(String),
    HexString(String),
    Array(Vec<PdfValue>),
    Dictionary(PdfDictionary),
    Reference(ObjectRef),
    Raw(String),
}

impl PdfValue {
    pub fn as_dictionary(&self) -> Option<&PdfDictionary> {
        match self {
            PdfValue::Dictionary(dict) => Some(dict),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[PdfValue]> {
        match self {
            PdfValue::Array(values) => Some(values),
            _ => None,
        }
    }

    pub fn as_name(&self) -> Option<&str> {
        match self {
            PdfValue::Name(name) => Some(name),
            _ => None,
        }
    }

    pub fn as_reference(&self) -> Option<ObjectRef> {
        match self {
            PdfValue::Reference(reference) => Some(*reference),
            _ => None,
        }
    }

    pub fn as_usize(&self) -> Option<usize> {
        match self {
            PdfValue::Number(value) if *value >= 0.0 && value.fract() == 0.0 => {
                Some(*value as usize)
            }
            _ => None,
        }
    }

    pub fn get(&self, key: &str) -> Option<&PdfValue> {
        self.as_dictionary()?.get(key)
    }

    pub fn summary(&self) -> String {
        match self {
            PdfValue::Null => "null".to_string(),
            PdfValue::Boolean(value) => value.to_string(),
            PdfValue::Number(value) => {
                if value.fract() == 0.0 {
                    format!("{}", *value as i64)
                } else {
                    value.to_string()
                }
            }
            PdfValue::Name(name) => format!("/{name}"),
            PdfValue::String(value) => format!("({})", truncate(value, 48)),
            PdfValue::HexString(value) => {
                let decoded = decode_pdf_hex_string(value);
                if decoded.text.contains('\u{fffd}') || decoded.text.is_empty() {
                    format!("<{}>", truncate(value, 48))
                } else {
                    format!("({})", truncate(&decoded.text, 48))
                }
            }
            PdfValue::Array(values) => format!("[{} item(s)]", values.len()),
            PdfValue::Dictionary(dict) => {
                let keys = dict.keys().take(6).cloned().collect::<Vec<_>>().join(", ");
                format!("<<{}{}>>", keys, if dict.len() > 6 { ", ..." } else { "" })
            }
            PdfValue::Reference(reference) => reference.to_string(),
            PdfValue::Raw(value) => truncate(value, 64),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct PdfStream {
    pub dictionary: PdfDictionary,
    pub declared_length: Option<usize>,
    pub actual_length: usize,
    pub filters: Vec<String>,
    pub raw_range: ByteRange,
    #[serde(skip_serializing)]
    pub raw_bytes: Vec<u8>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PdfObject {
    pub reference: ObjectRef,
    pub value: PdfValue,
    pub stream: Option<PdfStream>,
    pub raw_range: ByteRange,
    #[serde(skip_serializing)]
    pub raw_bytes: Vec<u8>,
}

impl PdfObject {
    pub fn raw_text_lossy(&self) -> String {
        String::from_utf8_lossy(&self.raw_bytes).into_owned()
    }

    pub fn dictionary(&self) -> Option<&PdfDictionary> {
        if let Some(stream) = &self.stream {
            Some(&stream.dictionary)
        } else {
            self.value.as_dictionary()
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct XrefEntry {
    pub reference: ObjectRef,
    pub offset: usize,
    pub in_use: bool,
    pub valid_offset: Option<bool>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ParseWarning {
    pub offset: Option<usize>,
    pub message: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct PdfMetadata {
    pub file_name: String,
    pub file_size: usize,
    pub pdf_version: Option<String>,
    pub page_count: Option<u32>,
    pub encrypted: bool,
    pub linearized: bool,
    pub incremental_update_count: usize,
    pub root: Option<ObjectRef>,
    pub info: Option<ObjectRef>,
    pub trailer_keys: Vec<String>,
    pub info_summary: BTreeMap<String, String>,
    pub object_count: usize,
    pub stream_count: usize,
    pub xref_entry_count: usize,
    pub has_xref_stream: bool,
    pub has_object_stream: bool,
    pub parse_warning_count: usize,
}

#[derive(Clone, Debug)]
pub struct ParsedPdf {
    pub metadata: PdfMetadata,
    pub trailer: Option<PdfValue>,
    pub objects: BTreeMap<ObjectRef, PdfObject>,
    pub xref_entries: Vec<XrefEntry>,
    pub warnings: Vec<ParseWarning>,
}

impl ParsedPdf {
    pub fn object(&self, reference: ObjectRef) -> Option<&PdfObject> {
        self.objects.get(&reference)
    }

    pub fn stream(&self, reference: ObjectRef) -> Option<&PdfStream> {
        self.object(reference)?.stream.as_ref()
    }

    pub fn resolve_reference(&self, value: &PdfValue) -> Option<&PdfObject> {
        self.object(value.as_reference()?)
    }

    pub fn root_catalog(&self) -> Option<&PdfObject> {
        self.object(self.metadata.root?)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

impl Severity {
    pub fn label(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Finding {
    pub rule_id: String,
    pub severity: Severity,
    pub message: String,
    pub object: Option<ObjectRef>,
    pub page: Option<u32>,
    pub byte_offset: Option<usize>,
    pub suggested_next_step: String,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct DiagnosticSummary {
    pub errors: usize,
    pub warnings: usize,
    pub info: usize,
}

impl DiagnosticSummary {
    pub fn from_findings(findings: &[Finding]) -> Self {
        let mut summary = Self::default();
        for finding in findings {
            match finding.severity {
                Severity::Error => summary.errors += 1,
                Severity::Warning => summary.warnings += 1,
                Severity::Info => summary.info += 1,
            }
        }
        summary
    }

    pub fn has_errors(&self) -> bool {
        self.errors > 0
    }
}

fn truncate(value: &str, max_len: usize) -> String {
    if value.chars().count() <= max_len {
        return value.to_string();
    }

    let mut truncated = value
        .chars()
        .take(max_len.saturating_sub(1))
        .collect::<String>();
    truncated.push('…');
    truncated
}
