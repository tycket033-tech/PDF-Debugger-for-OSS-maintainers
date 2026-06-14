use crate::content_ops::ContentAnalysis;
use crate::lazy_pdf::{
    build_lazy_page_index, inspect_lazy_object, read_lazy_stream_for_content, view_lazy_stream,
};
use crate::pdf_model::{DiagnosticSummary, Finding, ObjectRef, Severity};
use crate::stream_decode::DecodeIssueKind;
use crate::stream_viewer::inspect_content_stream;
use crate::{LazyPdfDocument, PdfDebuggerError, Result};
use serde::Serialize;

const PAGE_DEEP_STREAM_LIMIT: usize = 4;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case", tag = "scope")]
pub enum LazyDeepDiagnosticRequest {
    Page { page_number: u32 },
    Stream { reference: ObjectRef },
    Object { reference: ObjectRef },
}

#[derive(Clone, Debug, Serialize)]
pub struct LazyReportEnrichment {
    pub scope: String,
    pub target: String,
    pub page_number: Option<u32>,
    pub reference: Option<ObjectRef>,
    pub partial: bool,
    pub summary: String,
    pub diagnostics: DiagnosticSummary,
    pub findings: Vec<Finding>,
    pub warnings: Vec<String>,
    pub limitations: Vec<String>,
    pub details: Vec<LazyEnrichmentDetail>,
}

#[derive(Clone, Debug, Serialize)]
pub struct LazyEnrichmentDetail {
    pub name: String,
    pub value: String,
}

pub fn run_lazy_deep_diagnostics(
    document: &mut LazyPdfDocument,
    request: LazyDeepDiagnosticRequest,
) -> Result<LazyReportEnrichment> {
    match request {
        LazyDeepDiagnosticRequest::Page { page_number } => {
            run_lazy_page_deep_diagnostics(document, page_number)
        }
        LazyDeepDiagnosticRequest::Stream { reference } => {
            run_lazy_stream_deep_diagnostics(document, reference, None)
        }
        LazyDeepDiagnosticRequest::Object { reference } => {
            run_lazy_object_deep_diagnostics(document, reference)
        }
    }
}

pub fn run_lazy_page_deep_diagnostics(
    document: &mut LazyPdfDocument,
    page_number: u32,
) -> Result<LazyReportEnrichment> {
    let page_index = build_lazy_page_index(document);
    let page = page_index
        .pages
        .iter()
        .find(|page| page.page_number == page_number)
        .ok_or_else(|| PdfDebuggerError::LazyOpen {
            message: format!("page {page_number} was not found in the lazy page index"),
        })?;

    let mut findings = Vec::new();
    let mut warnings = Vec::new();
    let mut limitations = vec![
        "Precise lazy page object extraction is still deferred; this enrichment inspects page metadata, object links, and a bounded number of content streams.".to_string(),
    ];
    let mut details = vec![
        detail("Page reference", page.reference.to_string()),
        detail("Rotation", page.rotation.unwrap_or_default().to_string()),
        detail("Fonts", page.resources.fonts.to_string()),
        detail("XObjects", page.resources.xobjects.to_string()),
        detail("Images", page.resources.images.to_string()),
        detail("Content streams", page.resources.contents.to_string()),
        detail("Annotations", page.resources.annotations.to_string()),
    ];

    if let Some(media_box) = page.media_box {
        details.push(detail(
            "MediaBox",
            format!(
                "{} {} {} {}",
                media_box.lower_left_x,
                media_box.lower_left_y,
                media_box.upper_right_x,
                media_box.upper_right_y
            ),
        ));
    } else {
        findings.push(Finding {
            rule_id: "lazy.deep.page_missing_mediabox".to_string(),
            severity: Severity::Warning,
            message: format!(
                "Page {page_number} ({}) has no inherited MediaBox in lazy metadata",
                page.reference
            ),
            object: Some(page.reference),
            page: Some(page_number),
            byte_offset: None,
            suggested_next_step: "Inspect the page tree ancestors for a /MediaBox entry"
                .to_string(),
        });
    }

    let content_links = page
        .links
        .iter()
        .filter(|link| matches!(link.kind, crate::PageObjectLinkKind::Contents))
        .map(|link| link.reference)
        .collect::<Vec<_>>();
    let resource_link_count = page
        .links
        .iter()
        .filter(|link| {
            matches!(
                link.kind,
                crate::PageObjectLinkKind::Resources
                    | crate::PageObjectLinkKind::Font
                    | crate::PageObjectLinkKind::XObject
                    | crate::PageObjectLinkKind::Image
                    | crate::PageObjectLinkKind::Annotation
            )
        })
        .count();
    details.push(detail("Page object links", page.links.len().to_string()));
    details.push(detail("Resource links", resource_link_count.to_string()));

    for reference in content_links.iter().take(PAGE_DEEP_STREAM_LIMIT) {
        match run_lazy_stream_deep_diagnostics(document, *reference, Some(page_number)) {
            Ok(stream) => {
                findings.extend(stream.findings);
                warnings.extend(
                    stream
                        .warnings
                        .into_iter()
                        .map(|warning| format!("Content stream {reference}: {warning}")),
                );
                details.push(detail(
                    format!("Content stream {reference}"),
                    stream.summary,
                ));
            }
            Err(error) => findings.push(Finding {
                rule_id: "lazy.deep.page_content_stream_unavailable".to_string(),
                severity: Severity::Warning,
                message: format!(
                    "Could not run deep diagnostics for content stream {reference}: {error}"
                ),
                object: Some(*reference),
                page: Some(page_number),
                byte_offset: None,
                suggested_next_step:
                    "Inspect this content stream directly through lazy stream diagnostics"
                        .to_string(),
            }),
        }
    }

    if content_links.len() > PAGE_DEEP_STREAM_LIMIT {
        limitations.push(format!(
            "Only the first {PAGE_DEEP_STREAM_LIMIT} content stream(s) were analyzed for this page; {} additional stream(s) were skipped.",
            content_links.len().saturating_sub(PAGE_DEEP_STREAM_LIMIT)
        ));
    }

    Ok(enrichment(
        "page",
        format!("Page {page_number}"),
        Some(page_number),
        Some(page.reference),
        !limitations.is_empty(),
        format!(
            "Page {page_number} lazy deep diagnostics inspected {} link(s) and {} content stream(s).",
            page.links.len(),
            content_links.len().min(PAGE_DEEP_STREAM_LIMIT)
        ),
        findings,
        warnings,
        limitations,
        details,
    ))
}

