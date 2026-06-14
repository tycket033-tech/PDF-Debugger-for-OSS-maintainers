use crate::lazy_pdf::{build_lazy_page_index, LazyPdfDocument, LazyXrefEntry};
use crate::page_index::{PageIndex, PageObjectLinkKind};
use crate::pdf_model::{DiagnosticSummary, Finding, ObjectRef, PdfStream, PdfValue, Severity};
use serde::Serialize;
use std::collections::{BTreeSet, HashMap, HashSet};

const LAZY_FAST_STREAM_METADATA_LIMIT: usize = 256;

#[derive(Clone, Debug, Serialize)]
pub struct LazyDiagnosticsReport {
    pub diagnostics: DiagnosticSummary,
    pub findings: Vec<Finding>,
}

pub fn run_lazy_fast_diagnostics(document: &mut LazyPdfDocument) -> Vec<Finding> {
    let page_index = build_lazy_page_index(document);
    run_lazy_fast_diagnostics_with_page_index(document, &page_index)
}

pub fn build_lazy_diagnostics_report(document: &mut LazyPdfDocument) -> LazyDiagnosticsReport {
    let findings = run_lazy_fast_diagnostics(document);
    LazyDiagnosticsReport {
        diagnostics: DiagnosticSummary::from_findings(&findings),
        findings,
    }
}

pub fn run_lazy_fast_diagnostics_with_page_index(
    document: &LazyPdfDocument,
    page_index: &PageIndex,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut seen = HashSet::new();
    let mut checked_references = HashSet::new();
    let mut checked_streams = HashSet::new();
    let xref_lookup = document
        .xref
        .entries
        .iter()
        .map(|entry| (entry.reference, entry))
        .collect::<HashMap<_, _>>();
    let mut context = LazyFastDiagnosticContext {
        xref_lookup,
        stream_metadata_limit_reported: false,
    };

    add_lazy_warnings(document, &mut findings, &mut seen);
    add_document_findings(
        document,
        &context,
        &mut findings,
        &mut seen,
        &mut checked_references,
    );
    add_page_findings(
        document,
        &mut context,
        page_index,
        &mut findings,
        &mut seen,
        &mut checked_references,
        &mut checked_streams,
    );
    add_deep_deferred_finding(document, &mut findings, &mut seen);

    findings
}

struct LazyFastDiagnosticContext<'a> {
    xref_lookup: HashMap<ObjectRef, &'a LazyXrefEntry>,
    stream_metadata_limit_reported: bool,
}

fn add_lazy_warnings(
    document: &LazyPdfDocument,
    findings: &mut Vec<Finding>,
    seen: &mut HashSet<String>,
) {
    for warning in &document.warnings {
        push_once(
            findings,
            seen,
            warning.rule_id.clone(),
            Severity::Warning,
            warning.message.clone(),
            None,
            None,
            warning.byte_offset,
            "Continue with lazy inspection; if this affects a branch, inspect the referenced object by xref offset.".to_string(),
        );
    }
}

