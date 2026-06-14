use crate::content_ops::{analyze_content_stream, font_names_used, xobject_names_used};
use crate::pdf_model::{
    Finding, ObjectRef, ParsedPdf, PdfDictionary, PdfObject, PdfValue, Severity,
};
use crate::stream_decode::{decode_stream, DecodeIssueKind};
use std::collections::{BTreeSet, HashSet};

pub fn run_diagnostics(pdf: &ParsedPdf) -> Vec<Finding> {
    let mut findings = Vec::new();

    add_parse_warnings(pdf, &mut findings);
    add_document_feature_findings(pdf, &mut findings);
    add_xref_findings(pdf, &mut findings);
    add_missing_reference_findings(pdf, &mut findings);
    add_stream_findings(pdf, &mut findings);
    add_page_tree_findings(pdf, &mut findings);

    findings
}

fn add_parse_warnings(pdf: &ParsedPdf, findings: &mut Vec<Finding>) {
    for warning in &pdf.warnings {
        findings.push(Finding {
            rule_id: "parse.warning".to_string(),
            severity: Severity::Warning,
            message: warning.message.clone(),
            object: None,
            page: None,
            byte_offset: warning.offset,
            suggested_next_step:
                "Open the file in inspect mode and verify the PDF header, xref table, and trailer"
                    .to_string(),
        });
    }
}

fn add_document_feature_findings(pdf: &ParsedPdf, findings: &mut Vec<Finding>) {
    if pdf.metadata.encrypted {
        findings.push(Finding {
            rule_id: "document.encryption_detected".to_string(),
            severity: Severity::Warning,
            message: "The trailer contains /Encrypt; encrypted content may not be inspectable without credentials".to_string(),
            object: None,
            page: None,
            byte_offset: None,
            suggested_next_step: "Reproduce with an unencrypted fixture when possible, or provide credentials to downstream tools".to_string(),
        });
    }

    if pdf.metadata.incremental_update_count > 0 {
        findings.push(Finding {
            rule_id: "document.incremental_updates_detected".to_string(),
            severity: Severity::Info,
            message: format!(
                "Detected {} incremental update section(s)",
                pdf.metadata.incremental_update_count
            ),
            object: None,
            page: None,
            byte_offset: None,
            suggested_next_step:
                "Check whether the issue reproduces before and after the incremental update section"
                    .to_string(),
        });
    }

    if pdf.metadata.has_object_stream {
        findings.push(Finding {
            rule_id: "document.object_stream_detected".to_string(),
            severity: Severity::Info,
            message: "Object stream detected; Milestone 1 reports it but does not decompress object streams".to_string(),
            object: first_object_with_type(pdf, "ObjStm"),
            page: None,
            byte_offset: None,
            suggested_next_step: "Use a later parser milestone or an external decompressor to inspect compressed object members".to_string(),
        });
    }

    if pdf.metadata.has_xref_stream {
        findings.push(Finding {
            rule_id: "document.xref_stream_detected".to_string(),
            severity: Severity::Info,
            message: "Xref stream detected; Milestone 1 reports it but targets classic xref tables".to_string(),
            object: first_object_with_type(pdf, "XRef"),
            page: None,
            byte_offset: None,
            suggested_next_step: "Save or regenerate a classic-xref fixture when possible, or inspect with a parser that expands xref streams".to_string(),
        });
    }
}

fn add_xref_findings(pdf: &ParsedPdf, findings: &mut Vec<Finding>) {
    for entry in pdf
        .xref_entries
        .iter()
        .filter(|entry| entry.in_use && entry.valid_offset == Some(false))
    {
        findings.push(Finding {
            rule_id: "xref.invalid_offset".to_string(),
            severity: Severity::Error,
            message: format!(
                "Xref entry for object {} points to byte {}, but that offset does not contain the expected object header",
                entry.reference, entry.offset
            ),
            object: Some(entry.reference),
            page: None,
            byte_offset: Some(entry.offset),
            suggested_next_step: format!("Dump object {} and compare the xref offset against the raw file bytes", entry.reference),
        });
    }
}

