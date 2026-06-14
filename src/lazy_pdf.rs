use crate::object_inspector::{inspect_object_shallow, ObjectInspection};
use crate::object_tree::{ObjectTreeNode, ObjectTreeNodeKind};
use crate::page_index::{
    PageBox, PageIndex, PageObjectLink, PageObjectLinkKind, PageResourceSummary, PageSummary,
};
use crate::pdf_model::{
    ByteRange, ObjectRef, PdfDictionary, PdfMetadata, PdfObject, PdfStream, PdfValue, XrefEntry,
};
use crate::pdf_parser::parse_pdf_value_with_consumed;
use crate::stream_decode::{
    decode_stream_with_limit, filter_names_from_dictionary, DecodeIssue, DecodeStep,
};
use crate::stream_viewer::hex_dump;
use crate::{PdfDebuggerError, Result};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

const HEADER_READ_LIMIT: usize = 1024;
const TAIL_READ_LIMIT: u64 = 1024 * 1024;
const OBJECT_INITIAL_READ_LIMIT: usize = 64 * 1024;
const OBJECT_READ_LIMIT: usize = 2 * 1024 * 1024;
const XREF_READ_LIMIT: usize = 16 * 1024 * 1024;
const XREF_STREAM_DECODE_LIMIT: usize = 32 * 1024 * 1024;
const XREF_CHAIN_MAX_SECTIONS: usize = 32;
const OBJECT_TREE_XREF_CHILD_LIMIT: usize = 512;
const PAGE_TREE_MAX_DEPTH: usize = 64;
const PAGE_TREE_MAX_NODES: usize = 20_000;
const PAGE_TREE_MAX_PAGES: usize = 20_000;
pub const LAZY_STREAM_PREVIEW_LIMIT: usize = 1024 * 1024;
pub const LAZY_STREAM_DECODE_INPUT_LIMIT: usize = 16 * 1024 * 1024;
pub const LAZY_STREAM_DECODE_OUTPUT_LIMIT: usize = 32 * 1024 * 1024;
pub const LAZY_STREAM_EXPORT_LIMIT: usize = 512 * 1024 * 1024;
pub const LAZY_STREAM_DECODE_EXPORT_LIMIT: usize = 256 * 1024 * 1024;

#[derive(Clone, Debug, Serialize)]
pub struct LazyPdfDocument {
    pub path: PathBuf,
    pub metadata: PdfMetadata,
    pub trailer: Option<PdfValue>,
    pub xref: LazyXrefIndex,
    pub warnings: Vec<LazyPdfWarning>,
}

#[derive(Clone, Debug, Serialize)]
pub struct LazyXrefIndex {
    pub startxref: Option<usize>,
    pub entries: Vec<LazyXrefEntry>,
}

#[derive(Clone, Debug, Serialize)]
pub struct LazyXrefEntry {
    pub reference: ObjectRef,
    pub offset: usize,
    pub in_use: bool,
    pub generation: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compressed: Option<LazyCompressedObjectEntry>,
}

#[derive(Clone, Debug, Serialize)]
pub struct LazyCompressedObjectEntry {
    pub object_stream: ObjectRef,
    pub index: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct LazyPdfWarning {
    pub rule_id: String,
    pub message: String,
    pub byte_offset: Option<usize>,
}

#[derive(Clone, Debug)]
struct LazyXrefSection {
    entries: Vec<LazyXrefEntry>,
    trailer: Option<PdfValue>,
}

#[derive(Clone, Debug, Serialize)]
pub struct LazyStreamView {
    pub reference: ObjectRef,
    pub declared_length: Option<usize>,
    pub actual_length: usize,
    pub raw_range: ByteRange,
    pub filters: Vec<String>,
    pub decoded_length: Option<usize>,
    pub decode_steps: Vec<DecodeStep>,
    pub decode_issues: Vec<DecodeIssue>,
    pub warnings: Vec<LazyPdfWarning>,
    pub raw_text: String,
    pub raw_text_truncated: bool,
    pub hex_text: String,
    pub hex_text_truncated: bool,
    pub decoded_text: Option<String>,
    pub decoded_text_truncated: bool,
    pub decoded_error: Option<String>,
    pub preview_limit: usize,
    pub can_export_raw: bool,
    pub can_export_decoded: bool,
}

pub fn open_lazy_pdf(path: &Path) -> Result<LazyPdfDocument> {
    let file_metadata = std::fs::metadata(path)?;
    let file_size = file_metadata.len();
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string());
    let mut warnings = Vec::new();
    let mut file = std::fs::File::open(path)?;

    let header = read_at(&mut file, 0, HEADER_READ_LIMIT.min(file_size as usize))?;
    let pdf_version = parse_pdf_version(&header, &mut warnings);
    let linearized = header
        .get(..header.len().min(1024))
        .is_some_and(|prefix| find_subslice(prefix, b"/Linearized").is_some());

    let tail_len = TAIL_READ_LIMIT.min(file_size) as usize;
    let tail_start = file_size.saturating_sub(tail_len as u64);
    let tail = read_at(&mut file, tail_start, tail_len)?;
    let incremental_update_count = count_occurrences(&tail, b"%%EOF").saturating_sub(1);
    let startxref = parse_startxref(&tail, tail_start as usize, &mut warnings);

    let (entries, trailer) = if let Some(startxref) = startxref {
        parse_xref_at(&mut file, file_size, startxref, &mut warnings)?
    } else {
        (Vec::new(), None)
    };

    let root = trailer
        .as_ref()
        .and_then(PdfValue::as_dictionary)
        .and_then(|dict| dict.get("Root"))
        .and_then(PdfValue::as_reference);
    let info = trailer
        .as_ref()
        .and_then(PdfValue::as_dictionary)
        .and_then(|dict| dict.get("Info"))
        .and_then(PdfValue::as_reference);
    let encrypted = trailer
        .as_ref()
        .and_then(PdfValue::as_dictionary)
        .is_some_and(|dict| dict.contains_key("Encrypt"));
    let trailer_keys = trailer
        .as_ref()
        .and_then(PdfValue::as_dictionary)
        .map(|dict| dict.keys().cloned().collect())
        .unwrap_or_default();

    let xref = LazyXrefIndex { startxref, entries };
    let page_count =
        root.and_then(|root| detect_page_count_lazy(&mut file, &xref, root, &mut warnings));

    let has_xref_stream = trailer
        .as_ref()
        .and_then(PdfValue::as_dictionary)
        .is_some_and(|dict| {
            dict.get("Type")
                .and_then(PdfValue::as_name)
                .is_some_and(|name| name == "XRef")
                || dict.contains_key("XRefStm")
        });
    let has_object_stream = xref.entries.iter().any(|entry| entry.compressed.is_some());
    let xref_entry_count = xref.entries.len();
    let object_count = xref.entries.iter().filter(|entry| entry.in_use).count();

    Ok(LazyPdfDocument {
        path: path.to_path_buf(),
        metadata: PdfMetadata {
            file_name,
            file_size: file_size.min(usize::MAX as u64) as usize,
            pdf_version,
            page_count,
            encrypted,
            linearized,
            incremental_update_count,
            root,
            info,
            trailer_keys,
            info_summary: BTreeMap::new(),
            object_count,
            stream_count: 0,
            xref_entry_count,
            has_xref_stream,
            has_object_stream,
            parse_warning_count: warnings.len(),
        },
        trailer,
        xref,
        warnings,
    })
}

pub fn inspect_lazy_object(path: &Path, reference: ObjectRef) -> Result<ObjectInspection> {
    let document = open_lazy_pdf(path)?;
    inspect_lazy_object_with_document(&document, reference)
}

pub fn inspect_lazy_object_with_document(
    document: &LazyPdfDocument,
    reference: ObjectRef,
) -> Result<ObjectInspection> {
    let object = read_lazy_object(&document.path, &document.xref, reference)?;
    Ok(inspect_object_shallow(&object))
}

pub fn read_lazy_object_for_tree(path: &Path, reference: ObjectRef) -> Result<PdfObject> {
    let document = open_lazy_pdf(path)?;
    read_lazy_object_for_tree_with_document(&document, reference)
}

pub fn read_lazy_object_for_tree_with_document(
    document: &LazyPdfDocument,
    reference: ObjectRef,
) -> Result<PdfObject> {
    read_lazy_object(&document.path, &document.xref, reference)
}

pub fn view_lazy_stream(path: &Path, reference: ObjectRef) -> Result<LazyStreamView> {
    let document = open_lazy_pdf(path)?;
    view_lazy_stream_with_document(&document, reference)
}

pub fn view_lazy_stream_with_document(
    document: &LazyPdfDocument,
    reference: ObjectRef,
) -> Result<LazyStreamView> {
    let metadata = read_lazy_stream_metadata(&document.path, &document.xref, reference)?;
    let full_length = metadata.actual_length;
    let read_full_for_decode = full_length <= LAZY_STREAM_DECODE_INPUT_LIMIT;
    let read_length = if read_full_for_decode {
        full_length
    } else {
        full_length.min(LAZY_STREAM_PREVIEW_LIMIT)
    };
    let mut stream = metadata.clone();
    stream.raw_bytes = read_lazy_stream_bytes(&document.path, &metadata, read_length)?;

    let raw_preview_length = stream.raw_bytes.len().min(LAZY_STREAM_PREVIEW_LIMIT);
    let raw_text = String::from_utf8_lossy(&stream.raw_bytes[..raw_preview_length]).into_owned();
    let raw_text_truncated = full_length > raw_preview_length;
    let hex_text = hex_dump(&stream.raw_bytes[..raw_preview_length]);
    let hex_text_truncated = raw_text_truncated;

    let mut warnings = Vec::new();
    if raw_text_truncated {
        warnings.push(LazyPdfWarning {
            rule_id: "lazy.stream.preview_truncated".to_string(),
            message: format!(
                "Stream preview is truncated to {} bytes from {} total bytes",
                LAZY_STREAM_PREVIEW_LIMIT, full_length
            ),
            byte_offset: Some(metadata.raw_range.start),
        });
    }

    let mut decoded_length = None;
    let mut decoded_text = None;
    let mut decoded_text_truncated = false;
    let mut decoded_error = None;
    let mut decode_steps = Vec::new();
    let mut decode_issues = Vec::new();

    if read_full_for_decode {
        let decode = decode_stream_with_limit(&stream, Some(LAZY_STREAM_DECODE_OUTPUT_LIMIT));
        decoded_length = (!decode.has_issues()).then_some(decode.decoded_length);
        decode_steps = decode.steps;
        decode_issues = decode.issues;
        if decode_issues.is_empty() {
            let preview_length = decode.decoded.len().min(LAZY_STREAM_PREVIEW_LIMIT);
            decoded_text =
                Some(String::from_utf8_lossy(&decode.decoded[..preview_length]).into_owned());
            decoded_text_truncated = decode.decoded.len() > preview_length;
            if decoded_text_truncated {
                warnings.push(LazyPdfWarning {
                    rule_id: "lazy.stream.decoded_preview_truncated".to_string(),
                    message: format!(
                        "Decoded stream preview is truncated to {} bytes from {} decoded bytes",
                        LAZY_STREAM_PREVIEW_LIMIT,
                        decode.decoded.len()
                    ),
                    byte_offset: Some(metadata.raw_range.start),
                });
            }
        } else {
            decoded_error = Some(format_decode_issues(&decode_issues));
        }
    } else {
        warnings.push(LazyPdfWarning {
            rule_id: "lazy.stream.decode_preview_skipped".to_string(),
            message: format!(
                "Decoded preview skipped because raw stream length {full_length} exceeds lazy decode input limit {LAZY_STREAM_DECODE_INPUT_LIMIT}"
            ),
            byte_offset: Some(metadata.raw_range.start),
        });
        decoded_error = Some(format!(
            "Decoded preview skipped because raw stream length exceeds {} bytes",
            LAZY_STREAM_DECODE_INPUT_LIMIT
        ));
    }

    let can_export_raw = full_length <= LAZY_STREAM_EXPORT_LIMIT;
    if !can_export_raw {
        warnings.push(LazyPdfWarning {
            rule_id: "lazy.stream.raw_export_limit".to_string(),
            message: format!(
                "Raw stream export is disabled because stream length {full_length} exceeds export limit {LAZY_STREAM_EXPORT_LIMIT}"
            ),
            byte_offset: Some(metadata.raw_range.start),
        });
    }
    let can_export_decoded =
        full_length <= LAZY_STREAM_DECODE_INPUT_LIMIT && decode_issues.is_empty();

    Ok(LazyStreamView {
        reference,
        declared_length: metadata.declared_length,
        actual_length: full_length,
        raw_range: metadata.raw_range,
        filters: metadata.filters,
        decoded_length,
        decode_steps,
        decode_issues,
        warnings,
        raw_text,
        raw_text_truncated,
        hex_text,
        hex_text_truncated,
        decoded_text,
        decoded_text_truncated,
        decoded_error,
        preview_limit: LAZY_STREAM_PREVIEW_LIMIT,
        can_export_raw,
        can_export_decoded,
    })
}