fn add_document_findings(
    document: &LazyPdfDocument,
    context: &LazyFastDiagnosticContext<'_>,
    findings: &mut Vec<Finding>,
    seen: &mut HashSet<String>,
    checked_references: &mut HashSet<ObjectRef>,
) {
    if document.trailer.is_none() {
        push_once(
            findings,
            seen,
            "lazy.trailer_missing".to_string(),
            Severity::Error,
            "Lazy open did not find a trailer dictionary".to_string(),
            None,
            None,
            document.xref.startxref,
            "Verify startxref and the classic xref trailer section".to_string(),
        );
    }

    if document.xref.startxref.is_none() {
        push_once(
            findings,
            seen,
            "lazy.startxref_missing".to_string(),
            Severity::Error,
            "No startxref offset was found in the file tail".to_string(),
            None,
            None,
            None,
            "Verify the PDF tail and %%EOF/startxref section".to_string(),
        );
    }

    if document.metadata.root.is_none() {
        push_once(
            findings,
            seen,
            "lazy.root_missing".to_string(),
            Severity::Error,
            "The trailer does not contain a /Root catalog reference".to_string(),
            None,
            None,
            document.xref.startxref,
            "Inspect the trailer dictionary and root catalog reference".to_string(),
        );
    } else if let Some(root) = document.metadata.root {
        add_reference_xref_finding(
            document,
            context,
            root,
            "lazy.root_missing_from_xref",
            "Root catalog reference",
            findings,
            seen,
            checked_references,
        );
    }

    if document.metadata.encrypted {
        push_once(
            findings,
            seen,
            "document.encryption_detected".to_string(),
            Severity::Warning,
            "The trailer contains /Encrypt; encrypted content may not be inspectable without credentials".to_string(),
            None,
            None,
            None,
            "Reproduce with an unencrypted fixture when possible, or provide credentials to downstream tools".to_string(),
        );
    }

    if document.metadata.incremental_update_count > 0 {
        push_once(
            findings,
            seen,
            "document.incremental_updates_detected".to_string(),
            Severity::Info,
            format!(
                "Detected {} incremental update section(s)",
                document.metadata.incremental_update_count
            ),
            None,
            None,
            None,
            "Check whether the issue reproduces before and after the incremental update section"
                .to_string(),
        );
    }

    if document.metadata.has_xref_stream {
        push_once(
            findings,
            seen,
            "document.xref_stream_detected".to_string(),
            Severity::Info,
            "Xref stream detected; lazy fast diagnostics use bounded xref stream metadata when available"
                .to_string(),
            None,
            None,
            document.xref.startxref,
            "Inspect referenced objects through the lazy Object Inspector to validate xref stream entries".to_string(),
        );
    }

    if document.metadata.has_object_stream {
        push_once(
            findings,
            seen,
            "document.object_stream_detected".to_string(),
            Severity::Info,
            "Object stream detected; lazy object inspection can expand selected compressed members on demand"
                .to_string(),
            None,
            None,
            None,
            "Open a referenced object to inspect the selected object-stream member within lazy limits"
                .to_string(),
        );
    }
}

fn add_page_findings(
    document: &LazyPdfDocument,
    context: &mut LazyFastDiagnosticContext<'_>,
    page_index: &PageIndex,
    findings: &mut Vec<Finding>,
    seen: &mut HashSet<String>,
    checked_references: &mut HashSet<ObjectRef>,
    checked_streams: &mut HashSet<ObjectRef>,
) {
    for page in &page_index.pages {
        if page.media_box.is_none() {
            push_once(
                findings,
                seen,
                "page.missing_mediabox".to_string(),
                Severity::Warning,
                format!(
                    "Page {} ({}) has no /MediaBox and does not inherit one",
                    page.page_number, page.reference
                ),
                Some(page.reference),
                Some(page.page_number),
                None,
                "Add /MediaBox to the page or an ancestor /Pages node".to_string(),
            );
        }

        add_reference_xref_finding(
            document,
            context,
            page.reference,
            "lazy.page_missing_from_xref",
            "Page reference",
            findings,
            seen,
            checked_references,
        );

        for link in &page.links {
            add_reference_xref_finding(
                document,
                context,
                link.reference,
                "lazy.page_link_missing_from_xref",
                &format!(
                    "Page {} {} link",
                    page.page_number,
                    page_link_label(&link.kind)
                ),
                findings,
                seen,
                checked_references,
            );

            if matches!(
                link.kind,
                PageObjectLinkKind::Contents
                    | PageObjectLinkKind::XObject
                    | PageObjectLinkKind::Image
            ) {
                if !checked_streams.insert(link.reference) {
                    continue;
                }
                if checked_streams.len() > LAZY_FAST_STREAM_METADATA_LIMIT {
                    if !context.stream_metadata_limit_reported {
                        context.stream_metadata_limit_reported = true;
                        push_once(
                            findings,
                            seen,
                            "lazy.fast_stream_metadata_limit".to_string(),
                            Severity::Info,
                            format!(
                                "Lazy fast diagnostics checked the first {LAZY_FAST_STREAM_METADATA_LIMIT} unique stream dictionaries and deferred additional stream metadata checks"
                            ),
                            None,
                            Some(page.page_number),
                            None,
                            "Open a specific stream or run selected deep diagnostics for additional stream checks"
                                .to_string(),
                        );
                    }
                    continue;
                }
                add_stream_metadata_findings(
                    document,
                    context,
                    link.reference,
                    Some(page.page_number),
                    findings,
                    seen,
                );
            }
        }
    }
}