pub fn run_lazy_stream_deep_diagnostics(
    document: &LazyPdfDocument,
    reference: ObjectRef,
    page_number: Option<u32>,
) -> Result<LazyReportEnrichment> {
    let view = view_lazy_stream(&document.path, reference)?;
    let mut findings = Vec::new();
    let warnings = view
        .warnings
        .iter()
        .map(|warning| warning.message.clone())
        .collect::<Vec<_>>();
    let mut limitations = Vec::new();
    let mut details = vec![
        detail("Declared length", option_usize(view.declared_length)),
        detail("Actual length", view.actual_length.to_string()),
        detail(
            "Filters",
            if view.filters.is_empty() {
                "None".to_string()
            } else {
                view.filters.join(" -> ")
            },
        ),
        detail("Decoded length", option_usize(view.decoded_length)),
        detail(
            "Raw range",
            format!("{}..{}", view.raw_range.start, view.raw_range.end),
        ),
    ];

    for issue in &view.decode_issues {
        findings.push(Finding {
            rule_id: match issue.kind {
                DecodeIssueKind::Failed => "lazy.deep.stream_decode_failed",
                DecodeIssueKind::Unsupported => "lazy.deep.stream_unsupported_filter",
            }
            .to_string(),
            severity: match issue.kind {
                DecodeIssueKind::Failed => Severity::Error,
                DecodeIssueKind::Unsupported => Severity::Warning,
            },
            message: format!(
                "Stream {reference} filter /{} could not be decoded during lazy deep diagnostics: {}",
                issue.filter, issue.message
            ),
            object: Some(reference),
            page: page_number,
            byte_offset: Some(view.raw_range.start),
            suggested_next_step:
                "Export the raw stream or inspect the /Filter chain and encoded bytes".to_string(),
        });
    }

    if view.raw_text_truncated || view.hex_text_truncated {
        limitations.push(format!(
            "Raw/hex preview was truncated at {} bytes; report enrichment does not embed stream bytes.",
            view.preview_limit
        ));
    }
    if view.decoded_text_truncated {
        limitations.push(format!(
            "Decoded preview was truncated at {} bytes; report enrichment does not embed decoded bytes.",
            view.preview_limit
        ));
    }
    if let Some(error) = &view.decoded_error {
        limitations.push(format!("Decoded content analysis unavailable: {error}"));
    }

    match read_lazy_stream_for_content(&document.path, reference) {
        Ok(stream) => match inspect_content_stream(reference, &stream) {
            Ok(content) => {
                add_content_analysis_details(&mut details, &content.analysis);
                for warning in content.analysis.warnings {
                    findings.push(Finding {
                        rule_id: format!("lazy.deep.{}", warning.rule_id),
                        severity: Severity::Warning,
                        message: warning.message,
                        object: Some(reference),
                        page: page_number,
                        byte_offset: warning
                            .byte_range
                            .map(|range| view.raw_range.start.saturating_add(range.start)),
                        suggested_next_step: warning.suggested_next_step,
                    });
                }
            }
            Err(error) => {
                limitations.push(error.message);
            }
        },
        Err(error) => {
            limitations.push(format!("Content stream operator analysis skipped: {error}"));
        }
    }

    Ok(enrichment(
        "stream",
        format!("Stream {reference}"),
        page_number,
        Some(reference),
        !limitations.is_empty(),
        format!(
            "Stream {reference} lazy deep diagnostics inspected metadata, decode status, and bounded content operators."
        ),
        findings,
        warnings,
        limitations,
        details,
    ))
}