fn add_missing_reference_findings(pdf: &ParsedPdf, findings: &mut Vec<Finding>) {
    let mut references = BTreeSet::new();
    for object in pdf.objects.values() {
        collect_references(&object.value, &mut references);
        if let Some(stream) = &object.stream {
            collect_references(
                &PdfValue::Dictionary(stream.dictionary.clone()),
                &mut references,
            );
        }
    }
    if let Some(trailer) = &pdf.trailer {
        collect_references(trailer, &mut references);
    }

    for reference in references {
        if !pdf.objects.contains_key(&reference) {
            findings.push(Finding {
                rule_id: "object.missing_reference".to_string(),
                severity: Severity::Error,
                message: format!("Referenced object {reference} was not found among parsed indirect objects"),
                object: Some(reference),
                page: None,
                byte_offset: None,
                suggested_next_step: format!("Check whether {reference} is absent, hidden in an object stream, or omitted from the xref table"),
            });
        }
    }
}

fn add_stream_findings(pdf: &ParsedPdf, findings: &mut Vec<Finding>) {
    for object in pdf.objects.values() {
        let Some(stream) = &object.stream else {
            continue;
        };

        if let Some(declared) = stream.declared_length {
            if declared != stream.actual_length {
                findings.push(Finding {
                    rule_id: "stream.length_mismatch".to_string(),
                    severity: Severity::Warning,
                    message: format!(
                        "Stream {} declares /Length {}, but {} byte(s) were found before endstream",
                        object.reference, declared, stream.actual_length
                    ),
                    object: Some(object.reference),
                    page: None,
                    byte_offset: Some(stream.raw_range.start),
                    suggested_next_step: format!("Regenerate or inspect stream {} and verify the writer's /Length calculation", object.reference),
                });
            }
        }

        let decode = decode_stream(stream);
        for issue in decode.issues {
            findings.push(Finding {
                rule_id: match issue.kind {
                    DecodeIssueKind::Failed => "stream.decode_failed",
                    DecodeIssueKind::Unsupported => "stream.unsupported_filter",
                }
                .to_string(),
                severity: match issue.kind {
                    DecodeIssueKind::Failed => Severity::Error,
                    DecodeIssueKind::Unsupported => Severity::Warning,
                },
                message: format!(
                    "Stream {} filter /{} could not be decoded: {}",
                    object.reference, issue.filter, issue.message
                ),
                object: Some(object.reference),
                page: None,
                byte_offset: Some(stream.raw_range.start),
                suggested_next_step: format!(
                    "Dump stream {} and verify the /Filter chain and encoded bytes",
                    object.reference
                ),
            });
        }

        add_image_stream_findings(object, findings);
    }
}

fn add_image_stream_findings(object: &PdfObject, findings: &mut Vec<Finding>) {
    let Some(dictionary) = object.dictionary() else {
        return;
    };
    if !name_is(dictionary, "Subtype", "Image") {
        return;
    }

    for key in ["Width", "Height", "ColorSpace", "BitsPerComponent"] {
        if !dictionary.contains_key(key) {
            findings.push(Finding {
                rule_id: "image.missing_required_key".to_string(),
                severity: Severity::Warning,
                message: format!("Image stream {} is missing /{}", object.reference, key),
                object: Some(object.reference),
                page: None,
                byte_offset: object.stream.as_ref().map(|stream| stream.raw_range.start),
                suggested_next_step:
                    "Verify the image XObject dictionary written by the PDF generator".to_string(),
            });
        }
    }
}