pub fn read_lazy_stream_raw_bytes(path: &Path, reference: ObjectRef) -> Result<Vec<u8>> {
    let document = open_lazy_pdf(path)?;
    read_lazy_stream_raw_bytes_with_document(&document, reference)
}

pub fn read_lazy_stream_raw_bytes_with_document(
    document: &LazyPdfDocument,
    reference: ObjectRef,
) -> Result<Vec<u8>> {
    let stream = read_lazy_stream_metadata(&document.path, &document.xref, reference)?;
    if stream.actual_length > LAZY_STREAM_EXPORT_LIMIT {
        return Err(PdfDebuggerError::LazyOpen {
            message: format!(
                "stream {reference} is {} bytes, above lazy raw export limit {} bytes",
                stream.actual_length, LAZY_STREAM_EXPORT_LIMIT
            ),
        });
    }
    read_lazy_stream_bytes(&document.path, &stream, stream.actual_length)
}

pub fn read_lazy_stream_metadata_with_document(
    document: &LazyPdfDocument,
    reference: ObjectRef,
) -> Result<PdfStream> {
    read_lazy_stream_metadata(&document.path, &document.xref, reference)
}

pub fn read_lazy_stream_preview_bytes_with_document(
    document: &LazyPdfDocument,
    reference: ObjectRef,
) -> Result<(PdfStream, Vec<u8>, bool)> {
    let stream = read_lazy_stream_metadata(&document.path, &document.xref, reference)?;
    let preview_length = stream.actual_length.min(LAZY_STREAM_PREVIEW_LIMIT);
    let bytes = read_lazy_stream_bytes(&document.path, &stream, preview_length)?;
    Ok((stream.clone(), bytes, stream.actual_length > preview_length))
}

pub fn read_lazy_stream_decoded_bytes(path: &Path, reference: ObjectRef) -> Result<Vec<u8>> {
    let document = open_lazy_pdf(path)?;
    read_lazy_stream_decoded_bytes_with_document(&document, reference)
}

pub fn read_lazy_stream_decoded_bytes_with_document(
    document: &LazyPdfDocument,
    reference: ObjectRef,
) -> Result<Vec<u8>> {
    let mut stream = read_lazy_stream_metadata(&document.path, &document.xref, reference)?;
    if stream.actual_length > LAZY_STREAM_DECODE_INPUT_LIMIT {
        return Err(PdfDebuggerError::LazyOpen {
            message: format!(
                "stream {reference} is {} bytes, above lazy decode input limit {} bytes",
                stream.actual_length, LAZY_STREAM_DECODE_INPUT_LIMIT
            ),
        });
    }
    stream.raw_bytes = read_lazy_stream_bytes(&document.path, &stream, stream.actual_length)?;
    let decode = decode_stream_with_limit(&stream, Some(LAZY_STREAM_DECODE_EXPORT_LIMIT));
    if decode.has_issues() {
        return Err(PdfDebuggerError::StreamDecode {
            reference,
            message: format_decode_issues(&decode.issues),
        });
    }
    Ok(decode.decoded)
}

pub fn read_lazy_stream_for_content(path: &Path, reference: ObjectRef) -> Result<PdfStream> {
    let document = open_lazy_pdf(path)?;
    read_lazy_stream_for_content_with_document(&document, reference)
}

pub fn read_lazy_stream_for_content_with_document(
    document: &LazyPdfDocument,
    reference: ObjectRef,
) -> Result<PdfStream> {
    let mut stream = read_lazy_stream_metadata(&document.path, &document.xref, reference)?;
    if stream.actual_length > LAZY_STREAM_DECODE_INPUT_LIMIT {
        return Err(PdfDebuggerError::LazyOpen {
            message: format!(
                "content stream {reference} is {} bytes, above lazy content analysis input limit {} bytes",
                stream.actual_length, LAZY_STREAM_DECODE_INPUT_LIMIT
            ),
        });
    }
    stream.raw_bytes = read_lazy_stream_bytes(&document.path, &stream, stream.actual_length)?;
    Ok(stream)
}

pub fn build_lazy_page_index(document: &mut LazyPdfDocument) -> PageIndex {
    build_lazy_page_index_with_options(document, true, None)
}

pub fn build_lazy_page_list(document: &mut LazyPdfDocument) -> PageIndex {
    build_lazy_page_index_with_options(document, false, None)
}

pub fn build_lazy_page_metadata(
    document: &mut LazyPdfDocument,
    page_number: u32,
) -> Result<PageSummary> {
    if page_number == 0 {
        return Err(PdfDebuggerError::LazyOpen {
            message: "Page numbers start at 1.".to_string(),
        });
    }

    let mut index = build_lazy_page_index_with_options(document, true, Some(page_number));
    index.pages.pop().ok_or_else(|| PdfDebuggerError::LazyOpen {
        message: format!("Page {page_number} was not found in the lazy page tree"),
    })
}

fn build_lazy_page_index_with_options(
    document: &mut LazyPdfDocument,
    include_page_details: bool,
    target_page_number: Option<u32>,
) -> PageIndex {
    let Some(root_reference) = document.metadata.root else {
        document.warnings.push(LazyPdfWarning {
            rule_id: "lazy.page_tree.root_missing".to_string(),
            message: "Lazy page index could not run because the trailer has no Root reference"
                .to_string(),
            byte_offset: None,
        });
        document.metadata.parse_warning_count = document.warnings.len();
        return PageIndex::default();
    };

    let mut file = match std::fs::File::open(&document.path) {
        Ok(file) => file,
        Err(error) => {
            document.warnings.push(LazyPdfWarning {
                rule_id: "lazy.page_tree.file_open_failed".to_string(),
                message: format!("Could not reopen PDF for lazy page tree traversal: {error}"),
                byte_offset: None,
            });
            document.metadata.parse_warning_count = document.warnings.len();
            return PageIndex::default();
        }
    };

    let Some(root_object) = read_lazy_object_from_file(
        &mut file,
        &document.xref,
        root_reference,
        &mut document.warnings,
    )
    .ok() else {
        document.warnings.push(LazyPdfWarning {
            rule_id: "lazy.page_tree.catalog_missing".to_string(),
            message: format!("Could not read Root catalog {root_reference}"),
            byte_offset: None,
        });
        document.metadata.parse_warning_count = document.warnings.len();
        return PageIndex::default();
    };

    let Some(pages_reference) = root_object
        .dictionary()
        .and_then(|dict| dict.get("Pages"))
        .and_then(PdfValue::as_reference)
    else {
        document.warnings.push(LazyPdfWarning {
            rule_id: "lazy.page_tree.pages_missing".to_string(),
            message: format!("Root catalog {root_reference} does not contain a /Pages reference"),
            byte_offset: Some(root_object.raw_range.start),
        });
        document.metadata.parse_warning_count = document.warnings.len();
        return PageIndex::default();
    };

    let mut context = LazyPageTreeContext {
        file,
        xref: &document.xref,
        warnings: &mut document.warnings,
        visited: BTreeSet::new(),
        node_count: 0,
        page_count_seen: 0,
        pages: Vec::new(),
        include_page_details,
        target_page_number,
        target_found: false,
    };
    context.walk_page_node(pages_reference, LazyInheritedPageAttributes::default(), 0);

    let pages = std::mem::take(&mut context.pages);
    drop(context);
    document.metadata.parse_warning_count = document.warnings.len();
    PageIndex { pages }
}

pub fn lazy_pdf_to_object_tree(document: &LazyPdfDocument) -> ObjectTreeNode {
    let mut root = ObjectTreeNode {
        label: "PDF".to_string(),
        kind: ObjectTreeNodeKind::Root,
        object: None,
        summary: Some(format!(
            "lazy open, {} xref entrie(s), {} in-use object(s)",
            document.metadata.xref_entry_count, document.metadata.object_count
        )),
        children: Vec::new(),
    };

    let mut trailer = ObjectTreeNode {
        label: "Trailer".to_string(),
        kind: ObjectTreeNodeKind::Trailer,
        object: None,
        summary: document.trailer.as_ref().map(PdfValue::summary),
        children: Vec::new(),
    };
    if let Some(dictionary) = document.trailer.as_ref().and_then(PdfValue::as_dictionary) {
        for (key, value) in dictionary {
            trailer.children.push(ObjectTreeNode {
                label: key.clone(),
                kind: ObjectTreeNodeKind::Reference,
                object: value.as_reference(),
                summary: Some(value.summary()),
                children: Vec::new(),
            });
        }
    }
    root.children.push(trailer);

    if let Some(root_reference) = document.metadata.root {
        root.children.push(ObjectTreeNode {
            label: "Catalog".to_string(),
            kind: ObjectTreeNodeKind::Catalog,
            object: Some(root_reference),
            summary: Some("lazy object, click to inspect by xref offset".to_string()),
            children: Vec::new(),
        });
    }

    let mut xref = ObjectTreeNode {
        label: "Cross-reference".to_string(),
        kind: ObjectTreeNodeKind::Xref,
        object: None,
        summary: Some(format!(
            "startxref {}, {} entrie(s)",
            document
                .xref
                .startxref
                .map(|offset| offset.to_string())
                .unwrap_or_else(|| "unknown".to_string()),
            document.xref.entries.len()
        )),
        children: Vec::new(),
    };
    for entry in document
        .xref
        .entries
        .iter()
        .filter(|entry| entry.in_use)
        .take(OBJECT_TREE_XREF_CHILD_LIMIT)
    {
        xref.children.push(ObjectTreeNode {
            label: entry.reference.to_string(),
            kind: ObjectTreeNodeKind::Object,
            object: Some(entry.reference),
            summary: Some(format!("xref offset {}, lazy object", entry.offset)),
            children: Vec::new(),
        });
    }
    if document.metadata.object_count > OBJECT_TREE_XREF_CHILD_LIMIT {
        xref.children.push(ObjectTreeNode {
            label: "More xref objects".to_string(),
            kind: ObjectTreeNodeKind::Xref,
            object: None,
            summary: Some(format!(
                "{} additional in-use object(s) hidden in this first lazy tree",
                document
                    .metadata
                    .object_count
                    .saturating_sub(OBJECT_TREE_XREF_CHILD_LIMIT)
            )),
            children: Vec::new(),
        });
    }
    root.children.push(xref);
    root
}

