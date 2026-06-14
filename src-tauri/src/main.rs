#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use image::{ImageBuffer, ImageFormat, Luma, Rgb, Rgba};
use pdf_debugger::content_ops::{ContentOperator, ContentToken, ContentWarning};
use pdf_debugger::pdf_model::{
    ByteRange, DiagnosticSummary, Finding, ObjectRef, PdfDictionary, PdfMetadata, PdfObject,
    PdfStream, PdfValue,
};
use pdf_debugger::stream_decode::{decode_stream, DecodeIssue};
use pdf_debugger::{
    build_object_tree, build_page_index, hex_dump, inspect_content_stream,
    inspect_object as inspect_pdf_object, inspect_object_shallow,
    inspect_page_objects as inspect_pdf_page_objects, parse_file_without_stream_bytes,
    ObjectInspection, ObjectTreeNode, PageIndex, PageObjectInspection, PageObjectLinkKind,
    ParsedPdf, PdfDebuggerError, PdfEditPathSegment, PdfObjectEdit, PdfStreamEdit,
    LAZY_STREAM_PREVIEW_LIMIT,
};
use pdfium_render::prelude::{PdfRenderConfig, Pdfium};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, VecDeque};
use std::ffi::OsString;
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tauri::Manager;

#[cfg(windows)]
fn apply_native_window_corners(window: &tauri::WebviewWindow) {
    use std::ffi::c_void;
    use windows_sys::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE,
    };

    const DWMWCP_ROUND: u32 = 2;

    let Ok(hwnd) = window.hwnd() else {
        return;
    };
    let preference = DWMWCP_ROUND;
    unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd.0 as _,
            DWMWA_WINDOW_CORNER_PREFERENCE as _,
            &preference as *const u32 as *const c_void,
            std::mem::size_of_val(&preference) as u32,
        );
    }
}

#[cfg(not(windows))]
fn apply_native_window_corners(_window: &tauri::WebviewWindow) {}