fn add_stream_metadata_findings(
    document: &LazyPdfDocument,
    context: &LazyFastDiagnosticContext<'_>,
    reference: ObjectRef,
    page: Option<u32>,
    findings: &mut Vec<Finding>,
    seen: &mut HashSet<String>,
) {
    let stream = match crate::lazy_pdf::read_lazy_stream_metadata_for_diagnostics(
        &document.path,
        &document.xref,
        reference,
    ) {
        Ok(stream) => stream,
        Err(error) => {
            push_once(
                findings,
                seen,
                "lazy.stream.metadata_unavailable".to_string(),
                Severity::Warning,
                format!("Could not read stream metadata for {reference}: {error}"),
                Some(reference),
                page,
                xref_offset(context, reference),
                "Inspect the object by xref offset and verify its stream dictionary".to_string(),
            );
            return;
        }
    };

    match validate_stream_length(document, reference, &stream) {
        Ok(()) => {}
        Err(message) => push_once(
            findings,
            seen,
            "lazy.stream.invalid_length".to_string(),
            Severity::Warning,
            message,
            Some(reference),
            page,
            Some(stream.raw_range.start),
            "Inspect the stream dictionary /Length and compare it with the xref byte range"
                .to_string(),
        ),
    }

    for filter in &stream.filters {
        if !is_metadata_supported_filter(filter) {
            push_once(
                findings,
                seen,
                "stream.unsupported_filter".to_string(),
                Severity::Warning,
                format!(
                    "Stream {reference} uses unsupported or deferred filter /{filter}; lazy fast diagnostics did not decode the stream bytes"
                ),
                Some(reference),
                page,
                Some(stream.raw_range.start),
                "Use raw export or add support for this filter before deep stream validation"
                    .to_string(),
            );
        }
    }
}

fn validate_stream_length(
    document: &LazyPdfDocument,
    reference: ObjectRef,
    stream: &PdfStream,
) -> std::result::Result<(), String> {
    let Some(length) = stream.declared_length else {
        return Err(format!(
            "Stream {reference} has no numeric /Length; marker-based lazy validation is deferred"
        ));
    };
    let end = stream
        .raw_range
        .start
        .checked_add(length)
        .ok_or_else(|| format!("Stream {reference} /Length overflows byte range"))?;
    if end > document.metadata.file_size {
        return Err(format!(
            "Stream {reference} byte range {}..{} exceeds file size {}",
            stream.raw_range.start, end, document.metadata.file_size
        ));
    }
    Ok(())
}

fn add_reference_xref_finding(
    document: &LazyPdfDocument,
    context: &LazyFastDiagnosticContext<'_>,
    reference: ObjectRef,
    rule_id: &str,
    label: &str,
    findings: &mut Vec<Finding>,
    seen: &mut HashSet<String>,
    checked_references: &mut HashSet<ObjectRef>,
) {
    if !checked_references.insert(reference) {
        return;
    }

    let Some(entry) = context.xref_lookup.get(&reference).copied() else {
        push_once(
            findings,
            seen,
            rule_id.to_string(),
            Severity::Error,
            format!("{label} {reference} is not present in the lazy xref table"),
            Some(reference),
            None,
            None,
            format!("Check whether {reference} is missing, in an object stream, or omitted from the classic xref table"),
        );
        return;
    };
    if !entry.in_use {
        push_once(
            findings,
            seen,
            "lazy.reference_free_xref_entry".to_string(),
            Severity::Error,
            format!("{label} {reference} points to a free xref entry"),
            Some(reference),
            None,
            Some(entry.offset),
            "Verify the xref entry and regenerate the referenced object".to_string(),
        );
        return;
    }
    if entry.offset >= document.metadata.file_size {
        push_once(
            findings,
            seen,
            "xref.invalid_offset".to_string(),
            Severity::Error,
            format!(
                "Xref entry for {reference} points outside the file at byte {}",
                entry.offset
            ),
            Some(reference),
            None,
            Some(entry.offset),
            "Compare the xref table against the raw file length and object header".to_string(),
        );
        return;
    }
}