fn detect_page_count_lazy(
    file: &mut std::fs::File,
    xref: &LazyXrefIndex,
    root: ObjectRef,
    warnings: &mut Vec<LazyPdfWarning>,
) -> Option<u32> {
    let root_object = read_lazy_object_from_file(file, xref, root, warnings).ok()?;
    let pages = root_object
        .dictionary()?
        .get("Pages")
        .and_then(PdfValue::as_reference)?;
    let pages_object = read_lazy_object_from_file(file, xref, pages, warnings).ok()?;
    pages_object
        .dictionary()?
        .get("Count")
        .and_then(PdfValue::as_usize)
        .and_then(|count| u32::try_from(count).ok())
}

#[derive(Clone, Default)]
struct LazyInheritedPageAttributes {
    media_box: Option<PageBox>,
    crop_box: Option<PageBox>,
    bleed_box: Option<PageBox>,
    trim_box: Option<PageBox>,
    art_box: Option<PageBox>,
    rotation: Option<i32>,
    resources: Option<PdfValue>,
}

struct LazyPageTreeContext<'a> {
    file: std::fs::File,
    xref: &'a LazyXrefIndex,
    warnings: &'a mut Vec<LazyPdfWarning>,
    visited: BTreeSet<ObjectRef>,
    node_count: usize,
    page_count_seen: u32,
    pages: Vec<PageSummary>,
    include_page_details: bool,
    target_page_number: Option<u32>,
    target_found: bool,
}

impl LazyPageTreeContext<'_> {
    fn walk_page_node(
        &mut self,
        reference: ObjectRef,
        inherited: LazyInheritedPageAttributes,
        depth: usize,
    ) {
        if self.target_found {
            return;
        }
        if depth > PAGE_TREE_MAX_DEPTH {
            self.warn(
                "lazy.page_tree.depth_limit",
                format!(
                    "Page tree traversal reached depth limit {PAGE_TREE_MAX_DEPTH} at {reference}"
                ),
                None,
            );
            return;
        }
        if self.node_count >= PAGE_TREE_MAX_NODES {
            self.warn(
                "lazy.page_tree.node_limit",
                format!("Page tree traversal reached node limit {PAGE_TREE_MAX_NODES}"),
                None,
            );
            return;
        }
        if self.page_count_seen as usize >= PAGE_TREE_MAX_PAGES {
            self.warn(
                "lazy.page_tree.page_limit",
                format!("Page tree traversal reached page limit {PAGE_TREE_MAX_PAGES}"),
                None,
            );
            return;
        }
        if !self.visited.insert(reference) {
            self.warn(
                "lazy.page_tree.cycle",
                format!("Cycle detected while traversing page tree at {reference}"),
                None,
            );
            return;
        }
        self.node_count += 1;

        let object =
            match read_lazy_object_from_file(&mut self.file, self.xref, reference, self.warnings) {
                Ok(object) => object,
                Err(error) => {
                    self.warn(
                        "lazy.page_tree.object_unavailable",
                        format!("Could not read page tree object {reference}: {error}"),
                        None,
                    );
                    return;
                }
            };
        let Some(dictionary) = object.dictionary() else {
            self.warn(
                "lazy.page_tree.non_dictionary",
                format!("Page tree object {reference} is not a dictionary"),
                Some(object.raw_range.start),
            );
            return;
        };

        let inherited = self.inherit_attributes(dictionary, inherited);
        match dictionary.get("Type").and_then(PdfValue::as_name) {
            Some("Page") => {
                self.page_count_seen += 1;
                let page_number = self.page_count_seen;
                let should_collect = self
                    .target_page_number
                    .map(|target| target == page_number)
                    .unwrap_or(true);
                if should_collect {
                    let summary = if self.include_page_details {
                        self.page_summary(page_number, reference, dictionary, &inherited)
                    } else {
                        self.page_list_summary(page_number, reference, dictionary, &inherited)
                    };
                    self.pages.push(summary);
                    if self.target_page_number.is_some() {
                        self.target_found = true;
                    }
                }
            }
            Some("Pages") | None => {
                let Some(kids) = dictionary.get("Kids").and_then(PdfValue::as_array) else {
                    self.warn(
                        "lazy.page_tree.kids_missing",
                        format!("Pages node {reference} does not contain a /Kids array"),
                        Some(object.raw_range.start),
                    );
                    return;
                };
                for kid in kids {
                    if let Some(kid_reference) = kid.as_reference() {
                        self.walk_page_node(kid_reference, inherited.clone(), depth + 1);
                        if self.target_found {
                            break;
                        }
                    } else {
                        self.warn(
                            "lazy.page_tree.invalid_kid",
                            format!("Pages node {reference} contains a non-reference /Kids entry"),
                            Some(object.raw_range.start),
                        );
                    }
                }
            }
            Some(other) => self.warn(
                "lazy.page_tree.unexpected_type",
                format!("Page tree object {reference} has unexpected /Type /{other}"),
                Some(object.raw_range.start),
            ),
        }
    }

    fn page_list_summary(
        &mut self,
        page_number: u32,
        reference: ObjectRef,
        dictionary: &PdfDictionary,
        inherited: &LazyInheritedPageAttributes,
    ) -> PageSummary {
        let mut links = vec![PageObjectLink {
            label: "Page".to_string(),
            kind: PageObjectLinkKind::Page,
            reference,
        }];

        if let Some(parent) = dictionary.get("Parent").and_then(PdfValue::as_reference) {
            links.push(PageObjectLink {
                label: "Parent".to_string(),
                kind: PageObjectLinkKind::Parent,
                reference: parent,
            });
        }

        let media_box = dictionary
            .get("MediaBox")
            .and_then(|value| self.page_box_from_value(value))
            .or(inherited.media_box);
        if media_box.is_none() {
            self.warn(
                "lazy.page_tree.media_box_missing",
                format!(
                    "Page {reference} does not have a MediaBox in local or inherited attributes"
                ),
                None,
            );
        }

        PageSummary {
            page_number,
            reference,
            media_box,
            crop_box: dictionary
                .get("CropBox")
                .and_then(|value| self.page_box_from_value(value))
                .or(inherited.crop_box),
            bleed_box: dictionary
                .get("BleedBox")
                .and_then(|value| self.page_box_from_value(value))
                .or(inherited.bleed_box),
            trim_box: dictionary
                .get("TrimBox")
                .and_then(|value| self.page_box_from_value(value))
                .or(inherited.trim_box),
            art_box: dictionary
                .get("ArtBox")
                .and_then(|value| self.page_box_from_value(value))
                .or(inherited.art_box),
            rotation: dictionary
                .get("Rotate")
                .and_then(|value| self.rotation_from_value(value))
                .or(inherited.rotation),
            resources: PageResourceSummary::default(),
            links,
        }
    }

    fn inherit_attributes(
        &mut self,
        dictionary: &PdfDictionary,
        mut inherited: LazyInheritedPageAttributes,
    ) -> LazyInheritedPageAttributes {
        if let Some(value) = dictionary.get("MediaBox") {
            inherited.media_box = self.page_box_from_value(value);
        }
        if let Some(value) = dictionary.get("CropBox") {
            inherited.crop_box = self.page_box_from_value(value);
        }
        if let Some(value) = dictionary.get("BleedBox") {
            inherited.bleed_box = self.page_box_from_value(value);
        }
        if let Some(value) = dictionary.get("TrimBox") {
            inherited.trim_box = self.page_box_from_value(value);
        }
        if let Some(value) = dictionary.get("ArtBox") {
            inherited.art_box = self.page_box_from_value(value);
        }
        if let Some(value) = dictionary.get("Rotate") {
            inherited.rotation = self.rotation_from_value(value);
        }
        if let Some(value) = dictionary.get("Resources") {
            inherited.resources = Some(value.clone());
        }
        inherited
    }

    fn page_summary(
        &mut self,
        page_number: u32,
        reference: ObjectRef,
        dictionary: &PdfDictionary,
        inherited: &LazyInheritedPageAttributes,
    ) -> PageSummary {
        let mut links = vec![PageObjectLink {
            label: "Page".to_string(),
            kind: PageObjectLinkKind::Page,
            reference,
        }];

        if let Some(parent) = dictionary.get("Parent").and_then(PdfValue::as_reference) {
            links.push(PageObjectLink {
                label: "Parent".to_string(),
                kind: PageObjectLinkKind::Parent,
                reference: parent,
            });
        }

        let mut resources = PageResourceSummary::default();
        if let Some(contents) = dictionary.get("Contents") {
            collect_lazy_links(
                contents,
                "Contents",
                PageObjectLinkKind::Contents,
                &mut links,
            );
            resources.contents = count_references(contents);
        }
        if let Some(annotations) = dictionary.get("Annots") {
            collect_lazy_links(
                annotations,
                "Annotation",
                PageObjectLinkKind::Annotation,
                &mut links,
            );
            resources.annotations = count_references(annotations);
        }

        let resources_value = dictionary
            .get("Resources")
            .cloned()
            .or_else(|| inherited.resources.clone());
        if let Some(value) = resources_value.as_ref() {
            if let Some(reference) = value.as_reference() {
                links.push(PageObjectLink {
                    label: "Resources".to_string(),
                    kind: PageObjectLinkKind::Resources,
                    reference,
                });
            }
            self.add_resource_links(value, &mut resources, &mut links);
        }

        dedupe_page_links(&mut links);

        let media_box = dictionary
            .get("MediaBox")
            .and_then(|value| self.page_box_from_value(value))
            .or(inherited.media_box);
        if media_box.is_none() {
            self.warn(
                "lazy.page_tree.media_box_missing",
                format!(
                    "Page {reference} does not have a MediaBox in local or inherited attributes"
                ),
                None,
            );
        }

        PageSummary {
            page_number,
            reference,
            media_box,
            crop_box: dictionary
                .get("CropBox")
                .and_then(|value| self.page_box_from_value(value))
                .or(inherited.crop_box),
            bleed_box: dictionary
                .get("BleedBox")
                .and_then(|value| self.page_box_from_value(value))
                .or(inherited.bleed_box),
            trim_box: dictionary
                .get("TrimBox")
                .and_then(|value| self.page_box_from_value(value))
                .or(inherited.trim_box),
            art_box: dictionary
                .get("ArtBox")
                .and_then(|value| self.page_box_from_value(value))
                .or(inherited.art_box),
            rotation: dictionary
                .get("Rotate")
                .and_then(|value| self.rotation_from_value(value))
                .or(inherited.rotation),
            resources,
            links,
        }
    }

    fn add_resource_links(
        &mut self,
        value: &PdfValue,
        resources: &mut PageResourceSummary,
        links: &mut Vec<PageObjectLink>,
    ) {
        let Some(dictionary) = self.resolve_dictionary(value) else {
            return;
        };

        if let Some(fonts) = dictionary.get("Font") {
            if let Some(font_dictionary) = self.resolve_dictionary(fonts) {
                resources.fonts = font_dictionary.len();
                for (name, value) in font_dictionary {
                    if let Some(reference) = value.as_reference() {
                        links.push(PageObjectLink {
                            label: format!("/{name} Font"),
                            kind: PageObjectLinkKind::Font,
                            reference,
                        });
                    }
                }
            }
        }

        if let Some(xobjects) = dictionary.get("XObject") {
            if let Some(xobject_dictionary) = self.resolve_dictionary(xobjects) {
                resources.xobjects = xobject_dictionary.len();
                for (name, value) in xobject_dictionary {
                    let Some(reference) = value.as_reference() else {
                        continue;
                    };
                    let is_image = self
                        .read_dictionary(reference)
                        .and_then(|dict| {
                            dict.get("Subtype")
                                .and_then(PdfValue::as_name)
                                .map(|subtype| subtype == "Image")
                        })
                        .unwrap_or(false);
                    if is_image {
                        resources.images += 1;
                    }
                    links.push(PageObjectLink {
                        label: if is_image {
                            format!("/{name} Image")
                        } else {
                            format!("/{name} XObject")
                        },
                        kind: if is_image {
                            PageObjectLinkKind::Image
                        } else {
                            PageObjectLinkKind::XObject
                        },
                        reference,
                    });
                }
            }
        }
    }

    fn resolve_dictionary(&mut self, value: &PdfValue) -> Option<PdfDictionary> {
        match value {
            PdfValue::Dictionary(dictionary) => Some(dictionary.clone()),
            PdfValue::Reference(reference) => self.read_dictionary(*reference),
            _ => None,
        }
    }

    fn read_dictionary(&mut self, reference: ObjectRef) -> Option<PdfDictionary> {
        read_lazy_object_from_file(&mut self.file, self.xref, reference, self.warnings)
            .ok()
            .and_then(|object| object.dictionary().cloned())
    }

    fn page_box_from_value(&mut self, value: &PdfValue) -> Option<PageBox> {
        match value {
            PdfValue::Array(values) => page_box_from_array(values),
            PdfValue::Reference(reference) => {
                let object = read_lazy_object_from_file(
                    &mut self.file,
                    self.xref,
                    *reference,
                    self.warnings,
                )
                .ok()?;
                self.page_box_from_value(&object.value)
            }
            _ => None,
        }
    }

    fn rotation_from_value(&mut self, value: &PdfValue) -> Option<i32> {
        match value {
            PdfValue::Number(value) if value.fract() == 0.0 => Some(*value as i32),
            PdfValue::Reference(reference) => {
                let object = read_lazy_object_from_file(
                    &mut self.file,
                    self.xref,
                    *reference,
                    self.warnings,
                )
                .ok()?;
                self.rotation_from_value(&object.value)
            }
            _ => None,
        }
    }

    fn warn(&mut self, rule_id: &str, message: String, byte_offset: Option<usize>) {
        self.warnings.push(LazyPdfWarning {
            rule_id: rule_id.to_string(),
            message,
            byte_offset,
        });
    }
}