pub fn run_lazy_object_deep_diagnostics(
    document: &LazyPdfDocument,
    reference: ObjectRef,
) -> Result<LazyReportEnrichment> {
    let inspection = inspect_lazy_object(&document.path, reference)?;
    let mut findings = Vec::new();
    let mut warnings = Vec::new();
    let mut limitations = Vec::new();
    let mut details = vec![
        detail("Object type", inspection.object_type.clone()),
        detail("Summary", inspection.value_summary.clone()),
        detail(
            "Raw range",
            format!(
                "{}..{}",
                inspection.raw_range.start, inspection.raw_range.end
            ),
        ),
        detail("Dictionary keys", inspection.dictionary_keys.join(", ")),
        detail(
            "Referenced objects",
            inspection.references.len().to_string(),
        ),
    ];

    for linked in &inspection.references {
        match document.xref.entries.iter().find(|entry| entry.reference == *linked) {
            Some(entry) if entry.in_use && entry.offset < document.metadata.file_size => {}
            Some(entry) if !entry.in_use => findings.push(Finding {
                rule_id: "lazy.deep.object_reference_free".to_string(),
                severity: Severity::Error,
                message: format!("Object {reference} references free xref entry {linked}"),
                object: Some(*linked),
                page: None,
                byte_offset: Some(entry.offset),
                suggested_next_step: "Verify the referenced object and regenerate the xref table"
                    .to_string(),
            }),
            Some(entry) => findings.push(Finding {
                rule_id: "xref.invalid_offset".to_string(),
                severity: Severity::Error,
                message: format!(
                    "Object {reference} references {linked}, whose xref offset {} is outside the file",
                    entry.offset
                ),
                object: Some(*linked),
                page: None,
                byte_offset: Some(entry.offset),
                suggested_next_step: "Compare the xref table against the raw file length".to_string(),
            }),
            None => findings.push(Finding {
                rule_id: "object.missing_reference".to_string(),
                severity: Severity::Error,
                message: format!("Object {reference} references {linked}, which is missing from the lazy xref table"),
                object: Some(*linked),
                page: None,
                byte_offset: None,
                suggested_next_step:
                    "Check whether the referenced object is missing or stored in an unsupported object stream"
                        .to_string(),
            }),
        }
    }

    if let Some(stream) = inspection.stream {
        details.push(detail(
            "Stream actual length",
            stream.actual_length.to_string(),
        ));
        details.push(detail(
            "Stream filters",
            if stream.filters.is_empty() {
                "None".to_string()
            } else {
                stream.filters.join(" -> ")
            },
        ));

        match run_lazy_stream_deep_diagnostics(document, reference, None) {
            Ok(stream_enrichment) => {
                findings.extend(stream_enrichment.findings);
                warnings.extend(stream_enrichment.warnings);
                limitations.extend(stream_enrichment.limitations);
                details.extend(stream_enrichment.details.into_iter().map(|detail| {
                    LazyEnrichmentDetail {
                        name: format!("Stream {}", detail.name),
                        value: detail.value,
                    }
                }));
            }
            Err(error) => limitations.push(format!(
                "Stream-level diagnostics for object {reference} were unavailable: {error}"
            )),
        }
    }

    Ok(enrichment(
        "object",
        format!("Object {reference}"),
        None,
        Some(reference),
        !limitations.is_empty(),
        format!(
            "Object {reference} lazy deep diagnostics inspected one indirect object and {} reference(s).",
            inspection.references.len()
        ),
        findings,
        warnings,
        limitations,
        details,
    ))
}

