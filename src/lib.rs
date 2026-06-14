pub mod content_ops;
pub mod diagnostics;
pub mod lazy_deep_diagnostics;
pub mod lazy_diagnostics;
pub mod lazy_pdf;
pub mod object_inspector;
pub mod object_tree;
pub mod page_index;
pub mod page_objects;
pub mod pdf_edit;
pub mod pdf_model;
pub mod pdf_parser;
pub mod pdf_string;
pub mod report;
pub mod stream_decode;
pub mod stream_viewer;

use std::path::PathBuf;

pub use diagnostics::run_diagnostics;
pub use lazy_deep_diagnostics::{
    run_lazy_deep_diagnostics, run_lazy_object_deep_diagnostics, run_lazy_page_deep_diagnostics,
    run_lazy_stream_deep_diagnostics, LazyDeepDiagnosticRequest, LazyEnrichmentDetail,
    LazyReportEnrichment,
};
pub use lazy_diagnostics::{
    build_lazy_diagnostics_report, run_lazy_fast_diagnostics,
    run_lazy_fast_diagnostics_with_page_index, LazyDiagnosticsReport,
};
pub use lazy_pdf::{
    build_lazy_page_index, build_lazy_page_list, build_lazy_page_metadata, inspect_lazy_object,
    inspect_lazy_object_with_document, lazy_pdf_to_object_tree, open_lazy_pdf,
    read_lazy_object_for_tree, read_lazy_object_for_tree_with_document,
    read_lazy_stream_decoded_bytes, read_lazy_stream_decoded_bytes_with_document,
    read_lazy_stream_for_content, read_lazy_stream_for_content_with_document,
    read_lazy_stream_metadata_with_document, read_lazy_stream_preview_bytes_with_document,
    read_lazy_stream_raw_bytes, read_lazy_stream_raw_bytes_with_document, view_lazy_stream,
    view_lazy_stream_with_document, LazyPdfDocument, LazyPdfWarning, LazyStreamView, LazyXrefEntry,
    LazyXrefIndex, LAZY_STREAM_DECODE_EXPORT_LIMIT, LAZY_STREAM_DECODE_INPUT_LIMIT,
    LAZY_STREAM_EXPORT_LIMIT, LAZY_STREAM_PREVIEW_LIMIT,
};
pub use object_inspector::{
    inspect_object, inspect_object_shallow, ObjectInspection, StreamInspection,
};
pub use object_tree::{build_object_tree, ObjectTreeNode, ObjectTreeNodeKind};
pub use page_index::{
    build_page_index, PageBox, PageIndex, PageObjectLink, PageObjectLinkKind, PageResourceSummary,
    PageSummary,
};
pub use page_objects::{
    inspect_page_objects, PageObject, PageObjectBounds, PageObjectInspection, PageObjectKind,
    PageObjectProperty, PageObjectWarning,
};
pub use pdf_edit::{
    parse_edit_value, save_modified_pdf_as, save_modified_pdf_in_place, PdfEditPathSegment,
    PdfObjectEdit, PdfSaveAsResult, PdfStreamEdit,
};
pub use pdf_model::{
    DiagnosticSummary, Finding, ObjectRef, ParsedPdf, PdfMetadata, PdfObject, PdfStream, PdfValue,
    Severity,
};
pub use pdf_parser::{parse_bytes, parse_file, parse_file_without_stream_bytes};
pub use pdf_string::{
    decode_pdf_hex_string, decode_pdf_string, DecodedPdfString, PdfStringEncoding,
};
pub use report::{
    build_lazy_report, build_lazy_report_with_enrichments, render_lazy_json_report,
    render_lazy_markdown_report, LazyDebugReport,
};
pub use stream_viewer::{
    hex_dump, inspect_content_stream, inspect_stream, ContentStreamView, ContentStreamViewError,
    StreamView,
};

pub type Result<T> = std::result::Result<T, PdfDebuggerError>;

#[derive(Debug, thiserror::Error)]
pub enum PdfDebuggerError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("object {reference} was not found")]
    ObjectNotFound { reference: ObjectRef },

    #[error("object {reference} is not a stream")]
    StreamNotFound { reference: ObjectRef },

    #[error("failed to parse PDF at byte offset {offset}: {message}")]
    Parse { offset: usize, message: String },

    #[error("PDF is too large for the current MVP full-file parser: {size} bytes selected, safe limit is {limit} bytes. This file was not parsed; future large-PDF support needs streaming or lazy parsing.")]
    FileTooLarge { size: u64, limit: u64 },

    #[error("stream decode failed for {reference}: {message}")]
    StreamDecode {
        reference: ObjectRef,
        message: String,
    },

    #[error("lazy PDF open failed: {message}")]
    LazyOpen { message: String },

    #[error("no output path was provided")]
    MissingOutputPath,

    #[error("path is not valid UTF-8: {0:?}")]
    NonUtf8Path(PathBuf),

    #[error("conflicting output modes: {message}")]
    ConflictingOutputModes { message: String },
}