fn read_lazy_object(path: &Path, xref: &LazyXrefIndex, reference: ObjectRef) -> Result<PdfObject> {
    let mut warnings = Vec::new();
    let mut file = std::fs::File::open(path)?;
    read_lazy_object_from_file(&mut file, xref, reference, &mut warnings)
}

pub(crate) fn read_lazy_stream_metadata_for_diagnostics(
    path: &Path,
    xref: &LazyXrefIndex,
    reference: ObjectRef,
) -> Result<PdfStream> {
    read_lazy_stream_metadata(path, xref, reference)
}

fn read_lazy_stream_metadata(
    path: &Path,
    xref: &LazyXrefIndex,
    reference: ObjectRef,
) -> Result<PdfStream> {
    let mut warnings = Vec::new();
    let mut file = std::fs::File::open(path)?;
    let object = read_lazy_object_from_file(&mut file, xref, reference, &mut warnings)?;
    let mut stream = object
        .stream
        .ok_or(PdfDebuggerError::StreamNotFound { reference })?;
    if stream.declared_length.is_none() {
        if let Some(length_reference) = stream
            .dictionary
            .get("Length")
            .and_then(PdfValue::as_reference)
        {
            let length_object =
                read_lazy_object_from_file(&mut file, xref, length_reference, &mut warnings)?;
            stream.declared_length = length_object.value.as_usize();
            if stream.declared_length.is_none() {
                return Err(PdfDebuggerError::LazyOpen {
                    message: format!(
                        "stream {reference} has an indirect /Length {length_reference} that is not a non-negative integer"
                    ),
                });
            }
        }
    }
    let Some(length) = stream.declared_length else {
        return Err(PdfDebuggerError::LazyOpen {
            message: format!(
                "stream {reference} does not have a numeric /Length; marker-based lazy stream reads are deferred"
            ),
        });
    };
    stream.actual_length = length;
    stream.raw_range.end =
        stream
            .raw_range
            .start
            .checked_add(length)
            .ok_or_else(|| PdfDebuggerError::LazyOpen {
                message: format!("stream {reference} /Length overflows byte range"),
            })?;

    let file_size = file.metadata()?.len() as usize;
    if stream.raw_range.end > file_size {
        return Err(PdfDebuggerError::LazyOpen {
            message: format!(
                "stream {reference} byte range {}..{} exceeds file size {}",
                stream.raw_range.start, stream.raw_range.end, file_size
            ),
        });
    }
    Ok(stream)
}

fn read_lazy_stream_bytes(path: &Path, stream: &PdfStream, length: usize) -> Result<Vec<u8>> {
    if length > stream.actual_length {
        return Err(PdfDebuggerError::LazyOpen {
            message: format!(
                "requested {} stream bytes but selected stream has only {} bytes",
                length, stream.actual_length
            ),
        });
    }
    let mut file = std::fs::File::open(path)?;
    read_at(&mut file, stream.raw_range.start as u64, length)
}

fn format_decode_issues(issues: &[DecodeIssue]) -> String {
    issues
        .iter()
        .map(|issue| format!("/{}: {}", issue.filter, issue.message))
        .collect::<Vec<_>>()
        .join("; ")
}

fn read_lazy_object_from_file(
    file: &mut std::fs::File,
    xref: &LazyXrefIndex,
    reference: ObjectRef,
    warnings: &mut Vec<LazyPdfWarning>,
) -> Result<PdfObject> {
    let entry = xref
        .entries
        .iter()
        .find(|entry| entry.in_use && entry.reference == reference)
        .ok_or(PdfDebuggerError::ObjectNotFound { reference })?;
    if let Some(compressed) = &entry.compressed {
        return read_lazy_compressed_object_from_file(file, xref, reference, compressed, warnings);
    }
    let window = read_lazy_object_window(file, entry.offset as u64)?;
    parse_indirect_object_window(&window, entry.offset, reference).ok_or_else(|| {
        warnings.push(LazyPdfWarning {
            rule_id: "lazy.object_parse_failed".to_string(),
            message: format!(
                "Could not parse object {reference} at xref offset {}",
                entry.offset
            ),
            byte_offset: Some(entry.offset),
        });
        PdfDebuggerError::LazyOpen {
            message: format!(
                "Could not parse object {reference} at xref offset {}",
                entry.offset
            ),
        }
    })
}

fn read_lazy_object_window(file: &mut std::fs::File, offset: u64) -> Result<Vec<u8>> {
    let mut limit = OBJECT_INITIAL_READ_LIMIT;
    loop {
        let window = read_at(file, offset, limit)?;
        let reached_eof = window.len() < limit;
        if find_subslice(&window, b"endobj").is_some() || reached_eof || limit >= OBJECT_READ_LIMIT
        {
            return Ok(window);
        }
        limit = (limit * 2).min(OBJECT_READ_LIMIT);
    }
}

fn read_lazy_compressed_object_from_file(
    file: &mut std::fs::File,
    xref: &LazyXrefIndex,
    reference: ObjectRef,
    compressed: &LazyCompressedObjectEntry,
    warnings: &mut Vec<LazyPdfWarning>,
) -> Result<PdfObject> {
    let object_stream = read_lazy_object_from_file(file, xref, compressed.object_stream, warnings)?;
    let mut stream = object_stream
        .stream
        .ok_or_else(|| PdfDebuggerError::LazyOpen {
            message: format!(
                "object {reference} is compressed in {}, but that object is not a stream",
                compressed.object_stream
            ),
        })?;
    if !name_is_dictionary(&stream.dictionary, "Type", "ObjStm") {
        warnings.push(LazyPdfWarning {
            rule_id: "lazy.object_stream.unexpected_type".to_string(),
            message: format!(
                "object {reference} is stored in {}, which is not marked /Type /ObjStm",
                compressed.object_stream
            ),
            byte_offset: Some(object_stream.raw_range.start),
        });
    }
    let first = stream
        .dictionary
        .get("First")
        .and_then(PdfValue::as_usize)
        .ok_or_else(|| PdfDebuggerError::LazyOpen {
            message: format!(
                "object stream {} is missing numeric /First",
                compressed.object_stream
            ),
        })?;
    let count = stream
        .dictionary
        .get("N")
        .and_then(PdfValue::as_usize)
        .ok_or_else(|| PdfDebuggerError::LazyOpen {
            message: format!(
                "object stream {} is missing numeric /N",
                compressed.object_stream
            ),
        })?;
    if count == 0 || compressed.index >= count {
        return Err(PdfDebuggerError::LazyOpen {
            message: format!(
                "object {reference} index {} is outside object stream {} member count {count}",
                compressed.index, compressed.object_stream
            ),
        });
    }
    if stream.actual_length > LAZY_STREAM_DECODE_INPUT_LIMIT {
        return Err(PdfDebuggerError::LazyOpen {
            message: format!(
                "object stream {} is {} bytes, above lazy object stream decode input limit {} bytes",
                compressed.object_stream, stream.actual_length, LAZY_STREAM_DECODE_INPUT_LIMIT
            ),
        });
    }
    stream.raw_bytes = read_at(file, stream.raw_range.start as u64, stream.actual_length)?;
    let decode = decode_stream_with_limit(&stream, Some(LAZY_STREAM_DECODE_OUTPUT_LIMIT));
    if decode.has_issues() {
        return Err(PdfDebuggerError::LazyOpen {
            message: format!(
                "Could not decode object stream {}: {}",
                compressed.object_stream,
                format_decode_issues(&decode.issues)
            ),
        });
    }
    let members = parse_object_stream_member_table(&decode.decoded, first, count)?;
    let (_, relative_offset) = members[compressed.index];
    let body_start =
        first
            .checked_add(relative_offset)
            .ok_or_else(|| PdfDebuggerError::LazyOpen {
                message: format!(
                    "object stream {} member offset overflows",
                    compressed.object_stream
                ),
            })?;
    if body_start >= decode.decoded.len() {
        return Err(PdfDebuggerError::LazyOpen {
            message: format!(
                "object {reference} body starts outside decoded object stream {}",
                compressed.object_stream
            ),
        });
    }
    let body_end = members
        .get(compressed.index + 1)
        .map(|(_, next_offset)| first.saturating_add(*next_offset))
        .unwrap_or(decode.decoded.len())
        .min(decode.decoded.len());
    let body =
        decode
            .decoded
            .get(body_start..body_end)
            .ok_or_else(|| PdfDebuggerError::LazyOpen {
                message: format!("object {reference} has an invalid object stream body range"),
            })?;
    let (value, consumed) =
        parse_pdf_value_with_consumed(body).ok_or_else(|| PdfDebuggerError::LazyOpen {
            message: format!(
                "Could not parse compressed object {reference} from object stream {}",
                compressed.object_stream
            ),
        })?;
    let raw_bytes = body[..consumed.min(body.len())].to_vec();
    Ok(PdfObject {
        reference,
        value,
        stream: None,
        raw_range: ByteRange {
            start: object_stream.raw_range.start,
            end: object_stream.raw_range.end,
        },
        raw_bytes,
    })
}

