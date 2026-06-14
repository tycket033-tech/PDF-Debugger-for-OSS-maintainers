use crate::lazy_deep_diagnostics::LazyReportEnrichment;
use crate::lazy_diagnostics::run_lazy_fast_diagnostics;
use crate::lazy_pdf::LazyPdfDocument;
use crate::pdf_model::{DiagnosticSummary, Finding, ObjectRef, ParsedPdf, PdfMetadata, Severity};
use serde::Serialize;

pub const REPORT_SCHEMA_VERSION: &str = "0.1.0";
pub const LAZY_REPORT_KIND: &str = "lazy_fast_diagnostics";
pub const LAZY_OPEN_MODE: &str = "lazy";
pub const LAZY_DEFERRED_DEEP_DIAGNOSTICS: &[&str] = &[
    "stream decode validation",
    "content stream operator analysis",
    "precise page object extraction",
    "image/font deep validation",
    "page screenshots",
    "full report enrichment",
];

#[derive(Clone, Debug, Serialize)]
pub struct DebugReport {
    pub schema_version: &'static str,
    pub file: PdfMetadata,
    pub diagnostics: DiagnosticSummary,
    pub findings: Vec<Finding>,
}

#[derive(Clone, Debug, Serialize)]
pub struct LazyDebugReport {
    pub schema_version: &'static str,
    pub report_kind: &'static str,
    pub open_mode: &'static str,
    pub file: PdfMetadata,
    pub diagnostics: DiagnosticSummary,
    pub findings: Vec<Finding>,
    pub enrichments: Vec<LazyReportEnrichment>,
    pub deferred_deep_diagnostics: Vec<String>,
}

pub fn build_report(pdf: &ParsedPdf, findings: Vec<Finding>) -> DebugReport {
    DebugReport {
        schema_version: REPORT_SCHEMA_VERSION,
        file: pdf.metadata.clone(),
        diagnostics: DiagnosticSummary::from_findings(&findings),
        findings,
    }
}

pub fn build_lazy_report(document: &mut LazyPdfDocument) -> LazyDebugReport {
    build_lazy_report_with_enrichments(document, Vec::new())
}

pub fn build_lazy_report_with_enrichments(
    document: &mut LazyPdfDocument,
    enrichments: Vec<LazyReportEnrichment>,
) -> LazyDebugReport {
    let findings = run_lazy_fast_diagnostics(document);
    let mut diagnostic_findings = findings.clone();
    for enrichment in &enrichments {
        diagnostic_findings.extend(enrichment.findings.clone());
    }
    let diagnostics = DiagnosticSummary::from_findings(&diagnostic_findings);
    LazyDebugReport {
        schema_version: REPORT_SCHEMA_VERSION,
        report_kind: LAZY_REPORT_KIND,
        open_mode: LAZY_OPEN_MODE,
        file: document.metadata.clone(),
        diagnostics,
        findings,
        enrichments,
        deferred_deep_diagnostics: LAZY_DEFERRED_DEEP_DIAGNOSTICS
            .iter()
            .map(|item| (*item).to_string())
            .collect(),
    }
}

pub fn render_json_report(report: &DebugReport) -> crate::Result<String> {
    Ok(serde_json::to_string_pretty(report)?)
}

pub fn render_lazy_json_report(report: &LazyDebugReport) -> crate::Result<String> {
    Ok(serde_json::to_string_pretty(report)?)
}