fn xref_offset(context: &LazyFastDiagnosticContext<'_>, reference: ObjectRef) -> Option<usize> {
    context
        .xref_lookup
        .get(&reference)
        .map(|entry| entry.offset)
}

fn add_deep_deferred_finding(
    document: &LazyPdfDocument,
    findings: &mut Vec<Finding>,
    seen: &mut HashSet<String>,
) {
    push_once(
        findings,
        seen,
        "lazy.deep_diagnostics_deferred".to_string(),
        Severity::Info,
        "Large PDF opened in lazy mode. Fast diagnostics are ready; stream decode validation, content stream analysis, page object extraction, image/font deep validation, and full report generation are deferred."
            .to_string(),
        document.metadata.root,
        None,
        document.xref.startxref,
        "Inspect individual objects/streams on demand, or run full diagnostics on a smaller repro PDF."
            .to_string(),
    );
}

fn push_once(
    findings: &mut Vec<Finding>,
    seen: &mut HashSet<String>,
    rule_id: String,
    severity: Severity,
    message: String,
    object: Option<ObjectRef>,
    page: Option<u32>,
    byte_offset: Option<usize>,
    suggested_next_step: String,
) {
    let key = format!("{rule_id}|{object:?}|{page:?}|{byte_offset:?}|{message}");
    if !seen.insert(key) {
        return;
    }
    findings.push(Finding {
        rule_id,
        severity,
        message,
        object,
        page,
        byte_offset,
        suggested_next_step,
    });
}

fn is_metadata_supported_filter(filter: &str) -> bool {
    matches!(
        filter,
        "FlateDecode"
            | "Fl"
            | "ASCIIHexDecode"
            | "AHx"
            | "ASCII85Decode"
            | "A85"
            | "RunLengthDecode"
            | "RL"
            | "DCTDecode"
            | "DCT"
    )
}

fn page_link_label(kind: &PageObjectLinkKind) -> &'static str {
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

#[allow(dead_code)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lazy_fast_diagnostics_reports_stream_metadata_without_decoding() {
        let directory = std::env::temp_dir().join(format!(
            "pdf-debugger-lazy-diagnostics-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&directory).expect("create temp dir");
        let path = directory.join("lazy-diagnostics.pdf");
        std::fs::write(&path, diagnostic_pdf()).expect("write lazy diagnostic fixture");

        let mut document = crate::lazy_pdf::open_lazy_pdf(&path).expect("lazy open");
        let findings = run_lazy_fast_diagnostics(&mut document);

        assert!(findings
            .iter()
            .any(|finding| finding.rule_id == "stream.unsupported_filter"
                && finding.object == Some(ObjectRef::new(4, 0))));
        assert!(findings
            .iter()
            .any(|finding| finding.rule_id == "lazy.deep_diagnostics_deferred"));
        assert!(findings
            .iter()
            .all(|finding| !finding.message.contains("BT /F1")));

        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_dir_all(directory);
    }

    fn diagnostic_pdf() -> Vec<u8> {
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
                b"4 0 obj\n<< /Length 23 /Filter /MysteryDecode >>\nstream\nBT /F1 12 Tf (Hi) Tj ET\nendstream\nendobj\n",
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

    fn push(bytes: &mut Vec<u8>, object: &[u8]) -> usize {
        let offset = bytes.len();
        bytes.extend_from_slice(object);
        offset
    }
}