fn parse_object_stream_member_table(
    decoded: &[u8],
    first: usize,
    count: usize,
) -> Result<Vec<(u32, usize)>> {
    let header = decoded
        .get(..first)
        .ok_or_else(|| PdfDebuggerError::LazyOpen {
            message: "object stream /First points outside decoded bytes".to_string(),
        })?;
    let header_text = String::from_utf8_lossy(header);
    let numbers = header_text
        .split_whitespace()
        .filter_map(|part| part.parse::<usize>().ok())
        .collect::<Vec<_>>();
    if numbers.len() < count.saturating_mul(2) {
        return Err(PdfDebuggerError::LazyOpen {
            message: format!(
                "object stream member table has {} number(s), expected {}",
                numbers.len(),
                count.saturating_mul(2)
            ),
        });
    }
    let mut members = Vec::new();
    for index in 0..count {
        members.push((
            numbers[index * 2].min(u32::MAX as usize) as u32,
            numbers[index * 2 + 1],
        ));
    }
    Ok(members)
}

fn parse_indirect_object_window(
    window: &[u8],
    absolute_start: usize,
    reference: ObjectRef,
) -> Option<PdfObject> {
    let header_end = object_header_end(window, reference)?;
    let body = &window[header_end..];
    let (value, value_end) = parse_pdf_value_with_consumed(body)?;
    let stream_keyword = skip_whitespace_and_comments(body, value_end);

    if body
        .get(stream_keyword..)
        .is_some_and(|tail| tail.starts_with(b"stream"))
    {
        let dictionary = value.as_dictionary().cloned().unwrap_or_default();
        let mut data_start = stream_keyword + b"stream".len();
        if body.get(data_start..data_start + 2) == Some(b"\r\n") {
            data_start += 2;
        } else if body
            .get(data_start)
            .is_some_and(|byte| *byte == b'\n' || *byte == b'\r')
        {
            data_start += 1;
        }
        let declared_length = dictionary.get("Length").and_then(PdfValue::as_usize);
        let marker_end =
            find_subslice(&body[data_start..], b"endstream").map(|relative| data_start + relative);
        let data_end = declared_length
            .map(|length| data_start.saturating_add(length))
            .or_else(|| {
                marker_end.map(|marker_end| trim_single_stream_eol(body, data_start, marker_end))
            })
            .unwrap_or(data_start);
        let data_end_in_window = data_end.min(body.len());
        let object_end = marker_end
            .and_then(|marker_end| {
                find_subslice(&body[marker_end..], b"endobj")
                    .map(|relative| header_end + marker_end + relative + b"endobj".len())
            })
            .unwrap_or(header_end + data_end_in_window);
        let preview_end = (header_end + data_start).min(window.len());
        let raw_range = ByteRange {
            start: absolute_start + header_end + data_start,
            end: absolute_start + header_end + data_end,
        };
        let actual_length = declared_length.unwrap_or_else(|| raw_range.len());
        let filters = filter_names_from_dictionary(&dictionary);
        Some(PdfObject {
            reference,
            value: PdfValue::Dictionary(dictionary.clone()),
            stream: Some(PdfStream {
                dictionary,
                declared_length,
                actual_length,
                filters,
                raw_range,
                raw_bytes: Vec::new(),
            }),
            raw_range: ByteRange {
                start: absolute_start,
                end: absolute_start + object_end,
            },
            raw_bytes: window[..preview_end].to_vec(),
        })
    } else {
        let relative_end = find_subslice(body, b"endobj")?;
        let raw_end = header_end + relative_end + b"endobj".len();
        Some(PdfObject {
            reference,
            value,
            stream: None,
            raw_range: ByteRange {
                start: absolute_start,
                end: absolute_start + raw_end,
            },
            raw_bytes: window[..raw_end].to_vec(),
        })
    }
}

fn object_header_end(window: &[u8], reference: ObjectRef) -> Option<usize> {
    let mut cursor = skip_whitespace_and_comments(window, 0);
    let (object, next) = parse_unsigned_integer(window, cursor)?;
    cursor = skip_whitespace_and_comments(window, next);
    let (generation, next) = parse_unsigned_integer(window, cursor)?;
    cursor = skip_whitespace_and_comments(window, next);
    if object != usize::try_from(reference.object).ok()?
        || generation != usize::from(reference.generation)
    {
        return None;
    }
    if !window.get(cursor..)?.starts_with(b"obj") {
        return None;
    }
    Some(cursor + b"obj".len())
}

fn parse_xref_at(
    file: &mut std::fs::File,
    file_size: u64,
    startxref: usize,
    warnings: &mut Vec<LazyPdfWarning>,
) -> Result<(Vec<LazyXrefEntry>, Option<PdfValue>)> {
    let mut sections = Vec::new();
    let mut visited = BTreeSet::new();
    let mut cursor = Some(startxref);

    while let Some(section_offset) = cursor {
        if sections.len() >= XREF_CHAIN_MAX_SECTIONS {
            warnings.push(LazyPdfWarning {
                rule_id: "lazy.xref_chain.limit".to_string(),
                message: format!(
                    "Stopped lazy xref /Prev traversal after {XREF_CHAIN_MAX_SECTIONS} section(s)"
                ),
                byte_offset: Some(section_offset),
            });
            break;
        }
        if !visited.insert(section_offset) {
            warnings.push(LazyPdfWarning {
                rule_id: "lazy.xref_chain.cycle".to_string(),
                message: format!("Detected a repeated xref /Prev offset at byte {section_offset}"),
                byte_offset: Some(section_offset),
            });
            break;
        }

        let mut section = parse_single_xref_section(file, file_size, section_offset, warnings)?;
        let previous = trailer_usize(&section.trailer, "Prev");
        if let Some(hybrid_offset) = trailer_usize(&section.trailer, "XRefStm") {
            match parse_xref_stream_at(file, file_size, hybrid_offset, warnings) {
                Ok(hybrid) => {
                    section.entries.extend(hybrid.entries);
                }
                Err(error) => warnings.push(LazyPdfWarning {
                    rule_id: "lazy.hybrid_xref_stream_failed".to_string(),
                    message: format!(
                        "Could not parse hybrid /XRefStm at byte {hybrid_offset}: {error}"
                    ),
                    byte_offset: Some(hybrid_offset),
                }),
            }
        }
        sections.push(section);
        cursor = previous;
    }

    let trailer = merge_xref_trailers(&sections);
    let mut merged = BTreeMap::new();
    for section in sections.iter().rev() {
        for entry in &section.entries {
            merged.insert(entry.reference, entry.clone());
        }
    }

    Ok((merged.into_values().collect(), trailer))
}

fn parse_single_xref_section(
    file: &mut std::fs::File,
    file_size: u64,
    startxref: usize,
    warnings: &mut Vec<LazyPdfWarning>,
) -> Result<LazyXrefSection> {
    if startxref as u64 >= file_size {
        return Err(PdfDebuggerError::LazyOpen {
            message: format!("startxref offset {startxref} is outside the file"),
        });
    }
    let read_len = XREF_READ_LIMIT.min(file_size.saturating_sub(startxref as u64) as usize);
    let bytes = read_at(file, startxref as u64, read_len)?;
    if bytes.starts_with(b"xref") {
        return parse_classic_xref_from_bytes(&bytes, startxref, warnings);
    }
    parse_xref_stream_at(file, file_size, startxref, warnings)
}

fn merge_xref_trailers(sections: &[LazyXrefSection]) -> Option<PdfValue> {
    let mut merged = PdfDictionary::new();
    let mut saw_dictionary = false;

    for section in sections.iter().rev() {
        if let Some(dictionary) = section.trailer.as_ref().and_then(PdfValue::as_dictionary) {
            saw_dictionary = true;
            for (key, value) in dictionary {
                merged.insert(key.clone(), value.clone());
            }
        }
    }

    saw_dictionary.then_some(PdfValue::Dictionary(merged))
}

fn trailer_usize(trailer: &Option<PdfValue>, key: &str) -> Option<usize> {
    trailer
        .as_ref()
        .and_then(PdfValue::as_dictionary)
        .and_then(|dict| dict.get(key))
        .and_then(PdfValue::as_usize)
}