fn add_page_tree_findings(pdf: &ParsedPdf, findings: &mut Vec<Finding>) {
    let Some(root_reference) = pdf.metadata.root else {
        findings.push(Finding {
            rule_id: "page_tree.missing_catalog".to_string(),
            severity: Severity::Error,
            message: "Trailer does not contain a /Root catalog reference".to_string(),
            object: None,
            page: None,
            byte_offset: None,
            suggested_next_step: "Verify the trailer dictionary and root catalog object"
                .to_string(),
        });
        return;
    };

    let Some(catalog) = pdf.object(root_reference).and_then(PdfObject::dictionary) else {
        findings.push(Finding {
            rule_id: "page_tree.invalid_catalog_reference".to_string(),
            severity: Severity::Error,
            message: format!(
                "Root catalog reference {root_reference} does not resolve to a dictionary"
            ),
            object: Some(root_reference),
            page: None,
            byte_offset: None,
            suggested_next_step: "Check the trailer /Root entry and referenced object".to_string(),
        });
        return;
    };

    let Some(pages_value) = catalog.get("Pages") else {
        findings.push(Finding {
            rule_id: "page_tree.missing_pages".to_string(),
            severity: Severity::Error,
            message: format!("Catalog {root_reference} does not contain /Pages"),
            object: Some(root_reference),
            page: None,
            byte_offset: None,
            suggested_next_step: "Verify that the catalog points to a valid page tree root"
                .to_string(),
        });
        return;
    };

    let Some(pages_reference) = pages_value.as_reference() else {
        findings.push(Finding {
            rule_id: "page_tree.invalid_pages_reference".to_string(),
            severity: Severity::Error,
            message: format!("Catalog {root_reference} has /Pages that is not an object reference"),
            object: Some(root_reference),
            page: None,
            byte_offset: None,
            suggested_next_step: "Write /Pages as an indirect reference to a /Pages dictionary"
                .to_string(),
        });
        return;
    };

    let mut visited = HashSet::new();
    let mut page_number = 0;
    walk_page_node(
        pdf,
        pages_reference,
        InheritedPageState::default(),
        &mut page_number,
        &mut visited,
        findings,
    );
}

#[derive(Clone, Default)]
struct InheritedPageState {
    has_media_box: bool,
    resources: Option<PdfValue>,
}

fn walk_page_node(
    pdf: &ParsedPdf,
    reference: ObjectRef,
    inherited: InheritedPageState,
    page_number: &mut u32,
    visited: &mut HashSet<ObjectRef>,
    findings: &mut Vec<Finding>,
) {
    if !visited.insert(reference) {
        findings.push(Finding {
            rule_id: "page_tree.cyclic_reference".to_string(),
            severity: Severity::Error,
            message: format!("Page tree cycle detected at {reference}"),
            object: Some(reference),
            page: None,
            byte_offset: None,
            suggested_next_step: "Break the cycle in the /Kids or /Parent references".to_string(),
        });
        return;
    }

    let Some(object) = pdf.object(reference) else {
        findings.push(Finding {
            rule_id: "page_tree.missing_node".to_string(),
            severity: Severity::Error,
            message: format!("Page tree node {reference} is missing"),
            object: Some(reference),
            page: None,
            byte_offset: None,
            suggested_next_step: "Verify the page tree /Kids array and xref entries".to_string(),
        });
        return;
    };
    let Some(dictionary) = object.dictionary() else {
        findings.push(Finding {
            rule_id: "page_tree.node_not_dictionary".to_string(),
            severity: Severity::Error,
            message: format!("Page tree node {reference} is not a dictionary"),
            object: Some(reference),
            page: None,
            byte_offset: Some(object.raw_range.start),
            suggested_next_step: "Replace the node with a valid /Pages or /Page dictionary"
                .to_string(),
        });
        return;
    };

    let inherited = InheritedPageState {
        has_media_box: inherited.has_media_box || dictionary.contains_key("MediaBox"),
        resources: dictionary
            .get("Resources")
            .cloned()
            .or_else(|| inherited.resources.clone()),
    };

    match dictionary.get("Type").and_then(PdfValue::as_name) {
        Some("Pages") => {
            let Some(kids) = dictionary.get("Kids").and_then(PdfValue::as_array) else {
                findings.push(Finding {
                    rule_id: "page_tree.pages_without_kids".to_string(),
                    severity: Severity::Error,
                    message: format!("/Pages node {reference} does not contain a /Kids array"),
                    object: Some(reference),
                    page: None,
                    byte_offset: Some(object.raw_range.start),
                    suggested_next_step:
                        "Write a /Kids array containing page or page-tree references".to_string(),
                });
                return;
            };
            for kid in kids {
                if let Some(kid_reference) = kid.as_reference() {
                    walk_page_node(
                        pdf,
                        kid_reference,
                        inherited.clone(),
                        page_number,
                        visited,
                        findings,
                    );
                } else {
                    findings.push(Finding {
                        rule_id: "page_tree.invalid_kid_reference".to_string(),
                        severity: Severity::Error,
                        message: format!("/Pages node {reference} contains a /Kids entry that is not a reference"),
                        object: Some(reference),
                        page: None,
                        byte_offset: Some(object.raw_range.start),
                        suggested_next_step: "Ensure every /Kids entry is an indirect object reference".to_string(),
                    });
                }
            }
        }
        Some("Page") => {
            *page_number += 1;
            if !inherited.has_media_box {
                findings.push(Finding {
                    rule_id: "page.missing_mediabox".to_string(),
                    severity: Severity::Warning,
                    message: format!(
                        "Page {} ({reference}) has no /MediaBox and does not inherit one",
                        page_number
                    ),
                    object: Some(reference),
                    page: Some(*page_number),
                    byte_offset: Some(object.raw_range.start),
                    suggested_next_step: "Add /MediaBox to the page or an ancestor /Pages node"
                        .to_string(),
                });
            }
            add_page_resource_findings(
                pdf,
                reference,
                dictionary,
                inherited.resources.as_ref(),
                *page_number,
                findings,
            );
        }
        other => findings.push(Finding {
            rule_id: "page_tree.invalid_node_type".to_string(),
            severity: Severity::Error,
            message: format!(
                "Page tree node {reference} has unexpected /Type {:?}",
                other
            ),
            object: Some(reference),
            page: None,
            byte_offset: Some(object.raw_range.start),
            suggested_next_step: "Ensure page tree nodes use /Type /Pages or /Type /Page"
                .to_string(),
        }),
    }
}