pub fn render_markdown_report(report: &DebugReport) -> String {
    let file = &report.file;
    let mut markdown = String::new();

    markdown.push_str("## PDF Debug Report\n\n");
    markdown.push_str(&format!("File: `{}`\n", file.file_name));
    markdown.push_str(&format!(
        "PDF version: {}\n",
        file.pdf_version.as_deref().unwrap_or("unknown")
    ));
    markdown.push_str(&format!(
        "Pages: {}\n",
        file.page_count
            .map(|count| count.to_string())
            .unwrap_or_else(|| "unknown".to_string())
    ));
    markdown.push_str(&format!("Encrypted: {}\n", yes_no(file.encrypted)));
    markdown.push_str(&format!("Linearized: {}\n", yes_no(file.linearized)));
    markdown.push_str(&format!(
        "Incremental updates: {}\n",
        file.incremental_update_count
    ));
    markdown.push_str(&format!("Objects: {}\n", file.object_count));
    markdown.push_str(&format!("Streams: {}\n\n", file.stream_count));

    push_findings_section(&mut markdown, "Errors", Severity::Error, &report.findings);
    push_findings_section(
        &mut markdown,
        "Warnings",
        Severity::Warning,
        &report.findings,
    );
    push_findings_section(&mut markdown, "Info", Severity::Info, &report.findings);

    markdown.push_str("### Suggested Next Steps\n\n");
    let mut pushed = 0;
    for finding in report.findings.iter().take(8) {
        markdown.push_str(&format!("- {}\n", finding.suggested_next_step));
        pushed += 1;
    }
    if pushed == 0 {
        markdown.push_str("- No diagnostics were emitted. Keep this report with the fixture for regression tracking.\n");
    }

    markdown
}