fn parse_classic_xref_from_bytes(
    bytes: &[u8],
    startxref: usize,
    warnings: &mut Vec<LazyPdfWarning>,
) -> Result<LazyXrefSection> {
    let mut entries = Vec::new();
    let mut cursor = line_end(bytes, 0).saturating_add(1);
    loop {
        cursor = skip_blank_lines(bytes, cursor);
        if cursor >= bytes.len() {
            break;
        }
        if bytes[cursor..].starts_with(b"trailer") {
            let trailer_start = cursor + b"trailer".len();
            let trailer = parse_pdf_value_with_consumed(&bytes[trailer_start..])
                .map(|(value, _)| value)
                .or_else(|| {
                    warnings.push(LazyPdfWarning {
                        rule_id: "lazy.trailer_parse_failed".to_string(),
                        message: "Could not parse lazy trailer dictionary".to_string(),
                        byte_offset: Some(startxref + trailer_start),
                    });
                    None
                });
            return Ok(LazyXrefSection { entries, trailer });
        }

        let current_end = line_end(bytes, cursor);
        let header_line = String::from_utf8_lossy(&bytes[cursor..current_end]);
        let header_parts = header_line.split_whitespace().collect::<Vec<_>>();
        if header_parts.len() != 2 {
            break;
        }
        let Some(first_object) = header_parts[0].parse::<u32>().ok() else {
            break;
        };
        let Some(count) = header_parts[1].parse::<usize>().ok() else {
            break;
        };
        cursor = current_end.saturating_add(1);

        for index in 0..count {
            cursor = skip_blank_lines(bytes, cursor);
            let entry_end = line_end(bytes, cursor);
            if entry_end <= cursor || entry_end > bytes.len() {
                break;
            }
            let entry_line = String::from_utf8_lossy(&bytes[cursor..entry_end]);
            let parts = entry_line.split_whitespace().collect::<Vec<_>>();
            if parts.len() >= 3 {
                if let (Ok(offset), Ok(generation)) =
                    (parts[0].parse::<usize>(), parts[1].parse::<u16>())
                {
                    entries.push(LazyXrefEntry {
                        reference: ObjectRef::new(first_object + index as u32, generation),
                        offset,
                        in_use: parts[2] == "n",
                        generation,
                        compressed: None,
                    });
                }
            }
            cursor = entry_end.saturating_add(1);
        }
    }

    warnings.push(LazyPdfWarning {
        rule_id: "lazy.trailer_missing".to_string(),
        message: "Classic xref table was parsed without a trailer dictionary".to_string(),
        byte_offset: Some(startxref),
    });
    Ok(LazyXrefSection {
        entries,
        trailer: None,
    })
}

fn parse_xref_stream_at(
    file: &mut std::fs::File,
    file_size: u64,
    startxref: usize,
    warnings: &mut Vec<LazyPdfWarning>,
) -> Result<LazyXrefSection> {
    if startxref as u64 >= file_size {
        return Err(PdfDebuggerError::LazyOpen {
            message: format!("startxref offset {startxref} is outside the file"),
        });
    }

    let window = read_at(file, startxref as u64, OBJECT_READ_LIMIT)?;
    let Some(reference) = object_reference_from_header(&window) else {
        return Err(PdfDebuggerError::LazyOpen {
            message: format!("startxref offset {startxref} does not point to an indirect object or classic xref table"),
        });
    };
    let Some(mut object) = parse_indirect_object_window(&window, startxref, reference) else {
        return Err(PdfDebuggerError::LazyOpen {
            message: format!("Could not parse xref stream object at byte {startxref}"),
        });
    };
    let Some(mut stream) = object.stream.take() else {
        return Err(PdfDebuggerError::LazyOpen {
            message: format!(
                "startxref object {reference} is not a classic xref table or xref stream"
            ),
        });
    };
    let dictionary = stream.dictionary.clone();
    if !name_is_dictionary(&dictionary, "Type", "XRef") {
        warnings.push(LazyPdfWarning {
            rule_id: "lazy.xref_stream_or_invalid".to_string(),
            message: "startxref points to a stream object that is not /Type /XRef".to_string(),
            byte_offset: Some(startxref),
        });
    }
    let Some(length) = stream.declared_length else {
        return Err(PdfDebuggerError::LazyOpen {
            message: format!("xref stream {reference} does not have a numeric /Length"),
        });
    };
    if length > XREF_STREAM_DECODE_LIMIT {
        return Err(PdfDebuggerError::LazyOpen {
            message: format!(
                "xref stream {reference} length {length} exceeds lazy xref stream limit {XREF_STREAM_DECODE_LIMIT}"
            ),
        });
    }
    stream.raw_bytes = read_at(file, stream.raw_range.start as u64, length)?;
    stream.actual_length = length;
    let decode = decode_stream_with_limit(&stream, Some(XREF_STREAM_DECODE_LIMIT));
    if decode.has_issues() {
        let message = decode
            .issues
            .iter()
            .map(|issue| format!("/{}: {}", issue.filter, issue.message))
            .collect::<Vec<_>>()
            .join("; ");
        return Err(PdfDebuggerError::LazyOpen {
            message: format!("Could not decode xref stream {reference}: {message}"),
        });
    }

    let entries = parse_decoded_xref_stream_entries(&dictionary, &decode.decoded, reference)?;
    Ok(LazyXrefSection {
        entries,
        trailer: Some(PdfValue::Dictionary(dictionary)),
    })
}

fn parse_decoded_xref_stream_entries(
    dictionary: &PdfDictionary,
    decoded: &[u8],
    xref_stream_reference: ObjectRef,
) -> Result<Vec<LazyXrefEntry>> {
    let widths = xref_widths(dictionary)?;
    let indexes = xref_indexes(dictionary)?;
    let entry_size = widths.iter().sum::<usize>();
    if entry_size == 0 {
        return Err(PdfDebuggerError::LazyOpen {
            message: format!("xref stream {xref_stream_reference} has zero-width /W entries"),
        });
    }

    let mut entries = Vec::new();
    let mut cursor = 0usize;
    for (first_object, count) in indexes {
        for index in 0..count {
            if cursor + entry_size > decoded.len() {
                return Ok(entries);
            }
            let fields = [
                read_xref_field(decoded, cursor, widths[0]),
                read_xref_field(decoded, cursor + widths[0], widths[1]),
                read_xref_field(decoded, cursor + widths[0] + widths[1], widths[2]),
            ];
            let entry_type = if widths[0] == 0 { 1 } else { fields[0] };
            let object_number = first_object + index as u32;
            match entry_type {
                0 => entries.push(LazyXrefEntry {
                    reference: ObjectRef::new(
                        object_number,
                        fields[2].min(u16::MAX as usize) as u16,
                    ),
                    offset: fields[1],
                    in_use: false,
                    generation: fields[2].min(u16::MAX as usize) as u16,
                    compressed: None,
                }),
                1 => entries.push(LazyXrefEntry {
                    reference: ObjectRef::new(
                        object_number,
                        fields[2].min(u16::MAX as usize) as u16,
                    ),
                    offset: fields[1],
                    in_use: true,
                    generation: fields[2].min(u16::MAX as usize) as u16,
                    compressed: None,
                }),
                2 => entries.push(LazyXrefEntry {
                    reference: ObjectRef::new(object_number, 0),
                    offset: fields[1],
                    in_use: true,
                    generation: 0,
                    compressed: Some(LazyCompressedObjectEntry {
                        object_stream: ObjectRef::new(fields[1].min(u32::MAX as usize) as u32, 0),
                        index: fields[2],
                    }),
                }),
                _ => {}
            }
            cursor += entry_size;
        }
    }
    Ok(entries)
}

fn xref_widths(dictionary: &PdfDictionary) -> Result<[usize; 3]> {
    let Some(values) = dictionary.get("W").and_then(PdfValue::as_array) else {
        return Err(PdfDebuggerError::LazyOpen {
            message: "xref stream is missing /W".to_string(),
        });
    };
    if values.len() != 3 {
        return Err(PdfDebuggerError::LazyOpen {
            message: "xref stream /W must contain exactly three widths".to_string(),
        });
    }
    Ok([
        values[0].as_usize().unwrap_or(0),
        values[1].as_usize().unwrap_or(0),
        values[2].as_usize().unwrap_or(0),
    ])
}

fn xref_indexes(dictionary: &PdfDictionary) -> Result<Vec<(u32, usize)>> {
    if let Some(values) = dictionary.get("Index").and_then(PdfValue::as_array) {
        let mut indexes = Vec::new();
        for chunk in values.chunks(2) {
            if chunk.len() != 2 {
                break;
            }
            let first = chunk[0].as_usize().unwrap_or(0).min(u32::MAX as usize) as u32;
            let count = chunk[1].as_usize().unwrap_or(0);
            indexes.push((first, count));
        }
        return Ok(indexes);
    }

    let size = dictionary
        .get("Size")
        .and_then(PdfValue::as_usize)
        .ok_or_else(|| PdfDebuggerError::LazyOpen {
            message: "xref stream is missing /Index and numeric /Size".to_string(),
        })?;
    Ok(vec![(0, size)])
}

fn read_xref_field(bytes: &[u8], start: usize, width: usize) -> usize {
    let mut value = 0usize;
    for byte in bytes
        .get(start..start.saturating_add(width))
        .unwrap_or_default()
    {
        value = (value << 8) | usize::from(*byte);
    }
    value
}

fn object_reference_from_header(window: &[u8]) -> Option<ObjectRef> {
    let mut cursor = skip_whitespace_and_comments(window, 0);
    let (object, next) = parse_unsigned_integer(window, cursor)?;
    cursor = skip_whitespace_and_comments(window, next);
    let (generation, next) = parse_unsigned_integer(window, cursor)?;
    cursor = skip_whitespace_and_comments(window, next);
    if !window.get(cursor..)?.starts_with(b"obj") {
        return None;
    }
    Some(ObjectRef::new(
        object.min(u32::MAX as usize) as u32,
        generation.min(u16::MAX as usize) as u16,
    ))
}

fn name_is_dictionary(dictionary: &PdfDictionary, key: &str, expected: &str) -> bool {
    dictionary
        .get(key)
        .and_then(PdfValue::as_name)
        .is_some_and(|name| name == expected)
}

fn parse_pdf_version(bytes: &[u8], warnings: &mut Vec<LazyPdfWarning>) -> Option<String> {
    if let Some(offset) = find_subslice(bytes, b"%PDF-") {
        let start = offset + 5;
        let end = bytes[start..]
            .iter()
            .position(|byte| *byte == b'\r' || *byte == b'\n')
            .map(|relative| start + relative)
            .unwrap_or(bytes.len());
        return Some(
            String::from_utf8_lossy(&bytes[start..end])
                .trim()
                .to_string(),
        );
    }

    warnings.push(LazyPdfWarning {
        rule_id: "lazy.header_missing".to_string(),
        message: "PDF header was not found in the first 1024 bytes".to_string(),
        byte_offset: Some(0),
    });
    None
}

fn parse_startxref(
    tail: &[u8],
    tail_absolute_start: usize,
    warnings: &mut Vec<LazyPdfWarning>,
) -> Option<usize> {
    let offset = rfind_subslice(tail, b"startxref")?;
    let number_start = skip_ascii_whitespace(tail, offset + b"startxref".len());
    let (value, _) = parse_unsigned_integer(tail, number_start)?;
    let absolute = tail_absolute_start + offset;
    if value == 0 {
        warnings.push(LazyPdfWarning {
            rule_id: "lazy.startxref_zero".to_string(),
            message: "startxref resolved to offset 0".to_string(),
            byte_offset: Some(absolute),
        });
    }
    Some(value)
}

fn read_at(file: &mut std::fs::File, offset: u64, len: usize) -> Result<Vec<u8>> {
    let mut buffer = vec![0; len];
    file.seek(SeekFrom::Start(offset))?;
    let bytes_read = file.read(&mut buffer)?;
    buffer.truncate(bytes_read);
    Ok(buffer)
}