fn add_page_resource_findings(
    pdf: &ParsedPdf,
    page_reference: ObjectRef,
    page_dictionary: &PdfDictionary,
    resources_value: Option<&PdfValue>,
    page_number: u32,
    findings: &mut Vec<Finding>,
) {
    let Some(content_bytes) = page_content_bytes(pdf, page_dictionary) else {
        return;
    };
    let analysis = analyze_content_stream(&content_bytes);

    let resources = resources_value.and_then(|value| resolve_dictionary(pdf, value));
    let font_names = resources
        .and_then(|dict| dict.get("Font"))
        .and_then(|value| resolve_dictionary(pdf, value))
        .map(dictionary_keys)
        .unwrap_or_default();
    let xobject_names = resources
        .and_then(|dict| dict.get("XObject"))
        .and_then(|value| resolve_dictionary(pdf, value))
        .map(dictionary_keys)
        .unwrap_or_default();

    for warning in &analysis.warnings {
        findings.push(Finding {
            rule_id: warning.rule_id.clone(),
            severity: Severity::Warning,
            message: format!("Page {} content stream: {}", page_number, warning.message),
            object: Some(page_reference),
            page: Some(page_number),
            byte_offset: None,
            suggested_next_step: warning.suggested_next_step.clone(),
        });
    }

    for font in font_names_used(&analysis) {
        if !font_names.contains(&font) {
            findings.push(Finding {
                rule_id: "page.missing_font_resource".to_string(),
                severity: Severity::Warning,
                message: format!(
                    "Page {} references font /{}, but /{} is not defined in page resources",
                    page_number, font, font
                ),
                object: Some(page_reference),
                page: Some(page_number),
                byte_offset: None,
                suggested_next_step:
                    "Check the page /Resources /Font dictionary and content stream Tf operators"
                        .to_string(),
            });
        }
    }

    for xobject in xobject_names_used(&analysis) {
        if !xobject_names.contains(&xobject) {
            findings.push(Finding {
                rule_id: "page.missing_xobject_resource".to_string(),
                severity: Severity::Warning,
                message: format!(
                    "Page {} invokes XObject /{}, but /{} is not defined in page resources",
                    page_number, xobject, xobject
                ),
                object: Some(page_reference),
                page: Some(page_number),
                byte_offset: None,
                suggested_next_step:
                    "Check the page /Resources /XObject dictionary and Do operators".to_string(),
            });
        }
    }
}