async fn run_blocking<T, F>(task: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(task)
        .await
        .map_err(|error| format!("Background task failed: {error}"))?
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OpenPdfSummary {
    path: String,
    metadata: PdfMetadata,
    trailer: GuiTrailerView,
    diagnostics: DiagnosticSummary,
    findings: Vec<Finding>,
    object_tree: ObjectTreeNode,
    page_index: PageIndex,
    annotations: GuiAnnotationsView,
    open_mode: String,
    capability_warnings: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct GuiTrailerView {
    entries: Vec<GuiTrailerEntry>,
    nodes: Vec<GuiTrailerNode>,
    warnings: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GuiTrailerEntry {
    key: String,
    value_type: String,
    value: String,
    reference: Option<ObjectRef>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GuiTrailerNode {
    kind: String,
    key: String,
    value_type: String,
    value: String,
    reference: Option<ObjectRef>,
    expandable: bool,
    children: Vec<GuiTrailerNode>,
    stream: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GuiTrailerObjectNode {
    reference: ObjectRef,
    node: GuiTrailerNode,
    warnings: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct GuiAcroFormView {
    fields: Vec<GuiAcroFormField>,
    warnings: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct GuiAcroFormField {
    name: String,
    reference: ObjectRef,
    field_type: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct GuiAnnotationsView {
    annotations: Vec<GuiAnnotationSummary>,
    warnings: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GuiAnnotationSummary {
    page_number: u32,
    page_reference: ObjectRef,
    reference: ObjectRef,
    subtype: Option<String>,
    rect: Option<String>,
    flags: Option<String>,
    contents: Option<String>,
    color: Option<String>,
    border: Option<String>,
    appearance: Option<String>,
    ca: Option<String>,
    keys: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GuiObjectInspection {
    #[serde(flatten)]
    inspection: ObjectInspection,
    raw_preview: String,
    object_node: GuiTrailerNode,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GuiStreamView {
    reference: ObjectRef,
    declared_length: Option<usize>,
    raw_length: usize,
    raw_range: ByteRange,
    decoded_length: Option<usize>,
    filters: Vec<String>,
    decode_issues: Vec<DecodeIssue>,
    warnings: Vec<String>,
    raw_text_truncated: bool,
    hex_text_truncated: bool,
    decoded_text_truncated: bool,
    preview_limit: Option<usize>,
    can_export_raw: bool,
    can_export_decoded: bool,
    raw_text: String,
    hex_text: String,
    decoded_text: Option<String>,
    decoded_error: Option<String>,
    image: Option<GuiStreamImageMetadata>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GuiStreamMetadata {
    reference: ObjectRef,
    declared_length: Option<usize>,
    raw_length: usize,
    raw_range: ByteRange,
    filters: Vec<String>,
    decode_issues: Vec<DecodeIssue>,
    warnings: Vec<String>,
    can_export_raw: bool,
    can_export_decoded: bool,
    image: Option<GuiStreamImageMetadata>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GuiStreamPreview {
    reference: ObjectRef,
    mode: StreamPreviewMode,
    text: Option<String>,
    decoded_length: Option<usize>,
    decode_issues: Vec<DecodeIssue>,
    warnings: Vec<String>,
    truncated: bool,
    preview_limit: Option<usize>,
    can_export_raw: bool,
    can_export_decoded: bool,
    error: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GuiStreamImageMetadata {
    width: Option<usize>,
    height: Option<usize>,
    color_space: Option<String>,
    bits_per_component: Option<usize>,
    subtype: Option<String>,
    renderable: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GuiStreamImagePreview {
    reference: ObjectRef,
    path: String,
    format: String,
    width: u32,
    height: u32,
    source: String,
    warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GuiContentStreamView {
    reference: ObjectRef,
    decoded_length: usize,
    filters: Vec<String>,
    tokens: Vec<ContentToken>,
    operators: Vec<ContentOperator>,
    warnings: Vec<ContentWarning>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GuiPdfEditPathSegment {
    kind: String,
    key: Option<String>,
    index: Option<usize>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GuiPdfObjectEdit {
    object: u32,
    generation: u16,
    path: Vec<GuiPdfEditPathSegment>,
    value: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GuiPdfStreamEdit {
    object: u32,
    generation: u16,
    decoded_text: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GuiValidatedEditValue {
    value_type: String,
    value: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GuiSaveModifiedPdfResult {
    path: String,
    bytes_written: usize,
    object_count: usize,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GuiDraftPreviewSnapshot {
    path: String,
    revision: u64,
    bytes_written: usize,
    object_count: usize,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GuiPagePreview {
    path: String,
    page_number: u32,
    zoom: f32,
    pixel_width: u16,
    pixel_height: u16,
    renderer: String,
    source_kind: String,
    loaded_from: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RendererStatus {
    available: bool,
    renderer: String,
    source_kind: String,
    loaded_from: Option<String>,
    message: String,
    environment_variable: String,
    attempted_sources: Vec<String>,
}

#[derive(Debug)]
struct PdfiumRuntime {
    pdfium: Pdfium,
    info: PdfiumRuntimeInfo,
}

#[derive(Clone, Debug)]
struct PdfiumRuntimeInfo {
    source_kind: String,
    loaded_from: Option<String>,
    attempted_sources: Vec<String>,
}

#[derive(Debug)]
struct CachedPdfiumRuntime {
    pdfium: Pdfium,
    info: PdfiumRuntimeInfo,
}

static PDFIUM_RUNTIME: OnceLock<Mutex<Option<CachedPdfiumRuntime>>> = OnceLock::new();
static PDFIUM_RENDER_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
static FULL_DOCUMENT_CACHE: OnceLock<Mutex<FullDocumentCache>> = OnceLock::new();
static STREAM_VIEW_CACHE: OnceLock<Mutex<StreamViewCache>> = OnceLock::new();
static STREAM_PREVIEW_CACHE: OnceLock<Mutex<StreamPreviewCache>> = OnceLock::new();
static PAGE_PREVIEW_CACHE: OnceLock<Mutex<PagePreviewCache>> = OnceLock::new();
static LATEST_PAGE_PREVIEW_REQUEST: OnceLock<Mutex<Option<PagePreviewRequestMarker>>> =
    OnceLock::new();
const PAGE_PREVIEW_BASE_WIDTH: f32 = 960.0;
const PAGE_PREVIEW_BASE_MAX_HEIGHT: f32 = 1280.0;
const PAGE_PREVIEW_MIN_ZOOM: f32 = 0.5;
const PAGE_PREVIEW_MAX_ZOOM: f32 = 4.0;
const PAGE_PREVIEW_MAX_RENDER_WIDTH: i32 = 3840;
const PAGE_PREVIEW_MAX_RENDER_HEIGHT: i32 = 5120;
const FULL_DOCUMENT_CACHE_LIMIT: usize = 4;
const LAZY_PAGE_METADATA_PREFETCH_RADIUS: i32 = 1;
const STREAM_VIEW_CACHE_LIMIT: usize = 1024;
const STREAM_PREVIEW_CACHE_LIMIT: usize = 2048;
const PAGE_PREVIEW_CACHE_LIMIT: usize = 512;
const ACROFORM_MAX_DEPTH: usize = 24;
const ACROFORM_MAX_FIELDS: usize = 10_000;
#[cfg(test)]
const GUI_MEDIUM_FULL_LOAD_TEST_SIZE: usize = 1 * 1024 * 1024 + 4096;

#[derive(Clone, Debug, Eq, PartialEq)]
struct LazyDocumentCacheKey {
    path: PathBuf,
    size: u64,
    modified_millis: u128,
}

#[derive(Debug, Default)]
struct FullDocumentCache {
    entries: VecDeque<(LazyDocumentCacheKey, Arc<CachedFullDocument>)>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StreamViewCacheKey {
    document: LazyDocumentCacheKey,
    reference: ObjectRef,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StreamPreviewCacheKey {
    document: LazyDocumentCacheKey,
    reference: ObjectRef,
    mode: StreamPreviewMode,
}

#[derive(Clone, Debug)]
struct PagePreviewCacheKey {
    document: LazyDocumentCacheKey,
    page_number: u32,
    zoom_millis: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PagePreviewRequestMarker {
    document: LazyDocumentCacheKey,
    page_number: u32,
    zoom_millis: u32,
    request_id: u64,
    open_generation: u64,
}

impl PartialEq for PagePreviewCacheKey {
    fn eq(&self, other: &Self) -> bool {
        self.document == other.document
            && self.page_number == other.page_number
            && self.zoom_millis == other.zoom_millis
    }
}

impl Eq for PagePreviewCacheKey {}

#[derive(Debug, Default)]
struct StreamViewCache {
    entries: VecDeque<(StreamViewCacheKey, GuiStreamView)>,
}

#[derive(Debug, Default)]
struct StreamPreviewCache {
    entries: VecDeque<(StreamPreviewCacheKey, GuiStreamPreview)>,
}

#[derive(Debug, Default)]
struct PagePreviewCache {
    entries: VecDeque<(PagePreviewCacheKey, GuiPagePreview)>,
}

#[derive(Clone, Debug)]
struct CachedFullDocument {
    pdf: Arc<ParsedPdf>,
    object_tree: ObjectTreeNode,
    page_index: PageIndex,
    trailer: GuiTrailerView,
    acroform: GuiAcroFormView,
    annotations: GuiAnnotationsView,
}

#[derive(Clone, Debug)]
struct CachedParsedDocument {
    document: Arc<CachedFullDocument>,
    cache_hit: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
enum StreamExportMode {
    Raw,
    Decoded,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
enum StreamPreviewMode {
    Raw,
    Hex,
    Decoded,
}

#[tauri::command]
async fn open_pdf(path: String) -> Result<OpenPdfSummary, String> {
    run_blocking(move || open_pdf_sync(path)).await
}

fn open_pdf_sync(path: String) -> Result<OpenPdfSummary, String> {
    let path = PathBuf::from(path);
    ensure_pdf_path(&path)?;
    let started = Instant::now();

    let result = match parse_file_cached(&path) {
        Ok(cached_pdf) => {
            perf_log(
                "open_pdf.full_document_cache",
                &path,
                started,
                Some(cached_pdf.cache_hit),
            );
            Ok(open_summary_from_cached_document(
                &path,
                &cached_pdf.document,
            ))
        }
        Err(error @ PdfDebuggerError::FileTooLarge { .. }) => Err(full_load_required_error(error)),
        Err(error) => Err(error.to_string()),
    };
    perf_log("open_pdf", &path, started, None);
    result
}

fn open_summary_from_cached_document(path: &Path, document: &CachedFullDocument) -> OpenPdfSummary {
    OpenPdfSummary {
        path: path.display().to_string(),
        metadata: document.pdf.metadata.clone(),
        trailer: document.trailer.clone(),
        diagnostics: DiagnosticSummary::default(),
        findings: Vec::new(),
        object_tree: document.object_tree.clone(),
        page_index: document.page_index.clone(),
        annotations: document.annotations.clone(),
        open_mode: "full".to_string(),
        capability_warnings: Vec::new(),
    }
}

fn full_load_required_error(error: PdfDebuggerError) -> String {
    format!(
        "{error} The desktop GUI requires the PDF to fully load before inspection commands are available."
    )
}

#[tauri::command]
fn renderer_status() -> RendererStatus {
    match bind_pdfium_runtime() {
        Ok(runtime) => RendererStatus {
            available: true,
            renderer: "PDFium".to_string(),
            source_kind: runtime.info.source_kind,
            loaded_from: runtime.info.loaded_from,
            message: "PDFium is available for page preview rendering.".to_string(),
            environment_variable: pdfium_env_var().to_string(),
            attempted_sources: runtime.info.attempted_sources,
        },
        Err(status) => status,
    }
}

#[tauri::command]
async fn inspect_object(
    path: String,
    object: u32,
    generation: u16,
) -> Result<GuiObjectInspection, String> {
    run_blocking(move || inspect_object_sync(path, object, generation)).await
}

fn inspect_object_sync(
    path: String,
    object: u32,
    generation: u16,
) -> Result<GuiObjectInspection, String> {
    let path = PathBuf::from(path);
    ensure_pdf_path(&path)?;

    let reference = ObjectRef::new(object, generation);
    match parse_file_cached(&path) {
        Ok(cached_pdf) => {
            let pdf = &cached_pdf.document.pdf;
            let object = pdf
                .object(reference)
                .ok_or_else(|| format!("Object {reference} was not found"))?;
            let loaded_stream;
            let loaded_object;
            let object = if let Some(stream) = object.stream.as_ref() {
                loaded_stream = stream_with_loaded_bytes(&path, stream)?;
                loaded_object = PdfObject {
                    reference: object.reference,
                    value: object.value.clone(),
                    stream: Some(loaded_stream),
                    raw_range: object.raw_range,
                    raw_bytes: object.raw_bytes.clone(),
                };
                &loaded_object
            } else {
                object
            };
            let inspection = inspect_pdf_object(object);

            Ok(GuiObjectInspection {
                inspection,
                raw_preview: preview_raw_source(object, &path),
                object_node: gui_trailer_object_node(object),
            })
        }
        Err(error @ PdfDebuggerError::FileTooLarge { .. }) => Err(full_load_required_error(error)),
        Err(error) => Err(error.to_string()),
    }
}

#[tauri::command]
async fn load_trailer_object(
    path: String,
    object: u32,
    generation: u16,
) -> Result<GuiTrailerObjectNode, String> {
    run_blocking(move || load_trailer_object_sync(path, object, generation)).await
}

fn load_trailer_object_sync(
    path: String,
    object: u32,
    generation: u16,
) -> Result<GuiTrailerObjectNode, String> {
    let path = PathBuf::from(path);
    ensure_pdf_path(&path)?;

    let reference = ObjectRef::new(object, generation);
    let pdf_object = match parse_file_cached(&path) {
        Ok(cached_pdf) => cached_pdf
            .document
            .pdf
            .object(reference)
            .cloned()
            .ok_or_else(|| format!("Object {reference} was not found"))?,
        Err(error @ PdfDebuggerError::FileTooLarge { .. }) => {
            return Err(full_load_required_error(error));
        }
        Err(error) => return Err(error.to_string()),
    };

    Ok(GuiTrailerObjectNode {
        reference,
        node: gui_trailer_object_node(&pdf_object),
        warnings: Vec::new(),
    })
}

#[tauri::command]
async fn load_acroform(path: String) -> Result<GuiAcroFormView, String> {
    run_blocking(move || load_acroform_sync(path)).await
}

fn load_acroform_sync(path: String) -> Result<GuiAcroFormView, String> {
    let path = PathBuf::from(path);
    ensure_pdf_path(&path)?;
    let started = Instant::now();

    let view = match parse_file_cached(&path) {
        Ok(cached_pdf) => {
            perf_log(
                "load_acroform.full_document_cache",
                &path,
                started,
                Some(cached_pdf.cache_hit),
            );
            cached_pdf.document.acroform.clone()
        }
        Err(error @ PdfDebuggerError::FileTooLarge { .. }) => {
            return Err(full_load_required_error(error));
        }
        Err(error) => return Err(error.to_string()),
    };
    perf_log("load_acroform", &path, started, None);
    Ok(view)
}

fn build_acroform_view_full(pdf: &ParsedPdf) -> GuiAcroFormView {
    let mut view = GuiAcroFormView::default();
    let Some(root) = pdf.root_catalog() else {
        view.warnings
            .push("Root catalog is missing; AcroForm cannot be inspected.".to_string());
        return view;
    };
    let Some(root_dictionary) = root.dictionary() else {
        view.warnings.push(format!(
            "Root catalog {} is not a dictionary.",
            root.reference
        ));
        return view;
    };
    let Some(acroform_value) = root_dictionary.get("AcroForm") else {
        return view;
    };
    let Some(acroform_dictionary) = resolve_full_dictionary(pdf, acroform_value) else {
        view.warnings.push(format!(
            "/AcroForm is {}, not a dictionary reference or dictionary.",
            pdf_value_type_label(acroform_value)
        ));
        return view;
    };
    let Some(fields) = acroform_dictionary.get("Fields") else {
        view.warnings
            .push("AcroForm dictionary does not contain a /Fields array.".to_string());
        return view;
    };
    let Some(fields) = fields.as_array() else {
        view.warnings.push(format!(
            "AcroForm /Fields is {}, not an array.",
            pdf_value_type_label(fields)
        ));
        return view;
    };
    if fields.is_empty() {
        return view;
    }

    let mut context = FullAcroFormContext {
        pdf,
        view: &mut view,
        visited: BTreeSet::new(),
    };
    for value in fields {
        context.walk_field_value(value, "", 0);
    }
    view
}

fn resolve_full_dictionary<'a>(
    pdf: &'a ParsedPdf,
    value: &'a PdfValue,
) -> Option<&'a PdfDictionary> {
    match value {
        PdfValue::Dictionary(dictionary) => Some(dictionary),
        PdfValue::Reference(reference) => pdf.object(*reference)?.dictionary(),
        _ => None,
    }
}

struct FullAcroFormContext<'a> {
    pdf: &'a ParsedPdf,
    view: &'a mut GuiAcroFormView,
    visited: BTreeSet<ObjectRef>,
}

impl FullAcroFormContext<'_> {
    fn walk_field_value(&mut self, value: &PdfValue, parent_name: &str, depth: usize) {
        match value {
            PdfValue::Reference(reference) => {
                self.walk_field_reference(*reference, parent_name, depth)
            }
            PdfValue::Dictionary(dictionary) => {
                self.view.warnings.push(format!(
                    "Direct AcroForm field dictionaries are summarized but cannot be opened as indirect objects: {}.",
                    dictionary
                        .get("T")
                        .map(pdf_string_value)
                        .filter(|name| !name.is_empty())
                        .unwrap_or_else(|| "<unnamed>".to_string())
                ));
                self.walk_field_dictionary(None, dictionary, parent_name, depth);
            }
            _ => self.view.warnings.push(format!(
                "AcroForm field entry is {}, not an indirect reference or dictionary.",
                pdf_value_type_label(value)
            )),
        }
    }

    fn walk_field_reference(&mut self, reference: ObjectRef, parent_name: &str, depth: usize) {
        if depth > ACROFORM_MAX_DEPTH {
            self.view.warnings.push(format!(
                "AcroForm traversal depth limit reached at {reference}."
            ));
            return;
        }
        if self.view.fields.len() >= ACROFORM_MAX_FIELDS {
            self.view.warnings.push(format!(
                "AcroForm field limit reached ({ACROFORM_MAX_FIELDS}); remaining fields were skipped."
            ));
            return;
        }
        if !self.visited.insert(reference) {
            self.view.warnings.push(format!(
                "Cycle detected while walking AcroForm field {reference}."
            ));
            return;
        }
        let Some(object) = self.pdf.object(reference) else {
            self.view
                .warnings
                .push(format!("AcroForm field object {reference} was not found."));
            self.visited.remove(&reference);
            return;
        };
        let Some(dictionary) = object.dictionary() else {
            self.view.warnings.push(format!(
                "AcroForm field object {reference} is not a dictionary."
            ));
            self.visited.remove(&reference);
            return;
        };
        self.walk_field_dictionary(Some(reference), dictionary, parent_name, depth);
        self.visited.remove(&reference);
    }

    fn walk_field_dictionary(
        &mut self,
        reference: Option<ObjectRef>,
        dictionary: &PdfDictionary,
        parent_name: &str,
        depth: usize,
    ) {
        let name = field_full_name(parent_name, dictionary);
        let kids = match dictionary.get("Kids") {
            Some(kids) => match kids.as_array() {
                Some(kids) => Some(kids),
                None => {
                    self.view.warnings.push(format!(
                        "AcroForm field {} has a /Kids entry that is {}, not an array.",
                        reference
                            .map(|reference| reference.to_string())
                            .unwrap_or_else(|| field_display_name(&name, ObjectRef::new(0, 0))),
                        pdf_value_type_label(kids)
                    ));
                    None
                }
            },
            None => None,
        };
        let is_terminal = kids.map(|kids| kids.is_empty()).unwrap_or(true);
        if is_terminal {
            if let Some(reference) = reference {
                self.view.fields.push(GuiAcroFormField {
                    name: field_display_name(&name, reference),
                    reference,
                    field_type: dictionary
                        .get("FT")
                        .and_then(PdfValue::as_name)
                        .map(ToString::to_string),
                });
            }
        }
        if let Some(kids) = kids {
            for child in kids {
                self.walk_field_value(child, &name, depth + 1);
            }
        }
    }
}

fn field_full_name(parent_name: &str, dictionary: &PdfDictionary) -> String {
    let partial = dictionary
        .get("T")
        .map(pdf_string_value)
        .unwrap_or_default();
    match (parent_name.is_empty(), partial.is_empty()) {
        (true, true) => String::new(),
        (true, false) => partial,
        (false, true) => parent_name.to_string(),
        (false, false) => format!("{parent_name}.{partial}"),
    }
}

fn field_display_name(name: &str, reference: ObjectRef) -> String {
    if name.trim().is_empty() {
        format!("<unnamed {}>", reference)
    } else {
        name.to_string()
    }
}

fn pdf_string_value(value: &PdfValue) -> String {
    match value {
        PdfValue::String(value) => value.clone(),
        PdfValue::HexString(_) => value
            .summary()
            .trim_start_matches('(')
            .trim_end_matches(')')
            .to_string(),
        _ => value.summary(),
    }
}

fn sort_acroform_fields(fields: &mut [GuiAcroFormField]) {
    fields.sort_by(|left, right| {
        left.name
            .to_lowercase()
            .cmp(&right.name.to_lowercase())
            .then_with(|| left.reference.cmp(&right.reference))
    });
}

fn build_annotations_view_full(pdf: &ParsedPdf, page_index: &PageIndex) -> GuiAnnotationsView {
    let mut view = GuiAnnotationsView::default();
    let mut seen = BTreeSet::new();

    for page in &page_index.pages {
        for link in page
            .links
            .iter()
            .filter(|link| link.kind == PageObjectLinkKind::Annotation)
        {
            if !seen.insert((page.page_number, link.reference)) {
                continue;
            }

            let Some(object) = pdf.object(link.reference) else {
                view.warnings.push(format!(
                    "Page {} annotation object {} was not found.",
                    page.page_number, link.reference
                ));
                continue;
            };
            let Some(dictionary) = object.dictionary() else {
                view.warnings.push(format!(
                    "Page {} annotation object {} is not a dictionary.",
                    page.page_number, link.reference
                ));
                continue;
            };

            view.annotations.push(GuiAnnotationSummary {
                page_number: page.page_number,
                page_reference: page.reference,
                reference: link.reference,
                subtype: dictionary
                    .get("Subtype")
                    .and_then(PdfValue::as_name)
                    .map(|name| {
                        if name.starts_with('/') {
                            name.to_string()
                        } else {
                            format!("/{name}")
                        }
                    }),
                rect: dictionary.get("Rect").map(PdfValue::summary),
                flags: dictionary.get("F").map(PdfValue::summary),
                contents: dictionary.get("Contents").map(pdf_string_value),
                color: dictionary.get("C").map(PdfValue::summary),
                border: dictionary.get("Border").map(PdfValue::summary),
                appearance: dictionary.get("AP").map(PdfValue::summary),
                ca: dictionary.get("CA").map(PdfValue::summary),
                keys: annotation_dictionary_keys(dictionary),
            });
        }
    }

    sort_annotations(&mut view.annotations);
    view
}

fn annotation_dictionary_keys(dictionary: &PdfDictionary) -> Vec<String> {
    dictionary
        .keys()
        .map(|key| {
            if key.starts_with('/') {
                key.to_string()
            } else {
                format!("/{key}")
            }
        })
        .collect()
}

fn sort_annotations(annotations: &mut [GuiAnnotationSummary]) {
    annotations.sort_by(|left, right| {
        left.page_number
            .cmp(&right.page_number)
            .then_with(|| left.reference.cmp(&right.reference))
    });
}

#[tauri::command]
async fn view_stream_metadata(
    path: String,
    object: u32,
    generation: u16,
) -> Result<GuiStreamMetadata, String> {
    run_blocking(move || view_stream_metadata_sync(path, object, generation)).await
}

fn view_stream_metadata_sync(
    path: String,
    object: u32,
    generation: u16,
) -> Result<GuiStreamMetadata, String> {
    let path = PathBuf::from(path);
    ensure_pdf_path(&path)?;

    let reference = ObjectRef::new(object, generation);
    match parse_file_cached(&path) {
        Ok(cached_pdf) => {
            let pdf = &cached_pdf.document.pdf;
            let object = pdf
                .object(reference)
                .ok_or_else(|| format!("Object {reference} was not found"))?;
            let inspection = inspect_object_shallow(object);
            stream_metadata_from_inspection(
                reference,
                &inspection,
                object.stream.as_ref(),
                Vec::new(),
            )
        }
        Err(error @ PdfDebuggerError::FileTooLarge { .. }) => Err(full_load_required_error(error)),
        Err(error) => Err(error.to_string()),
    }
}

fn stream_metadata_from_inspection(
    reference: ObjectRef,
    inspection: &ObjectInspection,
    source_stream: Option<&PdfStream>,
    warnings: Vec<String>,
) -> Result<GuiStreamMetadata, String> {
    let stream = inspection
        .stream
        .as_ref()
        .ok_or_else(|| format!("Object {reference} is not a stream"))?;
    Ok(GuiStreamMetadata {
        reference,
        declared_length: stream.declared_length,
        raw_length: stream.actual_length,
        raw_range: stream.raw_range,
        filters: stream.filters.clone(),
        decode_issues: stream.decode_issues.clone(),
        warnings,
        can_export_raw: true,
        can_export_decoded: true,
        image: source_stream.map(stream_image_metadata),
    })
}

fn stream_image_metadata(stream: &PdfStream) -> GuiStreamImageMetadata {
    let subtype = stream
        .dictionary
        .get("Subtype")
        .and_then(PdfValue::as_name)
        .map(str::to_string);
    let width = stream.dictionary.get("Width").and_then(PdfValue::as_usize);
    let height = stream.dictionary.get("Height").and_then(PdfValue::as_usize);
    let bits_per_component = stream
        .dictionary
        .get("BitsPerComponent")
        .and_then(PdfValue::as_usize);
    let color_space = image_color_space_name(stream);
    let has_supported_options = !has_unsupported_image_options(stream);
    let renderable = subtype.as_deref() == Some("Image")
        && width.is_some()
        && height.is_some()
        && has_supported_options
        && (is_jpeg_stream(stream)
            || (bits_per_component == Some(8)
                && matches!(
                    color_space.as_deref(),
                    Some("DeviceGray" | "DeviceRGB" | "DeviceCMYK")
                )));

    GuiStreamImageMetadata {
        width,
        height,
        color_space,
        bits_per_component,
        subtype,
        renderable,
    }
}

fn stream_with_loaded_bytes(path: &Path, stream: &PdfStream) -> Result<PdfStream, String> {
    let mut stream = stream.clone();
    if stream.raw_bytes.len() != stream.actual_length {
        stream.raw_bytes = read_file_range(path, stream.raw_range.start, stream.actual_length)
            .map_err(|error| {
                format!(
                    "Could not read stream bytes at {}..{}: {error}",
                    stream.raw_range.start, stream.raw_range.end
                )
            })?;
        stream.actual_length = stream.raw_bytes.len();
    }
    Ok(stream)
}

fn stream_preview_bytes(path: &Path, stream: &PdfStream) -> Result<(Vec<u8>, bool), String> {
    let preview_length = stream.actual_length.min(LAZY_STREAM_PREVIEW_LIMIT);
    if !stream.raw_bytes.is_empty() {
        return Ok((
            stream.raw_bytes[..preview_length.min(stream.raw_bytes.len())].to_vec(),
            stream.actual_length > preview_length,
        ));
    }
    let bytes = read_file_range(path, stream.raw_range.start, preview_length).map_err(|error| {
        format!(
            "Could not read stream preview bytes at {}..{}: {error}",
            stream.raw_range.start,
            stream.raw_range.start.saturating_add(preview_length)
        )
    })?;
    Ok((bytes, stream.actual_length > preview_length))
}

#[tauri::command]
async fn view_stream_preview(
    path: String,
    object: u32,
    generation: u16,
    mode: StreamPreviewMode,
) -> Result<GuiStreamPreview, String> {
    run_blocking(move || view_stream_preview_sync(path, object, generation, mode)).await
}

fn view_stream_preview_sync(
    path: String,
    object: u32,
    generation: u16,
    mode: StreamPreviewMode,
) -> Result<GuiStreamPreview, String> {
    let path = PathBuf::from(path);
    ensure_pdf_path(&path)?;

    let reference = ObjectRef::new(object, generation);
    let document_key = lazy_cache_key(&path)?;
    if let Some(preview) = load_stream_preview_cache(&document_key, reference, mode)? {
        perf_log(
            "view_stream_preview.cached",
            &path,
            Instant::now(),
            Some(true),
        );
        return Ok(preview);
    }

    let preview = match parse_file_cached(&path) {
        Ok(cached_pdf) => {
            let pdf = &cached_pdf.document.pdf;
            let stream = pdf
                .stream(reference)
                .ok_or_else(|| format!("Object {reference} is not a stream"))?;
            stream_preview_from_full_stream(&path, reference, stream, mode)
        }
        Err(error @ PdfDebuggerError::FileTooLarge { .. }) => {
            return Err(full_load_required_error(error));
        }
        Err(error) => return Err(error.to_string()),
    }?;

    store_stream_preview_cache(document_key, reference, mode, preview.clone())?;
    Ok(preview)
}

fn stream_preview_from_full_stream(
    path: &Path,
    reference: ObjectRef,
    stream: &PdfStream,
    mode: StreamPreviewMode,
) -> Result<GuiStreamPreview, String> {
    match mode {
        StreamPreviewMode::Raw => {
            let (bytes, truncated) = stream_preview_bytes(path, stream)?;
            let preview_length = bytes.len();
            Ok(GuiStreamPreview {
                reference,
                mode,
                text: Some(String::from_utf8_lossy(&bytes).into_owned()),
                decoded_length: None,
                decode_issues: Vec::new(),
                warnings: preview_warnings(
                    stream.raw_range,
                    "Raw",
                    stream.actual_length,
                    preview_length,
                ),
                truncated,
                preview_limit: Some(LAZY_STREAM_PREVIEW_LIMIT),
                can_export_raw: true,
                can_export_decoded: false,
                error: None,
            })
        }
        StreamPreviewMode::Hex => {
            let (bytes, truncated) = stream_preview_bytes(path, stream)?;
            let preview_length = bytes.len();
            Ok(GuiStreamPreview {
                reference,
                mode,
                text: Some(hex_dump(&bytes)),
                decoded_length: None,
                decode_issues: Vec::new(),
                warnings: preview_warnings(
                    stream.raw_range,
                    "Hex",
                    stream.actual_length,
                    preview_length,
                ),
                truncated,
                preview_limit: Some(LAZY_STREAM_PREVIEW_LIMIT),
                can_export_raw: true,
                can_export_decoded: false,
                error: None,
            })
        }
        StreamPreviewMode::Decoded => {
            let stream = stream_with_loaded_bytes(path, stream)?;
            let decode = decode_stream(&stream);
            let decoded_length = (!decode.has_issues()).then_some(decode.decoded_length);
            let decode_issues = decode.issues;
            if !decode_issues.is_empty() {
                let error = format_decode_issues_for_gui(&decode_issues);
                return Ok(GuiStreamPreview {
                    reference,
                    mode,
                    text: None,
                    decoded_length,
                    decode_issues,
                    warnings: Vec::new(),
                    truncated: false,
                    preview_limit: Some(LAZY_STREAM_PREVIEW_LIMIT),
                    can_export_raw: true,
                    can_export_decoded: false,
                    error: Some(error),
                });
            }
            let preview_length = decode.decoded.len().min(LAZY_STREAM_PREVIEW_LIMIT);
            Ok(GuiStreamPreview {
                reference,
                mode,
                text: Some(String::from_utf8_lossy(&decode.decoded[..preview_length]).into_owned()),
                decoded_length,
                decode_issues,
                warnings: preview_warnings(
                    stream.raw_range,
                    "Decoded",
                    decode.decoded.len(),
                    preview_length,
                ),
                truncated: decode.decoded.len() > preview_length,
                preview_limit: Some(LAZY_STREAM_PREVIEW_LIMIT),
                can_export_raw: true,
                can_export_decoded: true,
                error: None,
            })
        }
    }
}

#[tauri::command]
async fn render_stream_image_preview(
    path: String,
    object: u32,
    generation: u16,
) -> Result<GuiStreamImagePreview, String> {
    run_blocking(move || render_stream_image_preview_sync(path, object, generation)).await
}

fn render_stream_image_preview_sync(
    path: String,
    object: u32,
    generation: u16,
) -> Result<GuiStreamImagePreview, String> {
    let path = PathBuf::from(path);
    ensure_pdf_path(&path)?;

    let reference = ObjectRef::new(object, generation);
    let started = Instant::now();
    let preview = match parse_file_cached(&path) {
        Ok(cached_pdf) => {
            let pdf = &cached_pdf.document.pdf;
            let stream = pdf
                .stream(reference)
                .ok_or_else(|| format!("Object {reference} is not a stream"))?;
            let stream = stream_with_loaded_bytes(&path, stream)?;
            render_image_stream_to_file(&path, reference, &stream)
        }
        Err(error @ PdfDebuggerError::FileTooLarge { .. }) => {
            return Err(full_load_required_error(error));
        }
        Err(error) => return Err(error.to_string()),
    };
    perf_log("render_stream_image_preview", &path, started, None);
    preview
}

fn render_image_stream_to_file(
    pdf_path: &Path,
    reference: ObjectRef,
    stream: &PdfStream,
) -> Result<GuiStreamImagePreview, String> {
    ensure_image_xobject(reference, stream)?;
    let width = image_dimension(stream, "Width")?;
    let height = image_dimension(stream, "Height")?;
    ensure_supported_image_decode_params(stream)?;
    let decode = decode_stream(stream);
    if decode.has_issues() {
        return Err(format!(
            "Image stream could not be decoded: {}",
            format_decode_issues_for_gui(&decode.issues)
        ));
    }

    if is_jpeg_stream(stream) {
        let output_path = stream_image_preview_path(pdf_path, reference, "png")?;
        let (preview_width, preview_height) =
            write_dct_image_preview(&output_path, width, height, &decode.decoded)?;
        return Ok(GuiStreamImagePreview {
            reference,
            path: output_path.display().to_string(),
            format: "png".to_string(),
            width: preview_width,
            height: preview_height,
            source: "decoded DCTDecode JPEG".to_string(),
            warnings: Vec::new(),
        });
    }

    let bits_per_component = stream
        .dictionary
        .get("BitsPerComponent")
        .and_then(PdfValue::as_usize)
        .unwrap_or(8);
    if bits_per_component != 8 {
        return Err(format!(
            "Image preview supports 8-bit image data for now; this image uses {bits_per_component} bits per component."
        ));
    }

    let color_space = image_color_space_name(stream)
        .ok_or_else(|| "Image stream is missing a supported /ColorSpace.".to_string())?;
    let output_path = stream_image_preview_path(pdf_path, reference, "png")?;
    match color_space.as_str() {
        "DeviceGray" => write_gray_image_preview(&output_path, width, height, &decode.decoded)?,
        "DeviceRGB" => write_rgb_image_preview(&output_path, width, height, &decode.decoded)?,
        "DeviceCMYK" => write_cmyk_image_preview(&output_path, width, height, &decode.decoded)?,
        other => {
            return Err(format!(
                "Image preview supports /DeviceGray, /DeviceRGB, /DeviceCMYK, and /DCTDecode JPEG streams for now; /ColorSpace /{other} is not supported."
            ));
        }
    }

    Ok(GuiStreamImagePreview {
        reference,
        path: output_path.display().to_string(),
        format: "png".to_string(),
        width,
        height,
        source: format!("decoded {color_space} pixels"),
        warnings: Vec::new(),
    })
}

fn ensure_image_xobject(reference: ObjectRef, stream: &PdfStream) -> Result<(), String> {
    let subtype = stream.dictionary.get("Subtype").and_then(PdfValue::as_name);
    if subtype != Some("Image") {
        return Err(format!(
            "Object {reference} is not an image XObject stream."
        ));
    }
    Ok(())
}

fn image_dimension(stream: &PdfStream, key: &str) -> Result<u32, String> {
    let value = stream
        .dictionary
        .get(key)
        .and_then(PdfValue::as_usize)
        .ok_or_else(|| format!("Image stream is missing numeric /{key}."))?;
    u32::try_from(value).map_err(|_| format!("Image /{key} is too large to render."))
}

fn ensure_supported_image_decode_params(stream: &PdfStream) -> Result<(), String> {
    if matches!(
        stream.dictionary.get("ImageMask"),
        Some(PdfValue::Boolean(true))
    ) {
        return Err(
            "Image preview does not yet support /ImageMask streams; use Page Preview for mask rendering."
                .to_string(),
        );
    }
    if stream.dictionary.get("Decode").is_some() {
        return Err(
            "Image preview does not yet apply custom /Decode arrays; use Page Preview for renderer-accurate output."
                .to_string(),
        );
    }
    let Some(decode_parms) = stream.dictionary.get("DecodeParms") else {
        return Ok(());
    };
    if matches!(decode_parms, PdfValue::Null) {
        return Ok(());
    }
    let predictor = match decode_parms {
        PdfValue::Dictionary(dictionary) => {
            dictionary.get("Predictor").and_then(PdfValue::as_usize)
        }
        PdfValue::Array(values) => values.iter().find_map(|value| {
            value
                .as_dictionary()
                .and_then(|dictionary| dictionary.get("Predictor"))
                .and_then(PdfValue::as_usize)
        }),
        _ => None,
    }
    .unwrap_or(1);
    if predictor == 1 {
        Ok(())
    } else {
        Err(format!(
            "Image preview does not yet apply /DecodeParms /Predictor {predictor}; export decoded bytes or use Page Preview for this image."
        ))
    }
}

fn has_unsupported_image_options(stream: &PdfStream) -> bool {
    matches!(
        stream.dictionary.get("ImageMask"),
        Some(PdfValue::Boolean(true))
    ) || stream.dictionary.get("Decode").is_some()
        || image_decode_predictor(stream)
            .map(|predictor| predictor != 1)
            .unwrap_or(false)
}

fn image_decode_predictor(stream: &PdfStream) -> Option<usize> {
    let decode_parms = stream.dictionary.get("DecodeParms")?;
    match decode_parms {
        PdfValue::Dictionary(dictionary) => {
            dictionary.get("Predictor").and_then(PdfValue::as_usize)
        }
        PdfValue::Array(values) => values.iter().find_map(|value| {
            value
                .as_dictionary()
                .and_then(|dictionary| dictionary.get("Predictor"))
                .and_then(PdfValue::as_usize)
        }),
        _ => None,
    }
}

fn image_color_space_name(stream: &PdfStream) -> Option<String> {
    match stream.dictionary.get("ColorSpace") {
        Some(PdfValue::Name(name)) => Some(name.clone()),
        Some(PdfValue::Array(values)) => values
            .first()
            .and_then(PdfValue::as_name)
            .map(str::to_string),
        _ => None,
    }
}

fn is_jpeg_stream(stream: &PdfStream) -> bool {
    stream
        .filters
        .iter()
        .any(|filter| matches!(filter.as_str(), "DCTDecode" | "DCT"))
}

fn stream_image_preview_path(
    pdf_path: &Path,
    reference: ObjectRef,
    extension: &str,
) -> Result<PathBuf, String> {
    let directory = std::env::temp_dir().join("pdf-debugger-stream-images");
    fs::create_dir_all(&directory).map_err(|error| {
        format!(
            "Could not create stream image preview directory {}: {error}",
            directory.display()
        )
    })?;
    let stem = pdf_path
        .file_stem()
        .map(|stem| sanitize_file_stem(&stem.to_string_lossy()))
        .filter(|stem| !stem.is_empty())
        .unwrap_or_else(|| "pdf".to_string());
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    Ok(directory.join(format!(
        "{stem}-object-{}-{}-{nonce}.{extension}",
        reference.object, reference.generation
    )))
}

fn write_gray_image_preview(
    path: &Path,
    width: u32,
    height: u32,
    bytes: &[u8],
) -> Result<(), String> {
    let expected = image_expected_len(width, height, 1)?;
    if bytes.len() < expected {
        return Err(format!(
            "Decoded image data is shorter than expected: {} byte(s), expected {expected}.",
            bytes.len()
        ));
    }
    let image =
        ImageBuffer::<Luma<u8>, Vec<u8>>::from_raw(width, height, bytes[..expected].to_vec())
            .ok_or_else(|| "Could not build grayscale image preview buffer.".to_string())?;
    image.save(path).map_err(|error| {
        format!(
            "Could not write PNG image preview {}: {error}",
            path.display()
        )
    })
}

fn write_rgb_image_preview(
    path: &Path,
    width: u32,
    height: u32,
    bytes: &[u8],
) -> Result<(), String> {
    let expected = image_expected_len(width, height, 3)?;
    if bytes.len() < expected {
        return Err(format!(
            "Decoded image data is shorter than expected: {} byte(s), expected {expected}.",
            bytes.len()
        ));
    }
    let image =
        ImageBuffer::<Rgb<u8>, Vec<u8>>::from_raw(width, height, bytes[..expected].to_vec())
            .ok_or_else(|| "Could not build RGB image preview buffer.".to_string())?;
    image.save(path).map_err(|error| {
        format!(
            "Could not write PNG image preview {}: {error}",
            path.display()
        )
    })
}

fn write_cmyk_image_preview(
    path: &Path,
    width: u32,
    height: u32,
    bytes: &[u8],
) -> Result<(), String> {
    let expected = image_expected_len(width, height, 4)?;
    if bytes.len() < expected {
        return Err(format!(
            "Decoded image data is shorter than expected: {} byte(s), expected {expected}.",
            bytes.len()
        ));
    }
    let mut rgba = Vec::with_capacity(image_expected_len(width, height, 4)?);
    for pixel in bytes[..expected].chunks_exact(4) {
        let c = pixel[0] as u16;
        let m = pixel[1] as u16;
        let y = pixel[2] as u16;
        let k = pixel[3] as u16;
        rgba.push(255u8.saturating_sub((c + k).min(255) as u8));
        rgba.push(255u8.saturating_sub((m + k).min(255) as u8));
        rgba.push(255u8.saturating_sub((y + k).min(255) as u8));
        rgba.push(255);
    }
    let image = ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(width, height, rgba)
        .ok_or_else(|| "Could not build CMYK image preview buffer.".to_string())?;
    image.save(path).map_err(|error| {
        format!(
            "Could not write PNG image preview {}: {error}",
            path.display()
        )
    })
}

fn write_dct_image_preview(
    path: &Path,
    declared_width: u32,
    declared_height: u32,
    bytes: &[u8],
) -> Result<(u32, u32), String> {
    let image = image::load_from_memory_with_format(bytes, ImageFormat::Jpeg).map_err(|error| {
        format!("Could not decode /DCTDecode JPEG image stream for preview: {error}")
    })?;
    let rgba = image.to_rgba8();
    let width = rgba.width();
    let height = rgba.height();
    if width != declared_width || height != declared_height {
        return Err(format!(
            "Decoded JPEG dimensions {width} x {height} do not match PDF image dictionary {declared_width} x {declared_height}."
        ));
    }
    rgba.save(path).map_err(|error| {
        format!(
            "Could not write PNG image preview {}: {error}",
            path.display()
        )
    })?;
    Ok((width, height))
}

fn image_expected_len(width: u32, height: u32, components: usize) -> Result<usize, String> {
    (width as usize)
        .checked_mul(height as usize)
        .and_then(|pixels| pixels.checked_mul(components))
        .ok_or_else(|| "Image dimensions are too large to render.".to_string())
}

fn preview_warnings(
    range: ByteRange,
    label: &str,
    full_length: usize,
    preview_length: usize,
) -> Vec<String> {
    if full_length > preview_length {
        vec![format!(
            "{label} preview is truncated to {preview_length} bytes from {full_length} total bytes at {}..{}",
            range.start, range.end
        )]
    } else {
        Vec::new()
    }
}

fn format_decode_issues_for_gui(issues: &[DecodeIssue]) -> String {
    let message = issues
        .iter()
        .map(|issue| format!("/{}: {}", issue.filter, issue.message))
        .collect::<Vec<_>>()
        .join("; ");
    if message.is_empty() {
        "Decoded stream bytes are unavailable.".to_string()
    } else {
        message
    }
}

#[tauri::command]
async fn view_stream(path: String, object: u32, generation: u16) -> Result<GuiStreamView, String> {
    run_blocking(move || view_stream_sync(path, object, generation)).await
}

fn view_stream_sync(path: String, object: u32, generation: u16) -> Result<GuiStreamView, String> {
    let path = PathBuf::from(path);
    ensure_pdf_path(&path)?;

    let reference = ObjectRef::new(object, generation);
    let document_key = lazy_cache_key(&path)?;
    if let Some(view) = load_stream_view_cache(&document_key, reference)? {
        perf_log("view_stream.cached", &path, Instant::now(), Some(true));
        return Ok(view);
    }
    match parse_file_cached(&path) {
        Ok(cached_pdf) => {
            let pdf = &cached_pdf.document.pdf;
            let stream = pdf
                .stream(reference)
                .ok_or_else(|| format!("Object {reference} is not a stream"))?;
            let stream = stream_with_loaded_bytes(&path, stream)?;
            let decode = decode_stream(&stream);
            let decoded_length = (!decode.has_issues()).then_some(decode.decoded_length);
            let decoded_error = decode.has_issues().then(|| {
                decode
                    .issues
                    .iter()
                    .map(|issue| format!("/{}: {}", issue.filter, issue.message))
                    .collect::<Vec<_>>()
                    .join("; ")
            });
            let decoded_text = if decode.has_issues() {
                None
            } else {
                Some(String::from_utf8_lossy(&decode.decoded).into_owned())
            };

            let view = GuiStreamView {
                reference,
                declared_length: stream.declared_length,
                raw_length: stream.actual_length,
                raw_range: stream.raw_range,
                decoded_length,
                filters: stream.filters.clone(),
                decode_issues: decode.issues,
                warnings: Vec::new(),
                raw_text_truncated: false,
                hex_text_truncated: false,
                decoded_text_truncated: false,
                preview_limit: None,
                can_export_raw: true,
                can_export_decoded: decoded_length.is_some(),
                raw_text: String::from_utf8_lossy(&stream.raw_bytes).into_owned(),
                hex_text: hex_dump(&stream.raw_bytes),
                decoded_text,
                decoded_error,
                image: Some(stream_image_metadata(&stream)),
            };
            store_stream_view_cache(document_key, reference, view.clone())?;
            Ok(view)
        }
        Err(error @ PdfDebuggerError::FileTooLarge { .. }) => Err(full_load_required_error(error)),
        Err(error) => Err(error.to_string()),
    }
}

#[tauri::command]
async fn view_content_stream(
    path: String,
    object: u32,
    generation: u16,
) -> Result<GuiContentStreamView, String> {
    run_blocking(move || view_content_stream_sync(path, object, generation)).await
}

fn view_content_stream_sync(
    path: String,
    object: u32,
    generation: u16,
) -> Result<GuiContentStreamView, String> {
    let path = PathBuf::from(path);
    ensure_pdf_path(&path)?;

    let reference = ObjectRef::new(object, generation);
    let stream = match parse_file_cached(&path) {
        Ok(cached_pdf) => {
            let pdf = &cached_pdf.document.pdf;
            let stream = pdf
                .stream(reference)
                .ok_or_else(|| format!("Object {reference} is not a stream"))?
                .clone();
            stream_with_loaded_bytes(&path, &stream)?
        }
        Err(error @ PdfDebuggerError::FileTooLarge { .. }) => {
            return Err(full_load_required_error(error));
        }
        Err(error) => return Err(error.to_string()),
    };

    ensure_content_stream_candidate(reference, &stream)?;
    match inspect_content_stream(reference, &stream) {
        Ok(view) => Ok(GuiContentStreamView {
            reference: view.reference,
            decoded_length: view.decoded_length,
            filters: view.filters,
            tokens: view.analysis.tokens,
            operators: view.analysis.operators,
            warnings: view.analysis.warnings,
        }),
        Err(error) => Err(error.message),
    }
}

fn ensure_content_stream_candidate(reference: ObjectRef, stream: &PdfStream) -> Result<(), String> {
    let type_name = stream.dictionary.get("Type").and_then(PdfValue::as_name);
    let subtype_name = stream.dictionary.get("Subtype").and_then(PdfValue::as_name);

    if matches!(subtype_name, Some("Image")) {
        return Err(format!(
            "Object {reference} is an image XObject stream, not a PDF page content stream. Use Raw or Hex preview, or Export Decoded for the image bytes."
        ));
    }

    if matches!(type_name, Some("XObject")) {
        return Err(format!(
            "Object {reference} is an XObject stream ({}) rather than a page content stream. Content operator analysis is skipped.",
            subtype_name.unwrap_or("unknown subtype")
        ));
    }

    Ok(())
}

#[tauri::command]
async fn inspect_page_objects(
    path: String,
    page_number: u32,
) -> Result<PageObjectInspection, String> {
    run_blocking(move || inspect_page_objects_sync(path, page_number)).await
}

#[tauri::command]
async fn validate_edit_value(value: String) -> Result<GuiValidatedEditValue, String> {
    run_blocking(move || {
        let parsed = pdf_debugger::parse_edit_value(&value).map_err(|error| error.to_string())?;
        Ok(GuiValidatedEditValue {
            value_type: pdf_value_type_label(&parsed).to_string(),
            value: parsed.summary(),
        })
    })
    .await
}

#[tauri::command]
async fn save_modified_pdf_as(
    path: String,
    output_path: String,
    edits: Vec<GuiPdfObjectEdit>,
    stream_edits: Vec<GuiPdfStreamEdit>,
) -> Result<GuiSaveModifiedPdfResult, String> {
    run_blocking(move || save_modified_pdf_as_sync(path, output_path, edits, stream_edits)).await
}

#[tauri::command]
async fn save_modified_pdf_in_place(
    path: String,
    edits: Vec<GuiPdfObjectEdit>,
    stream_edits: Vec<GuiPdfStreamEdit>,
) -> Result<GuiSaveModifiedPdfResult, String> {
    run_blocking(move || save_modified_pdf_in_place_sync(path, edits, stream_edits)).await
}

#[tauri::command]
async fn create_draft_preview_snapshot(
    path: String,
    edits: Vec<GuiPdfObjectEdit>,
    stream_edits: Vec<GuiPdfStreamEdit>,
    revision: u64,
) -> Result<GuiDraftPreviewSnapshot, String> {
    run_blocking(move || create_draft_preview_snapshot_sync(path, edits, stream_edits, revision))
        .await
}

fn save_modified_pdf_as_sync(
    path: String,
    output_path: String,
    edits: Vec<GuiPdfObjectEdit>,
    stream_edits: Vec<GuiPdfStreamEdit>,
) -> Result<GuiSaveModifiedPdfResult, String> {
    let path = PathBuf::from(path);
    ensure_pdf_path(&path)?;
    let output_path = PathBuf::from(output_path);
    if output_path.exists() && !output_path.is_file() {
        return Err(format!(
            "Output path is not a file: {}",
            output_path.display()
        ));
    }
    if same_existing_path(&path, &output_path) {
        return Err(
            "Save As refused to overwrite the opened PDF. Choose a different output path."
                .to_string(),
        );
    }

    let result = write_modified_pdf(&path, &output_path, edits, stream_edits)?;
    invalidate_backend_caches_for_path(&output_path);
    Ok(GuiSaveModifiedPdfResult {
        path: output_path.display().to_string(),
        bytes_written: result.bytes_written,
        object_count: result.object_count,
    })
}

fn save_modified_pdf_in_place_sync(
    path: String,
    edits: Vec<GuiPdfObjectEdit>,
    stream_edits: Vec<GuiPdfStreamEdit>,
) -> Result<GuiSaveModifiedPdfResult, String> {
    let path = PathBuf::from(path);
    ensure_pdf_path(&path)?;
    let result = write_modified_pdf_in_place(&path, edits, stream_edits)?;
    invalidate_backend_caches_for_path(&path);
    Ok(GuiSaveModifiedPdfResult {
        path: path.display().to_string(),
        bytes_written: result.bytes_written,
        object_count: result.object_count,
    })
}

fn create_draft_preview_snapshot_sync(
    path: String,
    edits: Vec<GuiPdfObjectEdit>,
    stream_edits: Vec<GuiPdfStreamEdit>,
    revision: u64,
) -> Result<GuiDraftPreviewSnapshot, String> {
    let path = PathBuf::from(path);
    ensure_pdf_path(&path)?;
    if edits.is_empty() && stream_edits.is_empty() {
        return Err("No draft edits are available to preview.".to_string());
    }

    let output_path = draft_preview_snapshot_path(&path, revision)?;
    let result = write_modified_pdf(&path, &output_path, edits, stream_edits)?;
    Ok(GuiDraftPreviewSnapshot {
        path: output_path.display().to_string(),
        revision,
        bytes_written: result.bytes_written,
        object_count: result.object_count,
    })
}

fn write_modified_pdf(
    input_path: &Path,
    output_path: &Path,
    edits: Vec<GuiPdfObjectEdit>,
    stream_edits: Vec<GuiPdfStreamEdit>,
) -> Result<pdf_debugger::PdfSaveAsResult, String> {
    let parsed_edits = edits
        .into_iter()
        .map(gui_object_edit_to_core)
        .collect::<Result<Vec<_>, _>>()?;
    let parsed_stream_edits = stream_edits
        .into_iter()
        .map(gui_stream_edit_to_core)
        .collect::<Vec<_>>();
    pdf_debugger::save_modified_pdf_as(input_path, output_path, &parsed_edits, &parsed_stream_edits)
        .map_err(|error| error.to_string())
}

fn write_modified_pdf_in_place(
    input_path: &Path,
    edits: Vec<GuiPdfObjectEdit>,
    stream_edits: Vec<GuiPdfStreamEdit>,
) -> Result<pdf_debugger::PdfSaveAsResult, String> {
    let parsed_edits = edits
        .into_iter()
        .map(gui_object_edit_to_core)
        .collect::<Result<Vec<_>, _>>()?;
    let parsed_stream_edits = stream_edits
        .into_iter()
        .map(gui_stream_edit_to_core)
        .collect::<Vec<_>>();
    pdf_debugger::save_modified_pdf_in_place(input_path, &parsed_edits, &parsed_stream_edits)
        .map_err(|error| error.to_string())
}

fn gui_object_edit_to_core(edit: GuiPdfObjectEdit) -> Result<PdfObjectEdit, String> {
    let value = pdf_debugger::parse_edit_value(&edit.value).map_err(|error| error.to_string())?;
    let path = edit
        .path
        .into_iter()
        .map(gui_edit_path_segment_to_core)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(PdfObjectEdit {
        reference: ObjectRef::new(edit.object, edit.generation),
        path,
        value,
    })
}

fn gui_stream_edit_to_core(edit: GuiPdfStreamEdit) -> PdfStreamEdit {
    PdfStreamEdit {
        reference: ObjectRef::new(edit.object, edit.generation),
        decoded_text: edit.decoded_text,
    }
}

fn gui_edit_path_segment_to_core(
    segment: GuiPdfEditPathSegment,
) -> Result<PdfEditPathSegment, String> {
    match segment.kind.as_str() {
        "dict" => segment
            .key
            .map(PdfEditPathSegment::DictionaryKey)
            .ok_or_else(|| "Dictionary edit path segment is missing key.".to_string()),
        "array" => segment
            .index
            .map(PdfEditPathSegment::ArrayIndex)
            .ok_or_else(|| "Array edit path segment is missing index.".to_string()),
        other => Err(format!("Unsupported edit path segment kind: {other}")),
    }
}

fn inspect_page_objects_sync(
    path: String,
    page_number: u32,
) -> Result<PageObjectInspection, String> {
    let path = PathBuf::from(path);
    ensure_pdf_path(&path)?;
    if page_number == 0 {
        return Err("Page numbers start at 1.".to_string());
    }

    let started = Instant::now();
    let cached_pdf = parse_file_cached(&path).map_err(|error| match error {
        error @ PdfDebuggerError::FileTooLarge { .. } => full_load_required_error(error),
        error => error.to_string(),
    })?;
    let page_reference = cached_pdf
        .document
        .page_index
        .pages
        .iter()
        .find(|page| page.page_number == page_number)
        .map(|page| page.reference)
        .ok_or_else(|| format!("Page {page_number} was not found in the parsed page tree"))?;
    let pdf = page_object_analysis_pdf(&path, &cached_pdf.document.pdf, page_reference)?;
    perf_log(
        "inspect_page_objects.hydrate_streams",
        &path,
        started,
        Some(cached_pdf.cache_hit),
    );
    inspect_pdf_page_objects(&pdf, page_number)
        .ok_or_else(|| format!("Page {page_number} was not found in the parsed page tree"))
}

fn page_object_analysis_pdf(
    path: &Path,
    pdf: &ParsedPdf,
    page_reference: ObjectRef,
) -> Result<ParsedPdf, String> {
    let mut stream_references = BTreeSet::new();
    let mut form_stack = BTreeSet::new();
    collect_page_object_analysis_streams(
        pdf,
        page_reference,
        &mut stream_references,
        &mut form_stack,
    );

    let mut hydrated = pdf.clone();
    for reference in stream_references {
        hydrate_stream_bytes_for_analysis(path, &mut hydrated, reference)?;
    }
    Ok(hydrated)
}

fn collect_page_object_analysis_streams(
    pdf: &ParsedPdf,
    page_reference: ObjectRef,
    streams: &mut BTreeSet<ObjectRef>,
    form_stack: &mut BTreeSet<ObjectRef>,
) {
    let Some(page_dictionary) = pdf
        .object(page_reference)
        .and_then(|object| object.dictionary())
    else {
        return;
    };
    if let Some(contents) = page_dictionary.get("Contents") {
        collect_content_stream_references(pdf, contents, streams);
    }
    if let Some(resources) = inherited_page_resources(pdf, page_dictionary) {
        collect_resource_stream_references(pdf, &resources, streams, form_stack, 0);
    }
}

fn inherited_page_resources(pdf: &ParsedPdf, page_dictionary: &PdfDictionary) -> Option<PdfValue> {
    if let Some(resources) = page_dictionary.get("Resources") {
        return Some(resources.clone());
    }
    let mut current = page_dictionary
        .get("Parent")
        .and_then(PdfValue::as_reference);
    while let Some(reference) = current {
        let dictionary = pdf.object(reference)?.dictionary()?;
        if let Some(resources) = dictionary.get("Resources") {
            return Some(resources.clone());
        }
        current = dictionary.get("Parent").and_then(PdfValue::as_reference);
    }
    None
}

fn collect_content_stream_references(
    pdf: &ParsedPdf,
    value: &PdfValue,
    streams: &mut BTreeSet<ObjectRef>,
) {
    match value {
        PdfValue::Reference(reference) => {
            if pdf
                .object(*reference)
                .is_some_and(|object| object.stream.is_some())
            {
                streams.insert(*reference);
            } else if let Some(object) = pdf.object(*reference) {
                collect_content_stream_references(pdf, &object.value, streams);
            }
        }
        PdfValue::Array(values) => {
            for value in values {
                collect_content_stream_references(pdf, value, streams);
            }
        }
        _ => {}
    }
}

fn collect_resource_stream_references(
    pdf: &ParsedPdf,
    value: &PdfValue,
    streams: &mut BTreeSet<ObjectRef>,
    form_stack: &mut BTreeSet<ObjectRef>,
    depth: usize,
) {
    if depth > 4 {
        return;
    }
    let Some(dictionary) = resolve_full_dictionary(pdf, value) else {
        return;
    };

    if let Some(fonts) = dictionary
        .get("Font")
        .and_then(|value| resolve_full_dictionary(pdf, value))
    {
        for font in fonts.values() {
            if let Some(font_dictionary) = resolve_full_dictionary(pdf, font) {
                collect_font_stream_references(pdf, font_dictionary, streams);
            }
        }
    }

    if let Some(xobjects) = dictionary
        .get("XObject")
        .and_then(|value| resolve_full_dictionary(pdf, value))
    {
        for xobject in xobjects.values() {
            let Some(reference) = xobject.as_reference() else {
                continue;
            };
            let Some(object) = pdf.object(reference) else {
                continue;
            };
            let Some(stream) = object.stream.as_ref() else {
                continue;
            };
            let subtype = stream.dictionary.get("Subtype").and_then(PdfValue::as_name);
            if subtype == Some("Form") {
                if !form_stack.insert(reference) {
                    continue;
                }
                streams.insert(reference);
                if let Some(resources) = stream.dictionary.get("Resources") {
                    collect_resource_stream_references(
                        pdf,
                        resources,
                        streams,
                        form_stack,
                        depth + 1,
                    );
                }
                form_stack.remove(&reference);
            }
        }
    }
}

fn collect_font_stream_references(
    pdf: &ParsedPdf,
    dictionary: &PdfDictionary,
    streams: &mut BTreeSet<ObjectRef>,
) {
    if let Some(reference) = dictionary.get("ToUnicode").and_then(PdfValue::as_reference) {
        if pdf
            .object(reference)
            .is_some_and(|object| object.stream.is_some())
        {
            streams.insert(reference);
        }
    }
    if let Some(descendants) = dictionary
        .get("DescendantFonts")
        .and_then(PdfValue::as_array)
    {
        for descendant in descendants {
            if let Some(dictionary) = resolve_full_dictionary(pdf, descendant) {
                collect_font_stream_references(pdf, dictionary, streams);
            }
        }
    }
}

fn hydrate_stream_bytes_for_analysis(
    path: &Path,
    pdf: &mut ParsedPdf,
    reference: ObjectRef,
) -> Result<(), String> {
    let Some(object) = pdf.objects.get_mut(&reference) else {
        return Ok(());
    };
    let Some(stream) = object.stream.as_mut() else {
        return Ok(());
    };
    if stream.raw_bytes.len() == stream.actual_length {
        return Ok(());
    }
    stream.raw_bytes = read_file_range(path, stream.raw_range.start, stream.actual_length)
        .map_err(|error| {
            format!(
                "Could not read stream bytes for Page Objects analysis at {}..{} ({reference}): {error}",
                stream.raw_range.start, stream.raw_range.end
            )
        })?;
    stream.actual_length = stream.raw_bytes.len();
    Ok(())
}

#[tauri::command]
async fn export_stream(
    path: String,
    object: u32,
    generation: u16,
    output_path: String,
    mode: StreamExportMode,
) -> Result<usize, String> {
    run_blocking(move || export_stream_sync(path, object, generation, output_path, mode)).await
}

fn export_stream_sync(
    path: String,
    object: u32,
    generation: u16,
    output_path: String,
    mode: StreamExportMode,
) -> Result<usize, String> {
    let path = PathBuf::from(path);
    ensure_pdf_path(&path)?;

    let output_path = PathBuf::from(output_path);
    if output_path.exists() && !output_path.is_file() {
        return Err(format!(
            "Output path is not a file: {}",
            output_path.display()
        ));
    }
    if same_existing_path(&path, &output_path) {
        return Err("Choose an output path that is different from the opened PDF.".to_string());
    }

    let reference = ObjectRef::new(object, generation);

    let bytes = match parse_file_cached(&path) {
        Ok(cached_pdf) => {
            let pdf = &cached_pdf.document.pdf;
            let stream = pdf
                .stream(reference)
                .ok_or_else(|| format!("Object {reference} is not a stream"))?;
            let stream = stream_with_loaded_bytes(&path, stream)?;
            match mode {
                StreamExportMode::Raw => stream.raw_bytes.clone(),
                StreamExportMode::Decoded => {
                    let decode = decode_stream(&stream);
                    if decode.has_issues() {
                        let message = decode
                            .issues
                            .iter()
                            .map(|issue| format!("/{}: {}", issue.filter, issue.message))
                            .collect::<Vec<_>>()
                            .join("; ");
                        return Err(if message.is_empty() {
                            "Decoded stream bytes are unavailable.".to_string()
                        } else {
                            message
                        });
                    }
                    decode.decoded
                }
            }
        }
        Err(error @ PdfDebuggerError::FileTooLarge { .. }) => {
            return Err(full_load_required_error(error));
        }
        Err(error) => return Err(error.to_string()),
    };

    fs::write(&output_path, &bytes)
        .map_err(|error| format!("Could not write {}: {error}", output_path.display()))?;
    Ok(bytes.len())
}

#[tauri::command]
async fn render_page_preview(
    path: String,
    page_number: u32,
    zoom: Option<f32>,
    request_id: Option<u64>,
    open_generation: Option<u64>,
) -> Result<GuiPagePreview, String> {
    run_blocking(move || {
        render_page_preview_sync(path, page_number, zoom, request_id, open_generation)
    })
    .await
}

fn render_page_preview_sync(
    path: String,
    page_number: u32,
    zoom: Option<f32>,
    request_id: Option<u64>,
    open_generation: Option<u64>,
) -> Result<GuiPagePreview, String> {
    let path = PathBuf::from(path);
    ensure_pdf_path(&path)?;
    if page_number == 0 {
        return Err("Page numbers start at 1.".to_string());
    }

    let zoom = normalize_preview_zoom(zoom);
    let document_key = lazy_cache_key(&path)?;
    let total_started = Instant::now();
    let context = preview_log_context(page_number, zoom, request_id, open_generation);
    if let Some(preview) = load_page_preview_cache(&document_key, page_number, zoom)? {
        perf_log_detail(
            "render_page_preview.cache_hit",
            &path,
            total_started,
            &context,
        );
        return Ok(preview);
    }
    perf_log_detail(
        "render_page_preview.cache_miss",
        &path,
        total_started,
        &context,
    );
    let marker = register_page_preview_request(
        &document_key,
        page_number,
        zoom,
        request_id,
        open_generation,
    );
    perf_log_detail(
        "render_page_preview.register",
        &path,
        total_started,
        &context,
    );
    if let Err(error) = ensure_latest_page_preview_request(&marker) {
        perf_log_detail(
            "render_page_preview.stale_before_bind",
            &path,
            total_started,
            &context,
        );
        return Err(error);
    }
    let bind_started = Instant::now();
    let runtime = bind_pdfium_runtime().map_err(|status| status.message)?;
    perf_log_detail(
        "render_page_preview.bind_pdfium",
        &path,
        bind_started,
        &context,
    );
    if let Err(error) = ensure_latest_page_preview_request(&marker) {
        perf_log_detail(
            "render_page_preview.stale_after_bind",
            &path,
            total_started,
            &context,
        );
        return Err(error);
    }
    let lock_started = Instant::now();
    let _render_guard = PDFIUM_RENDER_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| "PDFium render lock was poisoned.".to_string())?;
    perf_log_detail(
        "render_page_preview.render_lock",
        &path,
        lock_started,
        &context,
    );
    if let Err(error) = ensure_latest_page_preview_request(&marker) {
        perf_log_detail(
            "render_page_preview.stale_before_render",
            &path,
            total_started,
            &context,
        );
        return Err(error);
    }
    let preview = render_page_preview_with_runtime(&runtime, &path, page_number, zoom, &marker)?;
    let cache_started = Instant::now();
    store_page_preview_cache(document_key.clone(), page_number, zoom, preview.clone())?;
    perf_log_detail(
        "render_page_preview.cache_store",
        &path,
        cache_started,
        &context,
    );
    prefetch_adjacent_page_previews(document_key, path, page_number, zoom);
    perf_log_detail(
        "render_page_preview.total",
        Path::new(&preview.path),
        total_started,
        &context,
    );
    Ok(preview)
}

fn register_page_preview_request(
    document: &LazyDocumentCacheKey,
    page_number: u32,
    zoom: f32,
    request_id: Option<u64>,
    open_generation: Option<u64>,
) -> PagePreviewRequestMarker {
    let marker = PagePreviewRequestMarker {
        document: document.clone(),
        page_number,
        zoom_millis: zoom_cache_key(zoom),
        request_id: request_id.unwrap_or(0),
        open_generation: open_generation.unwrap_or(0),
    };
    if request_id.is_some() || open_generation.is_some() {
        if let Ok(mut guard) = LATEST_PAGE_PREVIEW_REQUEST
            .get_or_init(|| Mutex::new(None))
            .lock()
        {
            *guard = Some(marker.clone());
        }
    }
    marker
}

fn ensure_latest_page_preview_request(marker: &PagePreviewRequestMarker) -> Result<(), String> {
    if marker.request_id == 0 && marker.open_generation == 0 {
        return Ok(());
    }
    let guard = LATEST_PAGE_PREVIEW_REQUEST
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|_| "Page preview request tracker was poisoned.".to_string())?;
    match guard.as_ref() {
        Some(latest) if latest == marker => Ok(()),
        _ => Err("Page preview request was superseded by a newer selection.".to_string()),
    }
}

fn render_page_preview_with_runtime(
    runtime: &PdfiumRuntime,
    path: &Path,
    page_number: u32,
    zoom: f32,
    marker: &PagePreviewRequestMarker,
) -> Result<GuiPagePreview, String> {
    let context = preview_log_context(
        page_number,
        zoom,
        Some(marker.request_id),
        Some(marker.open_generation),
    );
    let open_started = Instant::now();
    let document = runtime
        .pdfium
        .load_pdf_from_file(path, None)
        .map_err(|error| format!("PDFium could not open {}: {error}", path.display()))?;
    perf_log_detail("render_page_preview.pdf_open", path, open_started, &context);
    if let Err(error) = ensure_latest_page_preview_request(marker) {
        perf_log_detail(
            "render_page_preview.stale_after_pdf_open",
            path,
            open_started,
            &context,
        );
        return Err(error);
    }
    let page_index = page_number
        .checked_sub(1)
        .and_then(|index| i32::try_from(index).ok())
        .ok_or_else(|| format!("Page {page_number} is outside the renderer index range"))?;
    let page_started = Instant::now();
    let page = document
        .pages()
        .get(page_index)
        .map_err(|error| format!("PDFium could not load page {page_number}: {error}"))?;
    perf_log_detail(
        "render_page_preview.page_lookup",
        path,
        page_started,
        &context,
    );
    if let Err(error) = ensure_latest_page_preview_request(marker) {
        perf_log_detail(
            "render_page_preview.stale_after_page_lookup",
            path,
            page_started,
            &context,
        );
        return Err(error);
    }
    let target_width = preview_render_width(zoom);
    let maximum_height = preview_render_max_height(zoom);
    let render_started = Instant::now();
    let bitmap = page
        .render_with_config(
            &PdfRenderConfig::new()
                .set_target_width(target_width)
                .set_maximum_height(maximum_height),
        )
        .map_err(|error| format!("PDFium could not render page {page_number}: {error}"))?;
    perf_log_detail(
        "render_page_preview.bitmap_render",
        path,
        render_started,
        &context,
    );
    if let Err(error) = ensure_latest_page_preview_request(marker) {
        perf_log_detail(
            "render_page_preview.stale_after_bitmap_render",
            path,
            render_started,
            &context,
        );
        return Err(error);
    }
    let image_started = Instant::now();
    let image = bitmap.as_image().map_err(|error| {
        format!("PDFium rendered page {page_number}, but image conversion failed: {error}")
    })?;
    perf_log_detail(
        "render_page_preview.image_convert",
        path,
        image_started,
        &context,
    );
    let output_path = preview_output_path(&path, page_number)?;
    let write_started = Instant::now();
    image.save(&output_path).map_err(|error| {
        format!(
            "Could not write rendered preview {}: {error}",
            output_path.display()
        )
    })?;
    perf_log_detail(
        "render_page_preview.png_write",
        &output_path,
        write_started,
        &context,
    );

    Ok(GuiPagePreview {
        path: output_path.display().to_string(),
        page_number,
        zoom,
        pixel_width: image.width().min(u32::from(u16::MAX)) as u16,
        pixel_height: image.height().min(u32::from(u16::MAX)) as u16,
        renderer: "PDFium".to_string(),
        source_kind: runtime.info.source_kind.clone(),
        loaded_from: runtime.info.loaded_from.clone(),
    })
}

fn normalize_preview_zoom(zoom: Option<f32>) -> f32 {
    let zoom = zoom.unwrap_or(1.0);
    if zoom.is_finite() {
        zoom.clamp(PAGE_PREVIEW_MIN_ZOOM, PAGE_PREVIEW_MAX_ZOOM)
    } else {
        1.0
    }
}

fn preview_render_width(zoom: f32) -> i32 {
    ((PAGE_PREVIEW_BASE_WIDTH * zoom).round() as i32).clamp(1, PAGE_PREVIEW_MAX_RENDER_WIDTH)
}

fn preview_render_max_height(zoom: f32) -> i32 {
    ((PAGE_PREVIEW_BASE_MAX_HEIGHT * zoom).round() as i32).clamp(1, PAGE_PREVIEW_MAX_RENDER_HEIGHT)
}

fn bind_pdfium_runtime() -> Result<PdfiumRuntime, RendererStatus> {
    let env_var = pdfium_env_var();
    let cache = PDFIUM_RUNTIME.get_or_init(|| Mutex::new(None));
    let mut guard = cache.lock().map_err(|_| {
        pdfium_unavailable_status(
            env_var,
            Vec::new(),
            "PDFium runtime cache could not be locked.".to_string(),
        )
    })?;

    if let Some(cached) = guard.as_ref() {
        if let Some(status) = cached_runtime_env_override_status(cached, env_var) {
            return Err(status);
        }
        return Ok(PdfiumRuntime {
            pdfium: cached.pdfium.clone(),
            info: cached.info.clone(),
        });
    }

    let runtime = load_pdfium_runtime()?;
    let cached = CachedPdfiumRuntime {
        pdfium: runtime.pdfium.clone(),
        info: runtime.info.clone(),
    };
    *guard = Some(cached);
    Ok(runtime)
}

fn cached_runtime_env_override_status(
    cached: &CachedPdfiumRuntime,
    env_var: &str,
) -> Option<RendererStatus> {
    let value = std::env::var(env_var).ok()?;
    let mut attempted_sources = Vec::new();

    if value.trim().is_empty() {
        attempted_sources.push(format!("{env_var} is set but empty"));
        return Some(pdfium_unavailable_status(
            env_var,
            attempted_sources,
            format!(
                "{env_var} is set but empty. Unset it to use the prepared runtime, or restart the app with a valid PDFium dynamic library file or runtime directory."
            ),
        ));
    }

    let configured = PathBuf::from(value.trim());
    let candidate = pdfium_library_path_from_configured_path(&configured);
    attempted_sources.push(format!("{env_var}: {}", candidate.display()));

    let cached_matches_env = cached.info.source_kind == "env"
        && cached
            .info
            .loaded_from
            .as_deref()
            .is_some_and(|loaded| same_configured_pdfium_path(loaded, &candidate));
    if cached_matches_env {
        return None;
    }

    attempted_sources.push(match &cached.info.loaded_from {
        Some(loaded) => format!(
            "PDFium is already initialized from {} ({})",
            loaded, cached.info.source_kind
        ),
        None => format!(
            "PDFium is already initialized from {}",
            cached.info.source_kind
        ),
    });

    Some(pdfium_unavailable_status(
        env_var,
        attempted_sources,
        format!(
            "{env_var} points to {}, but PDFium is already initialized from another source. Restart the app with that explicit override, or unset {env_var} to use the prepared runtime.",
            candidate.display()
        ),
    ))
}

fn same_configured_pdfium_path(loaded: &str, candidate: &Path) -> bool {
    let loaded = PathBuf::from(loaded);
    match (loaded.canonicalize(), candidate.canonicalize()) {
        (Ok(loaded), Ok(candidate)) => loaded == candidate,
        _ => loaded == candidate,
    }
}

fn load_pdfium_runtime() -> Result<PdfiumRuntime, RendererStatus> {
    let mut attempted_sources = Vec::new();
    let env_var = pdfium_env_var();

    if let Ok(value) = std::env::var(env_var) {
        if value.trim().is_empty() {
            attempted_sources.push(format!("{env_var} is set but empty"));
            return Err(pdfium_unavailable_status(
                env_var,
                attempted_sources,
                format!(
                    "{env_var} is set but empty. Unset it to use the prepared runtime, or set it to a PDFium dynamic library file or a directory containing {}.",
                    Pdfium::pdfium_platform_library_name().to_string_lossy()
                ),
            ));
        } else {
            let configured = PathBuf::from(value.trim());
            let candidate = pdfium_library_path_from_configured_path(&configured);
            attempted_sources.push(format!("{env_var}: {}", candidate.display()));
            match Pdfium::bind_to_library(&candidate) {
                Ok(library) => {
                    return Ok(PdfiumRuntime {
                        pdfium: Pdfium::new(library),
                        info: PdfiumRuntimeInfo {
                            source_kind: "env".to_string(),
                            loaded_from: Some(candidate.display().to_string()),
                            attempted_sources,
                        },
                    });
                }
                Err(error) => {
                    attempted_sources
                        .push(format!("Could not load {}: {error}", candidate.display()));
                    return Err(pdfium_unavailable_status(
                        env_var,
                        attempted_sources,
                        format!(
                            "{env_var} points to {}, but PDFium could not be loaded from that explicit override. Unset {env_var} to use the prepared runtime, or set it to a valid PDFium dynamic library file or directory.",
                            candidate.display()
                        ),
                    ));
                }
            }
        }
    } else {
        attempted_sources.push(format!("{env_var} is not set"));
    }

    for directory in downloaded_pdfium_directories() {
        let candidate = Pdfium::pdfium_platform_library_name_at_path(&directory);
        attempted_sources.push(format!("downloaded: {}", candidate.display()));
        match Pdfium::bind_to_library(&candidate) {
            Ok(library) => {
                return Ok(PdfiumRuntime {
                    pdfium: Pdfium::new(library),
                    info: PdfiumRuntimeInfo {
                        source_kind: "downloaded".to_string(),
                        loaded_from: Some(candidate.display().to_string()),
                        attempted_sources,
                    },
                });
            }
            Err(error) => {
                attempted_sources.push(format!("Could not load {}: {error}", candidate.display()))
            }
        }
    }

    for directory in bundled_pdfium_directories() {
        let candidate = Pdfium::pdfium_platform_library_name_at_path(&directory);
        attempted_sources.push(format!("bundled: {}", candidate.display()));
        match Pdfium::bind_to_library(&candidate) {
            Ok(library) => {
                return Ok(PdfiumRuntime {
                    pdfium: Pdfium::new(library),
                    info: PdfiumRuntimeInfo {
                        source_kind: "bundled".to_string(),
                        loaded_from: Some(candidate.display().to_string()),
                        attempted_sources,
                    },
                });
            }
            Err(error) => {
                attempted_sources.push(format!("Could not load {}: {error}", candidate.display()))
            }
        }
    }

    attempted_sources.push(format!(
        "system: {}",
        Pdfium::pdfium_platform_library_name().to_string_lossy()
    ));
    match Pdfium::bind_to_system_library() {
        Ok(library) => Ok(PdfiumRuntime {
            pdfium: Pdfium::new(library),
            info: PdfiumRuntimeInfo {
                source_kind: "system".to_string(),
                loaded_from: Some(
                    Pdfium::pdfium_platform_library_name()
                        .to_string_lossy()
                        .into_owned(),
                ),
                attempted_sources,
            },
        }),
        Err(error) => {
            attempted_sources.push(format!("Could not load system PDFium: {error}"));
            Err(pdfium_unavailable_status(
                env_var,
                attempted_sources,
                format!(
                    "PDFium is unavailable. Run `npm run pdfium:prepare` to download the default runtime, or set {env_var} to a PDFium dynamic library file or a directory containing {}. The page metadata panels remain usable.",
                    Pdfium::pdfium_platform_library_name().to_string_lossy()
                ),
            ))
        }
    }
}

fn pdfium_unavailable_status(
    env_var: &str,
    attempted_sources: Vec<String>,
    message: String,
) -> RendererStatus {
    RendererStatus {
        available: false,
        renderer: "PDFium".to_string(),
        source_kind: "unavailable".to_string(),
        loaded_from: None,
        message,
        environment_variable: env_var.to_string(),
        attempted_sources,
    }
}

fn pdfium_env_var() -> &'static str {
    "PDF_DEBUGGER_PDFIUM_PATH"
}

fn pdfium_library_path_from_configured_path(path: &Path) -> PathBuf {
    if path.is_dir() {
        Pdfium::pdfium_platform_library_name_at_path(path)
    } else {
        path.to_path_buf()
    }
}

fn downloaded_pdfium_directories() -> Vec<PathBuf> {
    let mut directories = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        directories.push(cwd.join("src-tauri").join("resources").join("pdfium"));
        directories.push(cwd.join("resources").join("pdfium"));
    }
    dedupe_paths(directories)
}

fn bundled_pdfium_directories() -> Vec<PathBuf> {
    let mut directories = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            directories.push(exe_dir.join("pdfium"));
            directories.push(exe_dir.to_path_buf());
            directories.push(exe_dir.join("resources").join("pdfium"));
        }
    }
    dedupe_paths(directories)
}

fn dedupe_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut seen = Vec::<OsString>::new();
    let mut output = Vec::new();
    for path in paths {
        let key = path.as_os_str().to_os_string();
        if seen.iter().any(|value| value == &key) {
            continue;
        }
        seen.push(key);
        output.push(path);
    }
    output
}

fn ensure_pdf_path(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Err(format!("File does not exist: {}", path.display()));
    }

    if !path.is_file() {
        return Err(format!("Path is not a file: {}", path.display()));
    }

    let is_pdf = path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("pdf"));
    if !is_pdf {
        return Err("Please choose a local .pdf file.".to_string());
    }

    Ok(())
}

fn lazy_cache_key(path: &Path) -> Result<LazyDocumentCacheKey, String> {
    let canonical = path
        .canonicalize()
        .map_err(|error| format!("Could not resolve {}: {error}", path.display()))?;
    let metadata = fs::metadata(&canonical).map_err(|error| {
        format!(
            "Could not read metadata for {}: {error}",
            canonical.display()
        )
    })?;
    let modified_millis = metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    Ok(LazyDocumentCacheKey {
        path: canonical,
        size: metadata.len(),
        modified_millis,
    })
}

fn parse_file_cached(path: &Path) -> Result<CachedParsedDocument, PdfDebuggerError> {
    let key = lazy_cache_key(path).map_err(|message| PdfDebuggerError::LazyOpen { message })?;
    let cache = FULL_DOCUMENT_CACHE.get_or_init(|| Mutex::new(FullDocumentCache::default()));
    {
        let mut cache = cache.lock().map_err(|_| PdfDebuggerError::LazyOpen {
            message: "Full document cache was poisoned.".to_string(),
        })?;
        if let Some(index) = cache
            .entries
            .iter()
            .position(|(cached_key, _)| cached_key == &key)
        {
            let (_, document) = cache.entries.remove(index).expect("cached parsed PDF");
            let output = Arc::clone(&document);
            cache.entries.push_back((key.clone(), document));
            return Ok(CachedParsedDocument {
                document: output,
                cache_hit: true,
            });
        }
    }

    let parse_started = Instant::now();
    let pdf = Arc::new(parse_file_without_stream_bytes(path)?);
    perf_log("open_pdf.parse_file", path, parse_started, Some(false));
    let document = Arc::new(build_cached_full_document(pdf, path)?);
    let mut cache = cache.lock().map_err(|_| PdfDebuggerError::LazyOpen {
        message: "Full document cache was poisoned.".to_string(),
    })?;
    cache
        .entries
        .push_back((key.clone(), Arc::clone(&document)));
    while cache.entries.len() > FULL_DOCUMENT_CACHE_LIMIT {
        cache.entries.pop_front();
    }
    Ok(CachedParsedDocument {
        document,
        cache_hit: false,
    })
}

fn build_cached_full_document(
    pdf: Arc<ParsedPdf>,
    path: &Path,
) -> Result<CachedFullDocument, PdfDebuggerError> {
    let object_tree_started = Instant::now();
    let page_index_started = Instant::now();
    let trailer_started = Instant::now();
    let acroform_started = Instant::now();
    let annotations_started = Instant::now();

    let object_tree_pdf = Arc::clone(&pdf);
    let object_tree_handle = thread::spawn(move || build_object_tree(&object_tree_pdf));

    let page_index_pdf = Arc::clone(&pdf);
    let page_index_handle = thread::spawn(move || build_page_index(&page_index_pdf));

    let trailer_pdf = Arc::clone(&pdf);
    let trailer_handle = thread::spawn(move || gui_trailer_view(trailer_pdf.trailer.as_ref()));

    let acroform_pdf = Arc::clone(&pdf);
    let acroform_handle = thread::spawn(move || {
        let mut acroform = build_acroform_view_full(&acroform_pdf);
        sort_acroform_fields(&mut acroform.fields);
        acroform
    });

    let object_tree = join_gui_cache_stage("object tree", object_tree_handle)?;
    perf_log(
        "open_pdf.build_object_tree",
        path,
        object_tree_started,
        None,
    );

    let page_index = join_gui_cache_stage("page index", page_index_handle)?;
    perf_log("open_pdf.build_page_index", path, page_index_started, None);

    let annotations = build_annotations_view_full(&pdf, &page_index);
    perf_log(
        "open_pdf.build_annotations",
        path,
        annotations_started,
        None,
    );

    let trailer = join_gui_cache_stage("trailer view", trailer_handle)?;
    perf_log("open_pdf.build_trailer_view", path, trailer_started, None);

    let acroform = join_gui_cache_stage("AcroForm", acroform_handle)?;
    perf_log("open_pdf.build_acroform", path, acroform_started, None);

    Ok(CachedFullDocument {
        pdf,
        object_tree,
        page_index,
        trailer,
        acroform,
        annotations,
    })
}

fn join_gui_cache_stage<T>(
    label: &str,
    handle: thread::JoinHandle<T>,
) -> Result<T, PdfDebuggerError> {
    handle.join().map_err(|_| PdfDebuggerError::LazyOpen {
        message: format!("GUI cache build stage panicked: {label}"),
    })
}

fn load_stream_view_cache(
    document_key: &LazyDocumentCacheKey,
    reference: ObjectRef,
) -> Result<Option<GuiStreamView>, String> {
    let cache = STREAM_VIEW_CACHE.get_or_init(|| Mutex::new(StreamViewCache::default()));
    let mut cache = cache
        .lock()
        .map_err(|_| "Stream Viewer cache was poisoned.".to_string())?;
    let key = StreamViewCacheKey {
        document: document_key.clone(),
        reference,
    };
    Ok(lru_get(&mut cache.entries, &key))
}

fn store_stream_view_cache(
    document_key: LazyDocumentCacheKey,
    reference: ObjectRef,
    view: GuiStreamView,
) -> Result<(), String> {
    let cache = STREAM_VIEW_CACHE.get_or_init(|| Mutex::new(StreamViewCache::default()));
    let mut cache = cache
        .lock()
        .map_err(|_| "Stream Viewer cache was poisoned.".to_string())?;
    let key = StreamViewCacheKey {
        document: document_key,
        reference,
    };
    lru_put(&mut cache.entries, key, view, STREAM_VIEW_CACHE_LIMIT);
    Ok(())
}

fn load_stream_preview_cache(
    document_key: &LazyDocumentCacheKey,
    reference: ObjectRef,
    mode: StreamPreviewMode,
) -> Result<Option<GuiStreamPreview>, String> {
    let cache = STREAM_PREVIEW_CACHE.get_or_init(|| Mutex::new(StreamPreviewCache::default()));
    let mut cache = cache
        .lock()
        .map_err(|_| "Stream preview cache was poisoned.".to_string())?;
    let key = StreamPreviewCacheKey {
        document: document_key.clone(),
        reference,
        mode,
    };
    Ok(lru_get(&mut cache.entries, &key))
}

fn store_stream_preview_cache(
    document_key: LazyDocumentCacheKey,
    reference: ObjectRef,
    mode: StreamPreviewMode,
    preview: GuiStreamPreview,
) -> Result<(), String> {
    let cache = STREAM_PREVIEW_CACHE.get_or_init(|| Mutex::new(StreamPreviewCache::default()));
    let mut cache = cache
        .lock()
        .map_err(|_| "Stream preview cache was poisoned.".to_string())?;
    let key = StreamPreviewCacheKey {
        document: document_key,
        reference,
        mode,
    };
    lru_put(&mut cache.entries, key, preview, STREAM_PREVIEW_CACHE_LIMIT);
    Ok(())
}

fn load_page_preview_cache(
    document_key: &LazyDocumentCacheKey,
    page_number: u32,
    zoom: f32,
) -> Result<Option<GuiPagePreview>, String> {
    let cache = PAGE_PREVIEW_CACHE.get_or_init(|| Mutex::new(PagePreviewCache::default()));
    let mut cache = cache
        .lock()
        .map_err(|_| "Page Preview cache was poisoned.".to_string())?;
    let key = PagePreviewCacheKey {
        document: document_key.clone(),
        page_number,
        zoom_millis: zoom_cache_key(zoom),
    };
    if let Some(preview) = lru_get(&mut cache.entries, &key) {
        if Path::new(&preview.path).is_file() {
            return Ok(Some(preview));
        }
    }
    Ok(None)
}

fn store_page_preview_cache(
    document_key: LazyDocumentCacheKey,
    page_number: u32,
    zoom: f32,
    preview: GuiPagePreview,
) -> Result<(), String> {
    let cache = PAGE_PREVIEW_CACHE.get_or_init(|| Mutex::new(PagePreviewCache::default()));
    let mut cache = cache
        .lock()
        .map_err(|_| "Page Preview cache was poisoned.".to_string())?;
    let key = PagePreviewCacheKey {
        document: document_key,
        page_number,
        zoom_millis: zoom_cache_key(zoom),
    };
    lru_put(&mut cache.entries, key, preview, PAGE_PREVIEW_CACHE_LIMIT);
    Ok(())
}

fn invalidate_backend_caches_for_path(path: &Path) {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if let Some(cache) = FULL_DOCUMENT_CACHE.get() {
        if let Ok(mut cache) = cache.lock() {
            cache.entries.retain(|(key, _)| key.path != canonical);
        }
    }
    if let Some(cache) = STREAM_VIEW_CACHE.get() {
        if let Ok(mut cache) = cache.lock() {
            cache
                .entries
                .retain(|(key, _)| key.document.path != canonical);
        }
    }
    if let Some(cache) = STREAM_PREVIEW_CACHE.get() {
        if let Ok(mut cache) = cache.lock() {
            cache
                .entries
                .retain(|(key, _)| key.document.path != canonical);
        }
    }
    if let Some(cache) = PAGE_PREVIEW_CACHE.get() {
        if let Ok(mut cache) = cache.lock() {
            cache
                .entries
                .retain(|(key, _)| key.document.path != canonical);
        }
    }
}

fn lru_get<K, V>(entries: &mut VecDeque<(K, V)>, key: &K) -> Option<V>
where
    K: Eq,
    V: Clone,
{
    let index = entries
        .iter()
        .position(|(cached_key, _)| cached_key == key)?;
    let (key, value) = entries.remove(index)?;
    let output = value.clone();
    entries.push_back((key, value));
    Some(output)
}

fn lru_put<K, V>(entries: &mut VecDeque<(K, V)>, key: K, value: V, limit: usize)
where
    K: Eq,
{
    if let Some(index) = entries
        .iter()
        .position(|(cached_key, _)| cached_key == &key)
    {
        entries.remove(index);
    }
    entries.push_back((key, value));
    while entries.len() > limit {
        entries.pop_front();
    }
}

fn prefetch_adjacent_page_previews(
    document_key: LazyDocumentCacheKey,
    path: PathBuf,
    page_number: u32,
    zoom: f32,
) {
    let pages = adjacent_page_numbers(page_number);
    if pages.is_empty() {
        return;
    }
    tauri::async_runtime::spawn_blocking(move || {
        let Ok(runtime) = bind_pdfium_runtime().map_err(|status| status.message) else {
            return;
        };
        for page_number in pages {
            let context = preview_log_context(page_number, zoom, None, None);
            if load_page_preview_cache(&document_key, page_number, zoom)
                .ok()
                .flatten()
                .is_some()
            {
                perf_log_detail(
                    "render_page_preview.prefetch_cache_hit",
                    &path,
                    Instant::now(),
                    &context,
                );
                continue;
            }
            let Ok(_render_guard) = PDFIUM_RENDER_LOCK
                .get_or_init(|| Mutex::new(()))
                .lock()
                .map_err(|_| "PDFium render lock was poisoned.".to_string())
            else {
                return;
            };
            let marker = PagePreviewRequestMarker {
                document: document_key.clone(),
                page_number,
                zoom_millis: zoom_cache_key(zoom),
                request_id: 0,
                open_generation: 0,
            };
            let Ok(preview) =
                render_page_preview_with_runtime(&runtime, &path, page_number, zoom, &marker)
            else {
                continue;
            };
            let cache_started = Instant::now();
            let _ = store_page_preview_cache(document_key.clone(), page_number, zoom, preview);
            perf_log_detail(
                "render_page_preview.prefetch_cache_store",
                &path,
                cache_started,
                &context,
            );
        }
    });
}

fn adjacent_page_numbers(page_number: u32) -> Vec<u32> {
    let mut pages = Vec::new();
    for delta in -LAZY_PAGE_METADATA_PREFETCH_RADIUS..=LAZY_PAGE_METADATA_PREFETCH_RADIUS {
        if delta == 0 {
            continue;
        }
        let candidate = i64::from(page_number) + i64::from(delta);
        if candidate > 0 && candidate <= i64::from(u32::MAX) {
            pages.push(candidate as u32);
        }
    }
    pages
}

fn zoom_cache_key(zoom: f32) -> u32 {
    (normalize_preview_zoom(Some(zoom)) * 1000.0).round() as u32
}

fn perf_logging_enabled() -> bool {
    std::env::var("PDF_DEBUGGER_PERF_LOG")
        .is_ok_and(|value| value == "1" || value.eq_ignore_ascii_case("true"))
}

fn perf_log(command: &str, path: &Path, started: Instant, cache_hit: Option<bool>) {
    if !perf_logging_enabled() {
        return;
    }
    let cache = match cache_hit {
        Some(true) => " cache=hit",
        Some(false) => " cache=miss",
        None => "",
    };
    eprintln!(
        "[pdf-debugger perf] command={} path={}{} elapsed_ms={}",
        command,
        path.display(),
        cache,
        started.elapsed().as_millis()
    );
}

fn preview_log_context(
    page_number: u32,
    zoom: f32,
    request_id: Option<u64>,
    open_generation: Option<u64>,
) -> String {
    format!(
        "page={} zoom_millis={} request_id={} open_generation={}",
        page_number,
        zoom_cache_key(zoom),
        request_id.unwrap_or(0),
        open_generation.unwrap_or(0)
    )
}

fn perf_log_detail(command: &str, path: &Path, started: Instant, context: &str) {
    if !perf_logging_enabled() {
        return;
    }
    eprintln!(
        "[pdf-debugger perf] command={} path={} {} elapsed_ms={}",
        command,
        path.display(),
        context,
        started.elapsed().as_millis()
    );
}

fn gui_trailer_view(trailer: Option<&PdfValue>) -> GuiTrailerView {
    let Some(trailer) = trailer else {
        return GuiTrailerView {
            entries: Vec::new(),
            nodes: Vec::new(),
            warnings: vec!["No trailer dictionary was discovered.".to_string()],
        };
    };

    let Some(dictionary) = trailer.as_dictionary() else {
        return GuiTrailerView {
            entries: Vec::new(),
            nodes: Vec::new(),
            warnings: vec![format!(
                "Trailer metadata was parsed as {}, not a dictionary.",
                pdf_value_type_label(trailer)
            )],
        };
    };

    GuiTrailerView {
        entries: dictionary
            .iter()
            .map(|(key, value)| GuiTrailerEntry {
                key: format!("/{key}"),
                value_type: pdf_value_type_label(value).to_string(),
                value: value.summary(),
                reference: value.as_reference(),
            })
            .collect(),
        nodes: dictionary
            .iter()
            .map(|(key, value)| gui_trailer_value_node(format!("/{key}"), value))
            .collect(),
        warnings: Vec::new(),
    }
}

fn gui_trailer_object_node(object: &PdfObject) -> GuiTrailerNode {
    let (value_type, value, children) = if let Some(stream) = object.stream.as_ref() {
        (
            "stream".to_string(),
            object.value.summary(),
            stream
                .dictionary
                .iter()
                .map(|(key, value)| gui_trailer_value_node(format!("/{key}"), value))
                .collect(),
        )
    } else {
        (
            pdf_value_type_label(&object.value).to_string(),
            object.value.summary(),
            gui_trailer_value_children(&object.value),
        )
    };

    GuiTrailerNode {
        kind: "object".to_string(),
        key: format!(
            "{} {} obj",
            object.reference.object, object.reference.generation
        ),
        value_type,
        value,
        reference: Some(object.reference),
        expandable: !children.is_empty(),
        children,
        stream: object.stream.is_some(),
    }
}

fn gui_trailer_value_node(key: String, value: &PdfValue) -> GuiTrailerNode {
    let children = gui_trailer_value_children(value);
    GuiTrailerNode {
        kind: "value".to_string(),
        key,
        value_type: pdf_value_type_label(value).to_string(),
        value: value.summary(),
        reference: value.as_reference(),
        expandable: value.as_reference().is_some() || !children.is_empty(),
        children,
        stream: false,
    }
}

fn gui_trailer_value_children(value: &PdfValue) -> Vec<GuiTrailerNode> {
    match value {
        PdfValue::Dictionary(dictionary) => dictionary
            .iter()
            .map(|(key, value)| gui_trailer_value_node(format!("/{key}"), value))
            .collect(),
        PdfValue::Array(values) => values
            .iter()
            .enumerate()
            .map(|(index, value)| gui_trailer_value_node(format!("[{index}]"), value))
            .collect(),
        _ => Vec::new(),
    }
}

fn pdf_value_type_label(value: &PdfValue) -> &'static str {
    match value {
        PdfValue::Null => "null",
        PdfValue::Boolean(_) => "boolean",
        PdfValue::Number(_) => "number",
        PdfValue::Name(_) => "name",
        PdfValue::String(_) => "string",
        PdfValue::HexString(_) => "hex string",
        PdfValue::Array(_) => "array",
        PdfValue::Dictionary(_) => "dictionary",
        PdfValue::Reference(_) => "indirect reference",
        PdfValue::Raw(_) => "raw",
    }
}

fn preview_output_path(pdf_path: &Path, page_number: u32) -> Result<PathBuf, String> {
    let directory = std::env::temp_dir().join("pdf-debugger-previews");
    fs::create_dir_all(&directory).map_err(|error| {
        format!(
            "Could not create preview directory {}: {error}",
            directory.display()
        )
    })?;
    let stem = pdf_path
        .file_stem()
        .map(|stem| sanitize_file_stem(&stem.to_string_lossy()))
        .filter(|stem| !stem.is_empty())
        .unwrap_or_else(|| "pdf".to_string());
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    Ok(directory.join(format!("{stem}-page-{page_number}-{nonce}.png")))
}

fn draft_preview_snapshot_path(pdf_path: &Path, revision: u64) -> Result<PathBuf, String> {
    let directory = std::env::temp_dir().join("pdf-debugger-draft-previews");
    fs::create_dir_all(&directory).map_err(|error| {
        format!(
            "Could not create draft preview directory {}: {error}",
            directory.display()
        )
    })?;
    let stem = pdf_path
        .file_stem()
        .map(|stem| sanitize_file_stem(&stem.to_string_lossy()))
        .filter(|stem| !stem.is_empty())
        .unwrap_or_else(|| "pdf".to_string());
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    Ok(directory.join(format!("{stem}-draft-r{revision}-{nonce}.pdf")))
}

fn sanitize_file_stem(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .collect()
}

fn same_existing_path(left: &Path, right: &Path) -> bool {
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

fn preview_raw_source(object: &PdfObject, path: &Path) -> String {
    let raw = if object.raw_bytes.is_empty() {
        let preview_len = object.raw_range.len().min(4096);
        read_file_range(path, object.raw_range.start, preview_len)
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
            .unwrap_or_default()
    } else {
        String::from_utf8_lossy(&object.raw_bytes).into_owned()
    };
    preview_raw_text(raw, object.raw_range)
}

fn preview_raw_text(raw: String, range: ByteRange) -> String {
    let preview_len = range.len().min(4096);
    let mut chars = raw.chars().take(preview_len);
    let mut preview = chars.by_ref().take(800).collect::<String>();
    if chars.next().is_some() {
        preview.push_str("\n...");
    }
    if preview.trim().is_empty() {
        format!("raw bytes {}..{}", range.start, range.end)
    } else {
        preview
    }
}

fn read_file_range(path: &Path, start: usize, length: usize) -> std::io::Result<Vec<u8>> {
    let mut file = fs::File::open(path)?;
    file.seek(SeekFrom::Start(start as u64))?;
    let mut bytes = vec![0; length];
    let read = file.read(&mut bytes)?;
    bytes.truncate(read);
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EnvGuard {
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: String) -> Self {
            let previous = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, previous }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            if let Some(previous) = &self.previous {
                std::env::set_var(self.key, previous);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }

    #[test]
    fn invalid_pdfium_env_override_does_not_fall_back() {
        let missing_library = std::env::temp_dir()
            .join(format!(
                "pdf-debugger-missing-pdfium-{}",
                std::process::id()
            ))
            .join(Pdfium::pdfium_platform_library_name());
        let _guard = EnvGuard::set(pdfium_env_var(), missing_library.display().to_string());

        let status = bind_pdfium_runtime().expect_err("invalid env override must fail");

        assert!(!status.available);
        assert_eq!(status.source_kind, "unavailable");
        assert!(
            status.message.contains("explicit override"),
            "message should explain that the env var is authoritative: {}",
            status.message
        );
        assert!(
            status
                .attempted_sources
                .iter()
                .all(|source| !source.starts_with("downloaded:")),
            "invalid env override should not fall back to downloaded runtime: {:?}",
            status.attempted_sources
        );
    }

    #[test]
    fn render_page_preview_writes_png_when_pdfium_is_available() {
        if let Err(status) = bind_pdfium_runtime() {
            eprintln!("skipping PDFium render smoke test: {}", status.message);
            return;
        }

        let directory =
            std::env::temp_dir().join(format!("pdf-debugger-render-smoke-{}", std::process::id()));
        fs::create_dir_all(&directory).expect("create render smoke directory");
        let pdf_path = directory.join("render-smoke.pdf");
        fs::write(&pdf_path, minimal_renderable_pdf()).expect("write render smoke PDF");

        let preview =
            render_page_preview_sync(pdf_path.display().to_string(), 1, Some(1.5), None, None)
                .expect("render page preview");
        let output_path = PathBuf::from(preview.path);

        assert!(output_path.is_file(), "preview PNG should exist");
        let png = fs::read(&output_path).expect("read preview PNG");
        assert!(
            png.starts_with(b"\x89PNG\r\n\x1a\n"),
            "preview output should be a PNG"
        );
        assert!(preview.pixel_width > 0);
        assert!(preview.pixel_height > 0);
        assert_eq!(preview.zoom, 1.5);

        let _ = fs::remove_dir_all(&directory);
        let _ = fs::remove_file(output_path);
    }

    #[test]
    fn speed_first_caches_page_preview_bitmaps_when_pdfium_is_available() {
        if let Err(status) = bind_pdfium_runtime() {
            eprintln!("skipping PDFium preview cache test: {}", status.message);
            return;
        }

        let directory =
            std::env::temp_dir().join(format!("pdf-debugger-preview-cache-{}", std::process::id()));
        fs::create_dir_all(&directory).expect("create preview cache directory");
        let pdf_path = directory.join("preview-cache.pdf");
        fs::write(&pdf_path, minimal_renderable_pdf()).expect("write render smoke PDF");
        let document_key = lazy_cache_key(&pdf_path).expect("cache key");

        let preview =
            render_page_preview_sync(pdf_path.display().to_string(), 1, Some(1.25), None, None)
                .expect("render page preview");
        let cached = load_page_preview_cache(&document_key, 1, 1.25)
            .expect("preview cache read")
            .expect("preview cache entry");
        assert_eq!(cached.path, preview.path);
        assert!(PathBuf::from(&cached.path).is_file());

        let cached_again =
            render_page_preview_sync(pdf_path.display().to_string(), 1, Some(1.25), None, None)
                .expect("render cached page preview");
        assert_eq!(cached_again.path, preview.path);

        let _ = fs::remove_dir_all(&directory);
        let _ = fs::remove_file(preview.path);
    }

    #[test]
    fn image_xobject_stream_does_not_run_content_operator_analysis() {
        let directory =
            std::env::temp_dir().join(format!("pdf-debugger-image-stream-{}", std::process::id()));
        fs::create_dir_all(&directory).expect("create image stream directory");
        let pdf_path = directory.join("image-stream.pdf");
        fs::write(&pdf_path, image_xobject_pdf()).expect("write image stream PDF");

        let metadata = view_stream_metadata_sync(pdf_path.display().to_string(), 4, 0)
            .expect("view image stream metadata");
        assert_eq!(metadata.reference, ObjectRef::new(4, 0));
        assert_eq!(metadata.filters, vec!["FlateDecode".to_string()]);
        let image = metadata.image.expect("image metadata");
        assert_eq!(image.width, Some(1));
        assert_eq!(image.height, Some(1));
        assert!(image.renderable);

        let preview = render_stream_image_preview_sync(pdf_path.display().to_string(), 4, 0)
            .expect("render image stream preview");
        assert_eq!(preview.reference, ObjectRef::new(4, 0));
        assert_eq!(preview.format, "png");
        assert_eq!(preview.width, 1);
        assert_eq!(preview.height, 1);
        let output_path = PathBuf::from(&preview.path);
        assert!(output_path.is_file(), "image stream preview should exist");
        let png = fs::read(&output_path).expect("read image stream preview");
        assert!(
            png.starts_with(b"\x89PNG\r\n\x1a\n"),
            "image stream preview should be a PNG"
        );

        let error = view_content_stream_sync(pdf_path.display().to_string(), 4, 0)
            .expect_err("image stream should not be analyzed as page content");
        assert!(error.contains("image XObject stream"));

        let _ = fs::remove_dir_all(&directory);
        let _ = fs::remove_file(output_path);
    }

    #[test]
    fn dct_image_xobject_renders_browser_friendly_png_preview() {
        let directory = std::env::temp_dir().join(format!(
            "pdf-debugger-dct-image-stream-{}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).expect("create DCT image stream directory");
        let pdf_path = directory.join("dct-image-stream.pdf");
        fs::write(&pdf_path, dct_image_xobject_pdf()).expect("write DCT image stream PDF");

        let metadata = view_stream_metadata_sync(pdf_path.display().to_string(), 4, 0)
            .expect("view DCT image stream metadata");
        assert_eq!(metadata.reference, ObjectRef::new(4, 0));
        assert_eq!(metadata.filters, vec!["DCTDecode".to_string()]);
        assert!(metadata.image.expect("image metadata").renderable);

        let preview = render_stream_image_preview_sync(pdf_path.display().to_string(), 4, 0)
            .expect("render DCT image stream preview");
        assert_eq!(preview.reference, ObjectRef::new(4, 0));
        assert_eq!(preview.format, "png");
        assert_eq!(preview.source, "decoded DCTDecode JPEG");
        assert_eq!(preview.width, 1);
        assert_eq!(preview.height, 1);
        let output_path = PathBuf::from(&preview.path);
        assert!(
            output_path.is_file(),
            "DCT image stream preview should exist"
        );
        let png = fs::read(&output_path).expect("read DCT image stream preview");
        assert!(
            png.starts_with(b"\x89PNG\r\n\x1a\n"),
            "DCT image stream preview should be a PNG"
        );

        let _ = fs::remove_dir_all(&directory);
        let _ = fs::remove_file(output_path);
    }

    #[test]
    fn page_objects_reuse_cached_document_and_hydrate_needed_streams() {
        let directory =
            std::env::temp_dir().join(format!("pdf-debugger-page-objects-{}", std::process::id()));
        fs::create_dir_all(&directory).expect("create page object directory");
        let pdf_path = directory.join("page-objects.pdf");
        fs::write(&pdf_path, minimal_renderable_pdf()).expect("write page object PDF");

        let _summary = open_pdf_sync(pdf_path.display().to_string()).expect("open PDF");
        let inspection = inspect_page_objects_sync(pdf_path.display().to_string(), 1)
            .expect("inspect page objects");

        assert!(
            inspection
                .objects
                .iter()
                .any(|object| object.summary.contains("Text run")),
            "Page Objects should decode the page content stream from cached GUI state"
        );

        let _ = fs::remove_dir_all(&directory);
    }

    #[test]
    fn load_acroform_lists_fields_sorted_by_name() {
        let directory =
            std::env::temp_dir().join(format!("pdf-debugger-acroform-{}", std::process::id()));
        fs::create_dir_all(&directory).expect("create acroform directory");
        let pdf_path = directory.join("acroform.pdf");
        fs::write(&pdf_path, acroform_pdf()).expect("write acroform PDF");

        let view = load_acroform_sync(pdf_path.display().to_string()).expect("load AcroForm");

        let names = view
            .fields
            .iter()
            .map(|field| field.name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["contact.email", "zName"]);
        assert_eq!(view.fields[0].reference, ObjectRef::new(9, 0));
        assert_eq!(view.fields[0].field_type.as_deref(), Some("Tx"));
        assert_eq!(view.fields[1].reference, ObjectRef::new(7, 0));
        assert!(
            view.warnings.is_empty(),
            "unexpected warnings: {:?}",
            view.warnings
        );

        let _ = fs::remove_dir_all(&directory);
    }

    #[test]
    fn open_pdf_summarizes_page_annotations() {
        let directory =
            std::env::temp_dir().join(format!("pdf-debugger-annots-{}", std::process::id()));
        fs::create_dir_all(&directory).expect("create annotation directory");
        let pdf_path = directory.join("annots.pdf");
        fs::write(&pdf_path, annotation_pdf()).expect("write annotation PDF");

        let summary = open_pdf_sync(pdf_path.display().to_string()).expect("open annotation PDF");

        assert_eq!(summary.annotations.annotations.len(), 1);
        let annotation = &summary.annotations.annotations[0];
        assert_eq!(annotation.page_number, 1);
        assert_eq!(annotation.page_reference, ObjectRef::new(3, 0));
        assert_eq!(annotation.reference, ObjectRef::new(4, 0));
        assert_eq!(annotation.subtype.as_deref(), Some("/Link"));
        assert_eq!(annotation.rect.as_deref(), Some("[4 item(s)]"));
        assert_eq!(annotation.flags.as_deref(), Some("4"));
        assert_eq!(annotation.contents.as_deref(), Some("fixture link"));
        assert_eq!(annotation.color.as_deref(), Some("[3 item(s)]"));
        assert!(annotation.keys.iter().any(|key| key == "/A"));
        assert!(
            summary.annotations.warnings.is_empty(),
            "unexpected warnings: {:?}",
            summary.annotations.warnings
        );

        let _ = fs::remove_dir_all(&directory);
    }

    #[test]
    fn load_acroform_requires_full_load_for_large_pdf() {
        let directory =
            std::env::temp_dir().join(format!("pdf-debugger-full-acroform-{}", std::process::id()));
        fs::create_dir_all(&directory).expect("create full-load AcroForm directory");
        let pdf_path = directory.join("large-acroform.pdf");
        write_sparse_large_acroform_pdf(&pdf_path).expect("write sparse large AcroForm PDF");

        let open_error =
            open_pdf_sync(pdf_path.display().to_string()).expect_err("large GUI open should fail");
        assert!(open_error.contains("fully load"));

        let acroform_error = load_acroform_sync(pdf_path.display().to_string())
            .expect_err("large AcroForm loading should require full load");
        assert!(acroform_error.contains("fully load"));

        let _ = fs::remove_dir_all(&directory);
    }

    #[test]
    fn open_pdf_discovers_xref_stream_object_stream_pages_and_acroform() {
        let directory = std::env::temp_dir().join(format!(
            "pdf-debugger-xref-objstm-gui-{}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).expect("create xref object stream directory");
        let pdf_path = directory.join("xref-object-stream-acroform.pdf");
        fs::write(&pdf_path, xref_stream_object_stream_acroform_pdf())
            .expect("write xref stream object stream PDF");

        let summary = open_pdf_sync(pdf_path.display().to_string())
            .expect("open xref stream object stream PDF");
        assert_eq!(summary.open_mode, "full");
        assert_eq!(summary.metadata.root, Some(ObjectRef::new(1, 0)));
        assert_eq!(summary.metadata.page_count, Some(1));
        assert!(summary.metadata.has_xref_stream);
        assert!(summary.metadata.has_object_stream);
        assert!(summary.trailer.nodes.iter().any(|node| node.key == "/Root"));
        assert_eq!(summary.page_index.pages.len(), 1);

        let view = load_acroform_sync(pdf_path.display().to_string())
            .expect("load AcroForm from object stream");
        assert_eq!(view.fields.len(), 1);
        assert_eq!(view.fields[0].name, "Name");
        assert_eq!(view.fields[0].reference, ObjectRef::new(13, 0));

        let _ = fs::remove_dir_all(&directory);
    }

    #[test]
    fn open_pdf_requires_full_load_for_large_classic_xref_pdf() {
        let directory =
            std::env::temp_dir().join(format!("pdf-debugger-large-open-{}", std::process::id()));
        fs::create_dir_all(&directory).expect("create large open directory");
        let pdf_path = directory.join("large-lazy.pdf");
        write_sparse_large_classic_xref_pdf(&pdf_path).expect("write sparse large PDF");

        let open_error =
            open_pdf_sync(pdf_path.display().to_string()).expect_err("large GUI open should fail");
        assert!(open_error.contains("fully load"));

        let object_error = inspect_object_sync(pdf_path.display().to_string(), 1, 0)
            .expect_err("large object inspection should require full load");
        assert!(object_error.contains("fully load"));

        let stream_error = view_stream_sync(pdf_path.display().to_string(), 4, 0)
            .expect_err("large stream view should require full load");
        assert!(stream_error.contains("fully load"));

        let _ = fs::remove_dir_all(&directory);
    }

    #[test]
    fn gui_open_full_parses_medium_classic_xref_pdf() {
        let directory = std::env::temp_dir().join(format!(
            "pdf-debugger-medium-full-open-{}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).expect("create medium full-open directory");
        let pdf_path = directory.join("medium-full-open.pdf");
        fs::write(
            &pdf_path,
            padded_classic_xref_pdf(GUI_MEDIUM_FULL_LOAD_TEST_SIZE),
        )
        .expect("write medium PDF");

        let summary = open_pdf_sync(pdf_path.display().to_string()).expect("open medium PDF");

        assert_eq!(summary.open_mode, "full");
        assert_eq!(summary.metadata.root, Some(ObjectRef::new(1, 0)));
        assert_eq!(summary.metadata.page_count, Some(1));
        assert_eq!(summary.page_index.pages.len(), 1);
        assert!(summary.capability_warnings.is_empty());

        let _ = fs::remove_dir_all(&directory);
    }

    #[test]
    #[ignore]
    fn bench_open_target_pdf_from_env() {
        let path = std::env::var("PDF_DEBUGGER_BENCH_PDF")
            .expect("set PDF_DEBUGGER_BENCH_PDF to the target PDF path");
        let path = PathBuf::from(path);
        let metadata = fs::metadata(&path).expect("read benchmark PDF metadata");
        let iterations = std::env::var("PDF_DEBUGGER_BENCH_ITERS")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(1)
            .max(1);

        let mut cold_elapsed_values = Vec::with_capacity(iterations);
        let mut cold = None;
        for _ in 0..iterations {
            invalidate_backend_caches_for_path(&path);
            let cold_started = Instant::now();
            cold =
                Some(open_pdf_sync(path.display().to_string()).expect("cold open benchmark PDF"));
            cold_elapsed_values.push(cold_started.elapsed());
        }
        let cold = cold.expect("benchmark should run at least one cold iteration");

        let warm_started = Instant::now();
        let warm = open_pdf_sync(path.display().to_string()).expect("warm open benchmark PDF");
        let warm_elapsed = warm_started.elapsed();
        let cold_min_ms = cold_elapsed_values
            .iter()
            .map(|elapsed| elapsed.as_millis())
            .min()
            .unwrap_or_default();
        let cold_max_ms = cold_elapsed_values
            .iter()
            .map(|elapsed| elapsed.as_millis())
            .max()
            .unwrap_or_default();
        let cold_avg_ms = cold_elapsed_values
            .iter()
            .map(|elapsed| elapsed.as_millis())
            .sum::<u128>()
            / cold_elapsed_values.len() as u128;

        eprintln!(
            "[pdf-debugger bench] path={} size_bytes={} pages={} objects={} streams={} cold_open_ms={} warm_open_ms={} cold_iters={} cold_min_ms={} cold_avg_ms={} cold_max_ms={}",
            path.display(),
            metadata.len(),
            cold.metadata.page_count.unwrap_or_default(),
            cold.metadata.object_count,
            cold.metadata.stream_count,
            cold_elapsed_values
                .first()
                .map(|elapsed| elapsed.as_millis())
                .unwrap_or_default(),
            warm_elapsed.as_millis(),
            iterations,
            cold_min_ms,
            cold_avg_ms,
            cold_max_ms
        );

        assert_eq!(cold.open_mode, "full");
        assert_eq!(warm.open_mode, "full");
    }

    fn minimal_renderable_pdf() -> Vec<u8> {
        let mut bytes = b"%PDF-1.4\n".to_vec();
        let offsets = [
            push_object(
                &mut bytes,
                b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n",
            ),
            push_object(
                &mut bytes,
                b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 200 200] >>\nendobj\n",
            ),
            push_object(
                &mut bytes,
                b"3 0 obj\n<< /Type /Page /Parent 2 0 R /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>\nendobj\n",
            ),
            push_object(
                &mut bytes,
                b"4 0 obj\n<< /Length 23 >>\nstream\nBT /F1 12 Tf (Hi) Tj ET\nendstream\nendobj\n",
            ),
            push_object(
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

    fn padded_classic_xref_pdf(target_size: usize) -> Vec<u8> {
        let mut bytes = b"%PDF-1.4\n".to_vec();
        let offsets = [
            push_object(
                &mut bytes,
                b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n",
            ),
            push_object(
                &mut bytes,
                b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 200 200] >>\nendobj\n",
            ),
            push_object(
                &mut bytes,
                b"3 0 obj\n<< /Type /Page /Parent 2 0 R /Resources << >> >>\nendobj\n",
            ),
        ];
        if bytes.len() < target_size {
            bytes.extend(std::iter::repeat(b'%').take(target_size - bytes.len()));
            bytes.push(b'\n');
        }
        let xref = bytes.len();
        bytes.extend_from_slice(b"xref\n0 4\n0000000000 65535 f \n");
        for offset in offsets {
            bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        bytes.extend_from_slice(
            format!("trailer\n<< /Size 4 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes(),
        );
        bytes
    }

    fn image_xobject_pdf() -> Vec<u8> {
        let mut bytes = b"%PDF-1.4\n".to_vec();
        let image_bytes = [
            0x78, 0x01, 0x63, 0x60, 0x60, 0x60, 0x00, 0x00, 0x00, 0x04, 0x00, 0x01,
        ];
        let mut image_object = Vec::new();
        image_object.extend_from_slice(
            format!(
                "4 0 obj\n<< /Type /XObject /Subtype /Image /Width 1 /Height 1 /ColorSpace /DeviceGray /BitsPerComponent 8 /Filter /FlateDecode /Length {} >>\nstream\n",
                image_bytes.len()
            )
            .as_bytes(),
        );
        image_object.extend_from_slice(&image_bytes);
        image_object.extend_from_slice(b"\nendstream\nendobj\n");

        let offsets = [
            push_object(
                &mut bytes,
                b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n",
            ),
            push_object(
                &mut bytes,
                b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 200 200] >>\nendobj\n",
            ),
            push_object(
                &mut bytes,
                b"3 0 obj\n<< /Type /Page /Parent 2 0 R /Resources << /XObject << /Im1 4 0 R >> >> /Contents 5 0 R >>\nendobj\n",
            ),
            push_object(&mut bytes, &image_object),
            push_object(
                &mut bytes,
                b"5 0 obj\n<< /Length 8 >>\nstream\n/Im1 Do\nendstream\nendobj\n",
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

    fn dct_image_xobject_pdf() -> Vec<u8> {
        let mut bytes = b"%PDF-1.4\n".to_vec();
        let image_bytes = tiny_one_pixel_jpeg();
        let mut image_object = Vec::new();
        image_object.extend_from_slice(
            format!(
                "4 0 obj\n<< /Type /XObject /Subtype /Image /Width 1 /Height 1 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length {} >>\nstream\n",
                image_bytes.len()
            )
            .as_bytes(),
        );
        image_object.extend_from_slice(&image_bytes);
        image_object.extend_from_slice(b"\nendstream\nendobj\n");

        let offsets = [
            push_object(
                &mut bytes,
                b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n",
            ),
            push_object(
                &mut bytes,
                b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 200 200] >>\nendobj\n",
            ),
            push_object(
                &mut bytes,
                b"3 0 obj\n<< /Type /Page /Parent 2 0 R /Resources << /XObject << /Im1 4 0 R >> >> /Contents 5 0 R >>\nendobj\n",
            ),
            push_object(&mut bytes, &image_object),
            push_object(
                &mut bytes,
                b"5 0 obj\n<< /Length 8 >>\nstream\n/Im1 Do\nendstream\nendobj\n",
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

    fn tiny_one_pixel_jpeg() -> Vec<u8> {
        let image = ImageBuffer::<Rgb<u8>, Vec<u8>>::from_raw(1, 1, vec![255, 0, 0])
            .expect("build tiny JPEG source image");
        let mut bytes = Vec::new();
        let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, 90);
        encoder
            .encode_image(&image)
            .expect("encode tiny JPEG source image");
        bytes
    }

    fn acroform_pdf() -> Vec<u8> {
        let mut bytes = b"%PDF-1.4\n".to_vec();
        let offsets = [
            push_object(
                &mut bytes,
                b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R /AcroForm 6 0 R >>\nendobj\n",
            ),
            push_object(
                &mut bytes,
                b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 200 200] >>\nendobj\n",
            ),
            push_object(
                &mut bytes,
                b"3 0 obj\n<< /Type /Page /Parent 2 0 R >>\nendobj\n",
            ),
            push_object(
                &mut bytes,
                b"4 0 obj\n<< /Producer (fixture) >>\nendobj\n",
            ),
            push_object(
                &mut bytes,
                b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n",
            ),
            push_object(
                &mut bytes,
                b"6 0 obj\n<< /Fields [7 0 R 8 0 R] >>\nendobj\n",
            ),
            push_object(
                &mut bytes,
                b"7 0 obj\n<< /FT /Tx /T (zName) >>\nendobj\n",
            ),
            push_object(
                &mut bytes,
                b"8 0 obj\n<< /T (contact) /Kids [9 0 R] >>\nendobj\n",
            ),
            push_object(
                &mut bytes,
                b"9 0 obj\n<< /FT /Tx /T (email) >>\nendobj\n",
            ),
        ];
        let xref = bytes.len();
        bytes.extend_from_slice(b"xref\n0 10\n0000000000 65535 f \n");
        for offset in offsets {
            bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        bytes.extend_from_slice(
            format!("trailer\n<< /Size 10 /Root 1 0 R /Info 4 0 R >>\nstartxref\n{xref}\n%%EOF\n")
                .as_bytes(),
        );
        bytes
    }

    fn annotation_pdf() -> Vec<u8> {
        let mut bytes = b"%PDF-1.4\n".to_vec();
        let offsets = [
            push_object(
                &mut bytes,
                b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n",
            ),
            push_object(
                &mut bytes,
                b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 200 200] >>\nendobj\n",
            ),
            push_object(
                &mut bytes,
                b"3 0 obj\n<< /Type /Page /Parent 2 0 R /Annots [4 0 R] >>\nendobj\n",
            ),
            push_object(
                &mut bytes,
                b"4 0 obj\n<< /Type /Annot /Subtype /Link /Rect [10 20 90 40] /F 4 /Contents (fixture link) /C [1 0 0] /Border [0 0 1] /CA 0.5 /A << /S /URI /URI (https://example.invalid) >> >>\nendobj\n",
            ),
        ];
        let xref = bytes.len();
        bytes.extend_from_slice(b"xref\n0 5\n0000000000 65535 f \n");
        for offset in offsets {
            bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        bytes.extend_from_slice(
            format!("trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes(),
        );
        bytes
    }

    fn xref_stream_object_stream_acroform_pdf() -> Vec<u8> {
        let mut bytes = b"%PDF-1.7\n".to_vec();
        let object_1 = push_object(
            &mut bytes,
            b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R /AcroForm 7 0 R >>\nendobj\n",
        );
        let object_3 = push_object(&mut bytes, b"3 0 obj\n<< /Producer (fixture) >>\nendobj\n");
        let body_2 = b"<< /Type /Pages /Kids [4 0 R] /Count 1 /MediaBox [0 0 200 200] >>";
        let body_4 = b"<< /Type /Page /Parent 2 0 R >>";
        let body_7 = b"<< /Fields [13 0 R] /DA (/F1 0 Tf 0 g) >>";
        let body_13 = b"<< /FT /Tx /T (Name) >>";
        let offset_4 = body_2.len() + 1;
        let offset_7 = offset_4 + body_4.len() + 1;
        let offset_13 = offset_7 + body_7.len() + 1;
        let table = format!("2 0 4 {offset_4} 7 {offset_7} 13 {offset_13} ");
        let first = table.len();
        let mut compressed = table.into_bytes();
        compressed.extend_from_slice(body_2);
        compressed.push(b' ');
        compressed.extend_from_slice(body_4);
        compressed.push(b' ');
        compressed.extend_from_slice(body_7);
        compressed.push(b' ');
        compressed.extend_from_slice(body_13);
        let object_19 = push_object(
            &mut bytes,
            format!(
                "19 0 obj\n<< /Type /ObjStm /N 4 /First {first} /Length {} >>\nstream\n",
                compressed.len()
            )
            .as_bytes(),
        );
        bytes.extend_from_slice(&compressed);
        bytes.extend_from_slice(b"\nendstream\nendobj\n");

        let xref_offset = bytes.len();
        let mut xref_data = Vec::new();
        push_xref_stream_entry(&mut xref_data, 0, 0, 65535);
        push_xref_stream_entry(&mut xref_data, 1, object_1, 0);
        push_xref_stream_entry(&mut xref_data, 2, 19, 0);
        push_xref_stream_entry(&mut xref_data, 1, object_3, 0);
        push_xref_stream_entry(&mut xref_data, 2, 19, 1);
        for _ in 5..7 {
            push_xref_stream_entry(&mut xref_data, 0, 0, 0);
        }
        push_xref_stream_entry(&mut xref_data, 2, 19, 2);
        for _ in 8..13 {
            push_xref_stream_entry(&mut xref_data, 0, 0, 0);
        }
        push_xref_stream_entry(&mut xref_data, 2, 19, 3);
        for _ in 14..19 {
            push_xref_stream_entry(&mut xref_data, 0, 0, 0);
        }
        push_xref_stream_entry(&mut xref_data, 1, object_19, 0);
        push_xref_stream_entry(&mut xref_data, 1, xref_offset, 0);
        bytes.extend_from_slice(
            format!(
                "20 0 obj\n<< /Type /XRef /Size 21 /Root 1 0 R /Info 3 0 R /W [1 4 2] /Length {} >>\nstream\n",
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

    fn write_sparse_large_classic_xref_pdf(path: &Path) -> std::io::Result<()> {
        let mut bytes = b"%PDF-1.4\n".to_vec();
        let offsets = [
            push_object(
                &mut bytes,
                b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n",
            ),
            push_object(
                &mut bytes,
                b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 200 300] /Rotate 90 /Resources << /Font << /F1 5 0 R >> >> >>\nendobj\n",
            ),
            push_object(
                &mut bytes,
                b"3 0 obj\n<< /Type /Page /Parent 2 0 R /Contents 4 0 R >>\nendobj\n",
            ),
            push_object(
                &mut bytes,
                b"4 0 obj\n<< /Length 23 >>\nstream\nBT /F1 12 Tf (Hi) Tj ET\nendstream\nendobj\n",
            ),
            push_object(
                &mut bytes,
                b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n",
            ),
        ];

        let mut file = std::fs::File::create(path)?;
        use std::io::{Seek, SeekFrom, Write};
        file.write_all(&bytes)?;
        let xref_offset = pdf_debugger::pdf_parser::MAX_FULL_PARSE_FILE_SIZE + 4096;
        file.seek(SeekFrom::Start(xref_offset))?;
        file.write_all(b"xref\n0 6\n0000000000 65535 f \n")?;
        for offset in offsets {
            file.write_all(format!("{offset:010} 00000 n \n").as_bytes())?;
        }
        file.write_all(
            format!("trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n")
                .as_bytes(),
        )?;
        Ok(())
    }

    fn write_sparse_large_acroform_pdf(path: &Path) -> std::io::Result<()> {
        let mut bytes = b"%PDF-1.4\n".to_vec();
        let offsets = [
            push_object(
                &mut bytes,
                b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R /AcroForm 6 0 R >>\nendobj\n",
            ),
            push_object(
                &mut bytes,
                b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 200 200] >>\nendobj\n",
            ),
            push_object(
                &mut bytes,
                b"3 0 obj\n<< /Type /Page /Parent 2 0 R >>\nendobj\n",
            ),
            push_object(
                &mut bytes,
                b"4 0 obj\n<< /Producer (fixture) >>\nendobj\n",
            ),
            push_object(
                &mut bytes,
                b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n",
            ),
            push_object(
                &mut bytes,
                b"6 0 obj\n<< /Fields [7 0 R 8 0 R] >>\nendobj\n",
            ),
            push_object(
                &mut bytes,
                b"7 0 obj\n<< /FT /Tx /T (zName) >>\nendobj\n",
            ),
            push_object(
                &mut bytes,
                b"8 0 obj\n<< /T (contact) /Kids [9 0 R] >>\nendobj\n",
            ),
            push_object(
                &mut bytes,
                b"9 0 obj\n<< /FT /Tx /T (email) >>\nendobj\n",
            ),
        ];

        let mut file = std::fs::File::create(path)?;
        use std::io::{Seek, SeekFrom, Write};
        file.write_all(&bytes)?;
        let xref_offset = pdf_debugger::pdf_parser::MAX_FULL_PARSE_FILE_SIZE + 4096;
        file.seek(SeekFrom::Start(xref_offset))?;
        file.write_all(b"xref\n0 10\n0000000000 65535 f \n")?;
        for offset in offsets {
            file.write_all(format!("{offset:010} 00000 n \n").as_bytes())?;
        }
        file.write_all(
            format!("trailer\n<< /Size 10 /Root 1 0 R /Info 4 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n")
                .as_bytes(),
        )?;
        Ok(())
    }

    fn push_object(bytes: &mut Vec<u8>, object: &[u8]) -> usize {
        let offset = bytes.len();
        bytes.extend_from_slice(object);
        offset
    }

    fn push_xref_stream_entry(bytes: &mut Vec<u8>, entry_type: u8, field_2: usize, field_3: usize) {
        bytes.push(entry_type);
        bytes.extend_from_slice(&(field_2 as u32).to_be_bytes());
        bytes.extend_from_slice(&(field_3 as u16).to_be_bytes());
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                apply_native_window_corners(&window);
                let window_for_events = window.clone();
                window.on_window_event(move |event| match event {
                    tauri::WindowEvent::Resized(_)
                    | tauri::WindowEvent::Focused(_)
                    | tauri::WindowEvent::ThemeChanged(_) => {
                        apply_native_window_corners(&window_for_events);
                    }
                    _ => {}
                });
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            open_pdf,
            renderer_status,
            inspect_object,
            load_trailer_object,
            load_acroform,
            view_stream_metadata,
            view_stream_preview,
            view_stream,
            render_stream_image_preview,
            view_content_stream,
            inspect_page_objects,
            validate_edit_value,
            save_modified_pdf_as,
            save_modified_pdf_in_place,
            create_draft_preview_snapshot,
            export_stream,
            render_page_preview
        ])
        .run(tauri::generate_context!())
        .expect("failed to run PDF Debugger desktop app");
}