fn parse_unsigned_integer(input: &[u8], mut cursor: usize) -> Option<(usize, usize)> {
    cursor = skip_ascii_whitespace(input, cursor);
    let start = cursor;
    while input.get(cursor).is_some_and(|byte| byte.is_ascii_digit()) {
        cursor += 1;
    }
    if cursor == start {
        return None;
    }
    let value = std::str::from_utf8(&input[start..cursor])
        .ok()?
        .parse()
        .ok()?;
    Some((value, cursor))
}

fn skip_ascii_whitespace(input: &[u8], mut cursor: usize) -> usize {
    while input
        .get(cursor)
        .is_some_and(|byte| byte.is_ascii_whitespace())
    {
        cursor += 1;
    }
    cursor
}

fn skip_whitespace_and_comments(input: &[u8], mut pos: usize) -> usize {
    loop {
        while input
            .get(pos)
            .is_some_and(|byte| byte.is_ascii_whitespace())
        {
            pos += 1;
        }
        if input.get(pos) == Some(&b'%') {
            while input
                .get(pos)
                .is_some_and(|byte| *byte != b'\n' && *byte != b'\r')
            {
                pos += 1;
            }
            continue;
        }
        return pos;
    }
}

fn trim_single_stream_eol(body: &[u8], data_start: usize, marker_end: usize) -> usize {
    if marker_end <= data_start {
        return marker_end;
    }
    if marker_end >= 2 && body.get(marker_end - 2..marker_end) == Some(b"\r\n") {
        marker_end - 2
    } else if body
        .get(marker_end - 1)
        .is_some_and(|byte| *byte == b'\n' || *byte == b'\r')
    {
        marker_end - 1
    } else {
        marker_end
    }
}

fn collect_lazy_links(
    value: &PdfValue,
    label: &str,
    kind: PageObjectLinkKind,
    links: &mut Vec<PageObjectLink>,
) {
    match value {
        PdfValue::Reference(reference) => links.push(PageObjectLink {
            label: label.to_string(),
            kind,
            reference: *reference,
        }),
        PdfValue::Array(values) => {
            for value in values {
                collect_lazy_links(value, label, kind.clone(), links);
            }
        }
        _ => {}
    }
}

fn count_references(value: &PdfValue) -> usize {
    match value {
        PdfValue::Reference(_) => 1,
        PdfValue::Array(values) => values.iter().map(count_references).sum(),
        _ => 0,
    }
}

fn dedupe_page_links(links: &mut Vec<PageObjectLink>) {
    let mut seen = BTreeSet::new();
    links.retain(|link| {
        seen.insert((
            page_link_kind_label(&link.kind),
            link.label.clone(),
            link.reference,
        ))
    });
}

fn page_link_kind_label(kind: &PageObjectLinkKind) -> &'static str {
    match kind {
        PageObjectLinkKind::Page => "page",
        PageObjectLinkKind::Parent => "parent",
        PageObjectLinkKind::Resources => "resources",
        PageObjectLinkKind::Contents => "contents",
        PageObjectLinkKind::Font => "font",
        PageObjectLinkKind::XObject => "xobject",
        PageObjectLinkKind::Image => "image",
        PageObjectLinkKind::Annotation => "annotation",
    }
}

fn page_box_from_array(values: &[PdfValue]) -> Option<PageBox> {
    if values.len() != 4 {
        return None;
    }
    let lower_left_x = number(&values[0])?;
    let lower_left_y = number(&values[1])?;
    let upper_right_x = number(&values[2])?;
    let upper_right_y = number(&values[3])?;
    Some(PageBox {
        lower_left_x,
        lower_left_y,
        upper_right_x,
        upper_right_y,
        width: (upper_right_x - lower_left_x).abs(),
        height: (upper_right_y - lower_left_y).abs(),
    })
}

fn number(value: &PdfValue) -> Option<f64> {
    match value {
        PdfValue::Number(value) => Some(*value),
        _ => None,
    }
}

fn skip_blank_lines(bytes: &[u8], mut cursor: usize) -> usize {
    loop {
        let end = line_end(bytes, cursor);
        if bytes
            .get(cursor..end)
            .is_some_and(|line| line.iter().all(|byte| byte.is_ascii_whitespace()))
        {
            cursor = end.saturating_add(1);
            if cursor >= bytes.len() {
                return cursor;
            }
        } else {
            return cursor;
        }
    }
}

fn line_end(bytes: &[u8], start: usize) -> usize {
    bytes
        .get(start..)
        .and_then(|tail| {
            tail.iter()
                .position(|byte| *byte == b'\n' || *byte == b'\r')
                .map(|relative| start + relative)
        })
        .unwrap_or(bytes.len())
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn rfind_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(haystack.len());
    }
    haystack
        .windows(needle.len())
        .rposition(|window| window == needle)
}

fn count_occurrences(haystack: &[u8], needle: &[u8]) -> usize {
    let mut count = 0;
    let mut cursor = 0;
    while let Some(relative) = find_subslice(&haystack[cursor..], needle) {
        count += 1;
        cursor += relative + needle.len();
    }
    count
}