fn page_content_bytes(pdf: &ParsedPdf, page_dictionary: &PdfDictionary) -> Option<Vec<u8>> {
    let contents = page_dictionary.get("Contents")?;
    let mut bytes = Vec::new();
    collect_content_stream_bytes(pdf, contents, &mut bytes);
    (!bytes.is_empty()).then_some(bytes)
}

fn collect_content_stream_bytes(pdf: &ParsedPdf, value: &PdfValue, output: &mut Vec<u8>) {
    match value {
        PdfValue::Reference(reference) => {
            if let Some(stream) = pdf.stream(*reference) {
                let decode = decode_stream(stream);
                if !decode.has_issues() {
                    output.extend_from_slice(&decode.decoded);
                }
            }
        }
        PdfValue::Array(values) => {
            for value in values {
                collect_content_stream_bytes(pdf, value, output);
                output.push(b'\n');
            }
        }
        _ => {}
    }
}

fn resolve_dictionary<'a>(pdf: &'a ParsedPdf, value: &'a PdfValue) -> Option<&'a PdfDictionary> {
    match value {
        PdfValue::Dictionary(dictionary) => Some(dictionary),
        PdfValue::Reference(reference) => pdf.object(*reference)?.dictionary(),
        _ => None,
    }
}

fn dictionary_keys(dictionary: &PdfDictionary) -> BTreeSet<String> {
    dictionary.keys().cloned().collect()
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

fn first_object_with_type(pdf: &ParsedPdf, object_type: &str) -> Option<ObjectRef> {
    pdf.objects
        .values()
        .find(|object| {
            object
                .dictionary()
                .is_some_and(|dict| name_is(dict, "Type", object_type))
        })
        .map(|object| object.reference)
}

fn name_is(dictionary: &PdfDictionary, key: &str, expected: &str) -> bool {
    dictionary
        .get(key)
        .and_then(PdfValue::as_name)
        .is_some_and(|name| name == expected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf_parser::parse_bytes;

    #[test]
    fn detects_missing_reference() {
        let pdf = parse_bytes(missing_reference_pdf().as_bytes(), "missing.pdf");
        let findings = run_diagnostics(&pdf);

        assert!(findings.iter().any(|finding| {
            finding.rule_id == "object.missing_reference"
                && finding.object == Some(ObjectRef::new(99, 0))
        }));
    }

    #[test]
    fn detects_unknown_content_operator() {
        let pdf = parse_bytes(
            page_content_pdf(b"BT /F1 12 Tf (Hi) Tj ZZ ET").as_bytes(),
            "unknown-operator.pdf",
        );
        let findings = run_diagnostics(&pdf);

        assert!(findings.iter().any(|finding| {
            finding.rule_id == "content.unknown_operator"
                && finding.object == Some(ObjectRef::new(3, 0))
                && finding.page == Some(1)
        }));
    }

    fn missing_reference_pdf() -> String {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"%PDF-1.4\n");
        let object_1 = push(
            &mut bytes,
            b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n",
        );
        let object_2 = push(
            &mut bytes,
            b"2 0 obj\n<< /Type /Pages /Kids [99 0 R] /Count 1 >>\nendobj\n",
        );
        let xref = bytes.len();
        bytes.extend_from_slice(b"xref\n0 3\n0000000000 65535 f \n");
        bytes.extend_from_slice(format!("{object_1:010} 00000 n \n").as_bytes());
        bytes.extend_from_slice(format!("{object_2:010} 00000 n \n").as_bytes());
        bytes.extend_from_slice(
            format!("trailer\n<< /Size 3 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes(),
        );
        String::from_utf8(bytes).unwrap()
    }

    fn page_content_pdf(content: &[u8]) -> String {
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
        let mut stream_object = Vec::new();
        stream_object.extend_from_slice(
            format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes(),
        );
        stream_object.extend_from_slice(content);
        stream_object.extend_from_slice(b"\nendstream\nendobj\n");
        let object_4 = push(&mut bytes, &stream_object);
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