fn add_content_analysis_details(
    details: &mut Vec<LazyEnrichmentDetail>,
    analysis: &ContentAnalysis,
) {
    details.push(detail("Content tokens", analysis.tokens.len().to_string()));
    details.push(detail(
        "Content operators",
        analysis.operators.len().to_string(),
    ));
    details.push(detail(
        "Content warnings",
        analysis.warnings.len().to_string(),
    ));
    let sample = analysis
        .operators
        .iter()
        .take(16)
        .map(|operator| operator.name.clone())
        .collect::<Vec<_>>()
        .join(" ");
    if !sample.is_empty() {
        details.push(detail("Operator sample", sample));
    }
}

fn enrichment(
    scope: &str,
    target: String,
    page_number: Option<u32>,
    reference: Option<ObjectRef>,
    partial: bool,
    summary: String,
    findings: Vec<Finding>,
    warnings: Vec<String>,
    limitations: Vec<String>,
    details: Vec<LazyEnrichmentDetail>,
) -> LazyReportEnrichment {
    let diagnostics = DiagnosticSummary::from_findings(&findings);
    LazyReportEnrichment {
        scope: scope.to_string(),
        target,
        page_number,
        reference,
        partial,
        summary,
        diagnostics,
        findings,
        warnings,
        limitations,
        details,
    }
}

fn detail(name: impl Into<String>, value: impl Into<String>) -> LazyEnrichmentDetail {
    LazyEnrichmentDetail {
        name: name.into(),
        value: value.into(),
    }
}

fn option_usize(value: Option<usize>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lazy_pdf::open_lazy_pdf;

    #[test]
    fn stream_deep_diagnostics_reports_content_operator_warnings_without_bytes() {
        let directory = std::env::temp_dir().join(format!(
            "pdf-debugger-lazy-deep-stream-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&directory).expect("create temp dir");
        let path = directory.join("lazy-deep.pdf");
        std::fs::write(&path, minimal_pdf(b"BT /F1 12 Tf ZZ ET")).expect("write fixture");

        let mut document = open_lazy_pdf(&path).expect("lazy open");
        let enrichment = run_lazy_deep_diagnostics(
            &mut document,
            LazyDeepDiagnosticRequest::Stream {
                reference: ObjectRef::new(4, 0),
            },
        )
        .expect("stream enrichment");

        assert_eq!(enrichment.scope, "stream");
        assert!(enrichment.findings.iter().any(|finding| {
            finding.rule_id == "lazy.deep.content.unknown_operator"
                && finding.object == Some(ObjectRef::new(4, 0))
        }));
        assert!(!serde_json::to_string(&enrichment)
            .expect("serialize")
            .contains("BT /F1 12 Tf"));

        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn page_deep_diagnostics_marks_page_object_extraction_partial() {
        let directory = std::env::temp_dir().join(format!(
            "pdf-debugger-lazy-deep-page-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&directory).expect("create temp dir");
        let path = directory.join("lazy-page.pdf");
        std::fs::write(&path, minimal_pdf(b"BT /F1 12 Tf (Hi) Tj ET")).expect("write fixture");

        let mut document = open_lazy_pdf(&path).expect("lazy open");
        let enrichment = run_lazy_deep_diagnostics(
            &mut document,
            LazyDeepDiagnosticRequest::Page { page_number: 1 },
        )
        .expect("page enrichment");

        assert_eq!(enrichment.scope, "page");
        assert!(enrichment.partial);
        assert!(enrichment
            .limitations
            .iter()
            .any(|limitation| limitation.contains("page object extraction")));

        let _ = std::fs::remove_dir_all(directory);
    }

    fn minimal_pdf(content: &[u8]) -> Vec<u8> {
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
                b"3 0 obj\n<< /Type /Page /Parent 2 0 R /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>\nendobj\n",
            ),
            push(&mut bytes, &stream_object(4, content)),
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

    fn stream_object(object: u32, content: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(
            format!("{object} 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes(),
        );
        bytes.extend_from_slice(content);
        bytes.extend_from_slice(b"\nendstream\nendobj\n");
        bytes
    }

    fn push(bytes: &mut Vec<u8>, object: &[u8]) -> usize {
        let offset = bytes.len();
        bytes.extend_from_slice(object);
        offset
    }
}