#[allow(dead_code)]
fn _xref_entries_for_full_model(document: &LazyPdfDocument) -> Vec<XrefEntry> {
    document
        .xref
        .entries
        .iter()
        .map(|entry| XrefEntry {
            reference: entry.reference,
            offset: entry.offset,
            in_use: entry.in_use,
            valid_offset: None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lazy_open_reads_metadata_without_full_scan() {
        let directory =
            std::env::temp_dir().join(format!("pdf-debugger-lazy-open-{}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("create temp dir");
        let path = directory.join("lazy.pdf");
        std::fs::write(&path, minimal_pdf()).expect("write lazy fixture");

        let document = open_lazy_pdf(&path).expect("lazy open");

        assert_eq!(document.metadata.pdf_version.as_deref(), Some("1.4"));
        assert_eq!(document.metadata.root, Some(ObjectRef::new(1, 0)));
        assert_eq!(document.metadata.page_count, Some(1));
        assert_eq!(document.metadata.object_count, 5);
        assert_eq!(document.metadata.xref_entry_count, 6);

        let inspection = inspect_lazy_object(&path, ObjectRef::new(1, 0)).expect("inspect object");
        assert_eq!(inspection.object_type, "dictionary");
        assert_eq!(inspection.references, vec![ObjectRef::new(2, 0)]);

        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn lazy_page_index_reads_pages_and_inherited_metadata() {
        let directory =
            std::env::temp_dir().join(format!("pdf-debugger-lazy-pages-{}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("create temp dir");
        let path = directory.join("lazy-pages.pdf");
        std::fs::write(&path, page_pdf()).expect("write lazy page fixture");

        let mut document = open_lazy_pdf(&path).expect("lazy open");
        let index = build_lazy_page_index(&mut document);

        assert_eq!(index.pages.len(), 1);
        let page = &index.pages[0];
        assert_eq!(page.reference, ObjectRef::new(3, 0));
        assert_eq!(page.media_box.unwrap().width, 200.0);
        assert_eq!(page.media_box.unwrap().height, 300.0);
        assert_eq!(page.crop_box.unwrap().lower_left_x, 10.0);
        assert_eq!(page.rotation, Some(90));
        assert_eq!(page.resources.fonts, 1);
        assert_eq!(page.resources.xobjects, 1);
        assert_eq!(page.resources.images, 1);
        assert_eq!(page.resources.contents, 1);
        assert_eq!(page.resources.annotations, 1);
        assert!(page.links.iter().any(|link| {
            link.kind == PageObjectLinkKind::Image && link.reference == ObjectRef::new(6, 0)
        }));

        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn lazy_stream_view_reads_selected_stream_bytes() {
        let directory =
            std::env::temp_dir().join(format!("pdf-debugger-lazy-stream-{}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("create temp dir");
        let path = directory.join("lazy-stream.pdf");
        std::fs::write(&path, page_pdf()).expect("write lazy stream fixture");

        let view = view_lazy_stream(&path, ObjectRef::new(4, 0)).expect("view lazy stream");

        assert_eq!(view.reference, ObjectRef::new(4, 0));
        assert_eq!(view.declared_length, Some(23));
        assert_eq!(view.actual_length, 23);
        assert_eq!(view.decoded_length, Some(23));
        assert!(view.decode_issues.is_empty());
        assert!(view.raw_text.contains("BT /F1 12 Tf"));
        assert!(view.hex_text.contains("42 54 20 2f"));
        assert_eq!(
            view.decoded_text.as_deref(),
            Some("BT /F1 12 Tf (Hi) Tj ET")
        );
        assert!(view.can_export_raw);
        assert!(view.can_export_decoded);

        let raw = read_lazy_stream_raw_bytes(&path, ObjectRef::new(4, 0)).expect("raw bytes");
        assert_eq!(raw, b"BT /F1 12 Tf (Hi) Tj ET");
        let decoded =
            read_lazy_stream_decoded_bytes(&path, ObjectRef::new(4, 0)).expect("decoded bytes");
        assert_eq!(decoded, b"BT /F1 12 Tf (Hi) Tj ET");

        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn lazy_open_supports_xref_stream_offsets() {
        let directory =
            std::env::temp_dir().join(format!("pdf-debugger-xref-stream-{}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("create temp dir");
        let path = directory.join("xref-stream.pdf");
        std::fs::write(&path, xref_stream_pdf()).expect("write xref stream fixture");

        let document = open_lazy_pdf(&path).expect("lazy open xref stream PDF");
        assert_eq!(document.metadata.root, Some(ObjectRef::new(1, 0)));
        assert!(document.metadata.has_xref_stream);
        assert_eq!(document.metadata.page_count, Some(1));

        let inspection = inspect_lazy_object(&path, ObjectRef::new(3, 0)).expect("inspect page");
        assert_eq!(inspection.reference, ObjectRef::new(3, 0));
        assert_eq!(inspection.object_type, "dictionary");

        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn lazy_object_inspector_expands_object_stream_member() {
        let directory =
            std::env::temp_dir().join(format!("pdf-debugger-obj-stream-{}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("create temp dir");
        let path = directory.join("object-stream.pdf");
        std::fs::write(&path, object_stream_pdf()).expect("write object stream fixture");

        let document = open_lazy_pdf(&path).expect("lazy open object stream PDF");
        assert!(document.metadata.has_object_stream);

        let inspection =
            inspect_lazy_object(&path, ObjectRef::new(5, 0)).expect("inspect compressed object");
        assert_eq!(inspection.reference, ObjectRef::new(5, 0));
        assert_eq!(inspection.object_type, "dictionary");
        assert!(inspection.dictionary_keys.contains(&"BaseFont".to_string()));

        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn lazy_xref_merges_incremental_prev_chain_with_latest_entries_winning() {
        let directory =
            std::env::temp_dir().join(format!("pdf-debugger-prev-chain-{}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("create temp dir");
        let path = directory.join("incremental-prev.pdf");
        std::fs::write(&path, incremental_prev_pdf()).expect("write incremental fixture");

        let document = open_lazy_pdf(&path).expect("lazy open incremental PDF");
        assert_eq!(document.metadata.root, Some(ObjectRef::new(1, 0)));
        assert_eq!(document.metadata.object_count, 6);
        assert_eq!(document.metadata.xref_entry_count, 7);

        let original_page = inspect_lazy_object(&path, ObjectRef::new(3, 0)).expect("inspect page");
        assert_eq!(original_page.object_type, "dictionary");

        let updated_font =
            inspect_lazy_object(&path, ObjectRef::new(5, 0)).expect("inspect updated font");
        assert!(updated_font
            .dictionary_keys
            .contains(&"Updated".to_string()));

        let new_font = inspect_lazy_object(&path, ObjectRef::new(6, 0)).expect("inspect new font");
        assert!(new_font.dictionary_keys.contains(&"Added".to_string()));

        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn lazy_xref_reads_hybrid_xref_stream_entries() {
        let directory =
            std::env::temp_dir().join(format!("pdf-debugger-hybrid-xref-{}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("create temp dir");
        let path = directory.join("hybrid-xref.pdf");
        std::fs::write(&path, hybrid_xref_pdf()).expect("write hybrid fixture");

        let document = open_lazy_pdf(&path).expect("lazy open hybrid PDF");
        assert!(document.metadata.has_xref_stream);
        assert!(document.metadata.has_object_stream);
        assert_eq!(document.metadata.root, Some(ObjectRef::new(1, 0)));

        let page = inspect_lazy_object(&path, ObjectRef::new(3, 0)).expect("inspect classic page");
        assert_eq!(page.object_type, "dictionary");

        let compressed_font =
            inspect_lazy_object(&path, ObjectRef::new(5, 0)).expect("inspect hybrid font");
        assert_eq!(compressed_font.object_type, "dictionary");
        assert!(compressed_font
            .dictionary_keys
            .contains(&"BaseFont".to_string()));

        let _ = std::fs::remove_dir_all(directory);
    }

    fn minimal_pdf() -> Vec<u8> {
        let mut bytes = b"%PDF-1.4\n".to_vec();
        let offsets = [
            push(
                &mut bytes,
                b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n",
            ),
            push(
                &mut bytes,
                b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 200 200] >>\nendobj\n",
            ),
            push(
                &mut bytes,
                b"3 0 obj\n<< /Type /Page /Parent 2 0 R /Contents 4 0 R >>\nendobj\n",
            ),
            push(
                &mut bytes,
                b"4 0 obj\n<< /Length 23 >>\nstream\nBT /F1 12 Tf (Hi) Tj ET\nendstream\nendobj\n",
            ),
            push(
                &mut bytes,
                b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n",
            ),
        ];
        let xref = bytes.len();
        bytes.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
        for offset in offsets {
            bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        bytes.extend_from_slice(
            format!("trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes(),
        );
        bytes
    }

    fn page_pdf() -> Vec<u8> {
        let mut bytes = b"%PDF-1.4\n".to_vec();
        let offsets = [
            push(
                &mut bytes,
                b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n",
            ),
            push(
                &mut bytes,
                b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 200 300] /CropBox [10 20 190 280] /Rotate 90 /Resources << /Font << /F1 5 0 R >> /XObject << /Im1 6 0 R >> >> >>\nendobj\n",
            ),
            push(
                &mut bytes,
                b"3 0 obj\n<< /Type /Page /Parent 2 0 R /BleedBox [5 5 195 290] /Contents 4 0 R /Annots [7 0 R] >>\nendobj\n",
            ),
            push(
                &mut bytes,
                b"4 0 obj\n<< /Length 23 >>\nstream\nBT /F1 12 Tf (Hi) Tj ET\nendstream\nendobj\n",
            ),
            push(
                &mut bytes,
                b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n",
            ),
            push(
                &mut bytes,
                b"6 0 obj\n<< /Type /XObject /Subtype /Image /Width 1 /Height 1 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Length 3 >>\nstream\nabc\nendstream\nendobj\n",
            ),
            push(
                &mut bytes,
                b"7 0 obj\n<< /Type /Annot /Subtype /Text /Rect [0 0 10 10] >>\nendobj\n",
            ),
        ];
        let xref = bytes.len();
        bytes.extend_from_slice(b"xref\n0 8\n0000000000 65535 f \n");
        for offset in offsets {
            bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        bytes.extend_from_slice(
            format!("trailer\n<< /Size 8 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes(),
        );
        bytes
    }

    fn xref_stream_pdf() -> Vec<u8> {
        let mut bytes = b"%PDF-1.5\n".to_vec();
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
            b"3 0 obj\n<< /Type /Page /Parent 2 0 R /Contents 4 0 R >>\nendobj\n",
        );
        let object_4 = push(
            &mut bytes,
            b"4 0 obj\n<< /Length 23 >>\nstream\nBT /F1 12 Tf (Hi) Tj ET\nendstream\nendobj\n",
        );
        let xref_offset = bytes.len();
        let mut xref_data = Vec::new();
        push_xref_stream_entry(&mut xref_data, 0, 0, 65535);
        for offset in [object_1, object_2, object_3, object_4, xref_offset] {
            push_xref_stream_entry(&mut xref_data, 1, offset, 0);
        }
        bytes.extend_from_slice(
            format!(
                "5 0 obj\n<< /Type /XRef /Size 6 /Root 1 0 R /W [1 4 2] /Length {} >>\nstream\n",
                xref_data.len()
            )
            .as_bytes(),
        );
        bytes.extend_from_slice(&xref_data);
        bytes.extend_from_slice(
            format!("\nendstream\nendobj\nstartxref\n{xref_offset}\n%%EOF\n").as_bytes(),
        );
        bytes
    }

    fn object_stream_pdf() -> Vec<u8> {
        let mut bytes = b"%PDF-1.5\n".to_vec();
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
        let object_4 = push(
            &mut bytes,
            b"4 0 obj\n<< /Length 23 >>\nstream\nBT /F1 12 Tf (Hi) Tj ET\nendstream\nendobj\n",
        );
        let compressed = b"5 0 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>";
        let object_6 = push(
            &mut bytes,
            format!(
                "6 0 obj\n<< /Type /ObjStm /N 1 /First 4 /Length {} >>\nstream\n",
                compressed.len()
            )
            .as_bytes(),
        );
        bytes.extend_from_slice(compressed);
        bytes.extend_from_slice(b"\nendstream\nendobj\n");
        let xref_offset = bytes.len();
        let mut xref_data = Vec::new();
        push_xref_stream_entry(&mut xref_data, 0, 0, 65535);
        for offset in [object_1, object_2, object_3, object_4] {
            push_xref_stream_entry(&mut xref_data, 1, offset, 0);
        }
        push_xref_stream_entry(&mut xref_data, 2, 6, 0);
        push_xref_stream_entry(&mut xref_data, 1, object_6, 0);
        push_xref_stream_entry(&mut xref_data, 1, xref_offset, 0);
        bytes.extend_from_slice(
            format!(
                "7 0 obj\n<< /Type /XRef /Size 8 /Root 1 0 R /W [1 4 2] /Length {} >>\nstream\n",
                xref_data.len()
            )
            .as_bytes(),
        );
        bytes.extend_from_slice(&xref_data);
        bytes.extend_from_slice(
            format!("\nendstream\nendobj\nstartxref\n{xref_offset}\n%%EOF\n").as_bytes(),
        );
        bytes
    }

    fn incremental_prev_pdf() -> Vec<u8> {
        let mut bytes = b"%PDF-1.4\n".to_vec();
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
            b"3 0 obj\n<< /Type /Page /Parent 2 0 R /Resources << /Font << /F1 5 0 R /F2 6 0 R >> >> /Contents 4 0 R >>\nendobj\n",
        );
        let object_4 = push(
            &mut bytes,
            b"4 0 obj\n<< /Length 23 >>\nstream\nBT /F1 12 Tf (Hi) Tj ET\nendstream\nendobj\n",
        );
        let object_5_original = push(
            &mut bytes,
            b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n",
        );
        let first_xref = bytes.len();
        bytes.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
        for offset in [object_1, object_2, object_3, object_4, object_5_original] {
            bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        bytes.extend_from_slice(
            format!("trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{first_xref}\n%%EOF\n")
                .as_bytes(),
        );

        let object_5_updated = push(
            &mut bytes,
            b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Courier /Updated true >>\nendobj\n",
        );
        let object_6 = push(
            &mut bytes,
            b"6 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Times-Roman /Added true >>\nendobj\n",
        );
        let second_xref = bytes.len();
        bytes.extend_from_slice(b"xref\n5 2\n");
        for offset in [object_5_updated, object_6] {
            bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        bytes.extend_from_slice(
            format!(
                "trailer\n<< /Size 7 /Root 1 0 R /Prev {first_xref} >>\nstartxref\n{second_xref}\n%%EOF\n"
            )
            .as_bytes(),
        );
        bytes
    }

    fn hybrid_xref_pdf() -> Vec<u8> {
        let mut bytes = b"%PDF-1.5\n".to_vec();
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
        let object_4 = push(
            &mut bytes,
            b"4 0 obj\n<< /Length 23 >>\nstream\nBT /F1 12 Tf (Hi) Tj ET\nendstream\nendobj\n",
        );
        let compressed = b"5 0 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>";
        let object_6 = push(
            &mut bytes,
            format!(
                "6 0 obj\n<< /Type /ObjStm /N 1 /First 4 /Length {} >>\nstream\n",
                compressed.len()
            )
            .as_bytes(),
        );
        bytes.extend_from_slice(compressed);
        bytes.extend_from_slice(b"\nendstream\nendobj\n");

        let xref_stream_offset = bytes.len();
        let mut xref_data = Vec::new();
        push_xref_stream_entry(&mut xref_data, 2, 6, 0);
        bytes.extend_from_slice(
            format!(
                "7 0 obj\n<< /Type /XRef /Size 8 /Index [5 1] /W [1 4 2] /Length {} >>\nstream\n",
                xref_data.len()
            )
            .as_bytes(),
        );
        bytes.extend_from_slice(&xref_data);
        bytes.extend_from_slice(b"\nendstream\nendobj\n");

        let classic_xref = bytes.len();
        bytes.extend_from_slice(b"xref\n0 8\n0000000000 65535 f \n");
        for offset in [object_1, object_2, object_3, object_4] {
            bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        bytes.extend_from_slice(b"0000000000 00000 f \n");
        for offset in [object_6, xref_stream_offset] {
            bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        bytes.extend_from_slice(
            format!(
                "trailer\n<< /Size 8 /Root 1 0 R /XRefStm {xref_stream_offset} >>\nstartxref\n{classic_xref}\n%%EOF\n"
            )
            .as_bytes(),
        );
        bytes
    }

    fn push_xref_stream_entry(bytes: &mut Vec<u8>, entry_type: u8, field_2: usize, field_3: usize) {
        bytes.push(entry_type);
        bytes.extend_from_slice(&(field_2 as u32).to_be_bytes());
        bytes.extend_from_slice(&(field_3 as u16).to_be_bytes());
    }

    fn push(bytes: &mut Vec<u8>, object: &[u8]) -> usize {
        let offset = bytes.len();
        bytes.extend_from_slice(object);
        offset
    }
}