pub fn render_lazy_markdown_report(report: &LazyDebugReport) -> String {
    let file = &report.file;
    let mut markdown = String::new();

    markdown.push_str("## PDF Lazy Debug Report\n\n");
    if report.enrichments.is_empty() {
        markdown.push_str("> Lazy report. Fast diagnostics only. Deep diagnostics deferred.\n\n");
    } else {
        markdown.push_str(
            "> Lazy report. Fast diagnostics plus selected deep diagnostics enrichment.\n\n",
        );
    }
    markdown.push_str(&format!("File: `{}`\n", file.file_name));
    markdown.push_str(&format!("File size: {} bytes\n", file.file_size));
    markdown.push_str(&format!(
        "PDF version: {}\n",
        file.pdf_version.as_deref().unwrap_or("unknown")
    ));
    markdown.push_str("Open mode: `lazy`\n");
    markdown.push_str(if report.enrichments.is_empty() {
        "Scope: fast diagnostics only\n"
    } else {
        "Scope: fast diagnostics plus selected target enrichments\n"
    });
    markdown.push_str(&format!(
        "Pages: {}\n",
        file.page_count
            .map(|count| count.to_string())
            .unwrap_or_else(|| "unknown".to_string())
    ));
    markdown.push_str(&format!("Encrypted: {}\n", yes_no(file.encrypted)));
    markdown.push_str(&format!("Linearized: {}\n", yes_no(file.linearized)));
    markdown.push_str(&format!(
        "Incremental updates: {}\n",
        file.incremental_update_count
    ));
    markdown.push_str(&format!("Root: {}\n", optional_ref(file.root)));
    markdown.push_str(&format!("Info: {}\n", optional_ref(file.info)));
    markdown.push_str(&format!("Xref entries: {}\n", file.xref_entry_count));
    markdown.push_str(&format!("Objects: {}\n", file.object_count));
    markdown.push_str(&format!(
        "Streams: {} (metadata only in lazy report)\n\n",
        file.stream_count
    ));

    markdown.push_str("### Lazy Report Scope\n\n");
    markdown.push_str("- This report was generated from metadata-only lazy open data, lazy page metadata, and lazy fast diagnostics.\n");
    markdown.push_str("- It does not full-parse the PDF, scan all indirect objects, read all stream bytes, decode all streams, or extract page objects.\n");
    markdown.push_str("- Deep diagnostics are deferred and should be run on demand or against a smaller repro PDF.\n\n");

    markdown.push_str("### Deferred Deep Diagnostics\n\n");
    if !report.enrichments.is_empty() {
        markdown.push_str(
            "The selected enrichment target(s) below were run on demand. Other unrequested deep diagnostics remain deferred.\n\n",
        );
    }
    for item in &report.deferred_deep_diagnostics {
        markdown.push_str(&format!("- {item}\n"));
    }
    markdown.push('\n');

    if !report.enrichments.is_empty() {
        markdown.push_str("### Deep Diagnostics Enrichment\n\n");
        for enrichment in &report.enrichments {
            markdown.push_str(&format!(
                "#### {} ({})\n\n",
                enrichment.target, enrichment.scope
            ));
            markdown.push_str(&format!("Partial: {}\n\n", yes_no(enrichment.partial)));
            markdown.push_str(&format!("{}\n\n", enrichment.summary));
            if !enrichment.details.is_empty() {
                markdown.push_str("Details:\n\n");
                for detail in &enrichment.details {
                    markdown.push_str(&format!("- {}: {}\n", detail.name, detail.value));
                }
                markdown.push('\n');
            }
            if !enrichment.warnings.is_empty() {
                markdown.push_str("Warnings:\n\n");
                for warning in &enrichment.warnings {
                    markdown.push_str(&format!("- {warning}\n"));
                }
                markdown.push('\n');
            }
            if !enrichment.limitations.is_empty() {
                markdown.push_str("Limitations:\n\n");
                for limitation in &enrichment.limitations {
                    markdown.push_str(&format!("- {limitation}\n"));
                }
                markdown.push('\n');
            }
            push_findings_section(
                &mut markdown,
                "Enrichment Errors",
                Severity::Error,
                &enrichment.findings,
            );
            push_findings_section(
                &mut markdown,
                "Enrichment Warnings",
                Severity::Warning,
                &enrichment.findings,
            );
            push_findings_section(
                &mut markdown,
                "Enrichment Info",
                Severity::Info,
                &enrichment.findings,
            );
        }
    }

    markdown.push_str("### Diagnostic Summary\n\n");
    markdown.push_str(&format!("- Errors: {}\n", report.diagnostics.errors));
    markdown.push_str(&format!("- Warnings: {}\n", report.diagnostics.warnings));
    markdown.push_str(&format!("- Info: {}\n\n", report.diagnostics.info));

    push_findings_section(&mut markdown, "Errors", Severity::Error, &report.findings);
    push_findings_section(
        &mut markdown,
        "Warnings",
        Severity::Warning,
        &report.findings,
    );
    push_findings_section(&mut markdown, "Info", Severity::Info, &report.findings);

    markdown.push_str("### Suggested Next Steps\n\n");
    let mut pushed = 0;
    for finding in report.findings.iter().take(8) {
        markdown.push_str(&format!("- {}\n", finding.suggested_next_step));
        pushed += 1;
    }
    if pushed == 0 {
        markdown.push_str("- No lazy fast diagnostics findings were emitted. Keep this report with the fixture for regression tracking.\n");
    }

    markdown
}

fn push_findings_section(
    markdown: &mut String,
    heading: &str,
    severity: Severity,
    findings: &[Finding],
) {
    let filtered = findings
        .iter()
        .filter(|finding| finding.severity == severity)
        .collect::<Vec<_>>();
    if filtered.is_empty() {
        return;
    }

    markdown.push_str(&format!("### {heading}\n\n"));
    for finding in filtered {
        let object = finding
            .object
            .map(|reference| format!("Object `{reference}`: "))
            .unwrap_or_default();
        let page = finding
            .page
            .map(|page| format!("Page `{page}`: "))
            .unwrap_or_default();
        let offset = finding
            .byte_offset
            .map(|offset| format!(" Byte offset `{offset}`."))
            .unwrap_or_default();
        markdown.push_str(&format!(
            "- {}{}{}{}\n",
            page, object, finding.message, offset
        ));
    }
    markdown.push('\n');
}

fn optional_ref(reference: Option<ObjectRef>) -> String {
    reference
        .map(|reference| reference.to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}
