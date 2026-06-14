use crate::pdf_model::{ObjectRef, ParsedPdf, PdfDictionary, PdfObject, PdfValue};
use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjectTreeNodeKind {
    Root,
    Trailer,
    Catalog,
    Pages,
    Page,
    Resources,
    Fonts,
    Font,
    XObjects,
    XObject,
    Images,
    Image,
    Annotations,
    Annotation,
    AcroForm,
    EmbeddedFiles,
    Xref,
    OrphanedObjects,
    Object,
    Reference,
    Cycle,
    Missing,
}

#[derive(Clone, Debug, Serialize)]
pub struct ObjectTreeNode {
    pub label: String,
    pub kind: ObjectTreeNodeKind,
    pub object: Option<ObjectRef>,
    pub summary: Option<String>,
    pub children: Vec<ObjectTreeNode>,
}

impl ObjectTreeNode {
    fn new(label: impl Into<String>, kind: ObjectTreeNodeKind) -> Self {
        Self {
            label: label.into(),
            kind,
            object: None,
            summary: None,
            children: Vec::new(),
        }
    }

    fn object(label: impl Into<String>, kind: ObjectTreeNodeKind, reference: ObjectRef) -> Self {
        Self {
            label: label.into(),
            kind,
            object: Some(reference),
            summary: None,
            children: Vec::new(),
        }
    }

    fn with_summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = Some(summary.into());
        self
    }

    fn with_children(mut self, children: Vec<ObjectTreeNode>) -> Self {
        self.children = children;
        self
    }
}

pub fn build_object_tree(pdf: &ParsedPdf) -> ObjectTreeNode {
    let mut referenced = BTreeSet::new();
    let mut root = ObjectTreeNode::new("PDF", ObjectTreeNodeKind::Root).with_summary(format!(
        "{} object(s), {} stream(s)",
        pdf.metadata.object_count, pdf.metadata.stream_count
    ));

    root.children.push(trailer_node(pdf, &mut referenced));

    if let Some(root_reference) = pdf.metadata.root {
        root.children
            .push(catalog_node(pdf, root_reference, &mut referenced));
    } else {
        root.children.push(ObjectTreeNode::new(
            "Catalog missing",
            ObjectTreeNodeKind::Missing,
        ));
    }

    root.children.push(xref_node(pdf));

    let orphans = orphaned_objects_node(pdf, &referenced);
    if !orphans.children.is_empty() {
        root.children.push(orphans);
    }

    root
}

fn trailer_node(pdf: &ParsedPdf, referenced: &mut BTreeSet<ObjectRef>) -> ObjectTreeNode {
    let mut node = ObjectTreeNode::new("Trailer", ObjectTreeNodeKind::Trailer);
    if let Some(trailer) = &pdf.trailer {
        node.summary = Some(trailer.summary());
        for (key, value) in dictionary_entries(trailer) {
            node.children.push(value_node(key, value, pdf, referenced));
        }
    } else {
        node.summary = Some("missing".to_string());
    }
    node
}

fn catalog_node(
    pdf: &ParsedPdf,
    reference: ObjectRef,
    referenced: &mut BTreeSet<ObjectRef>,
) -> ObjectTreeNode {
    referenced.insert(reference);
    let Some(catalog) = pdf.object(reference) else {
        return ObjectTreeNode::object("Catalog missing", ObjectTreeNodeKind::Missing, reference);
    };
    let mut node = ObjectTreeNode::object("Catalog", ObjectTreeNodeKind::Catalog, reference)
        .with_summary(object_summary(catalog));
    let Some(dictionary) = catalog.dictionary() else {
        return node;
    };

    if let Some(pages_reference) = dictionary.get("Pages").and_then(PdfValue::as_reference) {
        node.children.push(page_tree_node(
            pdf,
            pages_reference,
            referenced,
            &mut BTreeSet::new(),
        ));
    }
    if let Some(acroform) = dictionary.get("AcroForm") {
        node.children.push(named_value_node(
            "AcroForm",
            ObjectTreeNodeKind::AcroForm,
            acroform,
            pdf,
            referenced,
        ));
    }
    if let Some(names) = dictionary.get("Names") {
        add_embedded_files_node(&mut node, names, pdf, referenced);
    }

    node
}

fn page_tree_node(
    pdf: &ParsedPdf,
    reference: ObjectRef,
    referenced: &mut BTreeSet<ObjectRef>,
    visited: &mut BTreeSet<ObjectRef>,
) -> ObjectTreeNode {
    referenced.insert(reference);
    if !visited.insert(reference) {
        return ObjectTreeNode::object("Cycle", ObjectTreeNodeKind::Cycle, reference)
            .with_summary("cyclic page tree reference");
    }

    let Some(object) = pdf.object(reference) else {
        return ObjectTreeNode::object(
            "Missing page tree node",
            ObjectTreeNodeKind::Missing,
            reference,
        );
    };
    let Some(dictionary) = object.dictionary() else {
        return ObjectTreeNode::object(
            "Invalid page tree node",
            ObjectTreeNodeKind::Object,
            reference,
        )
        .with_summary(object_summary(object));
    };

    match dictionary.get("Type").and_then(PdfValue::as_name) {
        Some("Page") => page_node(pdf, reference, dictionary, referenced),
        Some("Pages") | None => {
            let mut node = ObjectTreeNode::object("Pages", ObjectTreeNodeKind::Pages, reference)
                .with_summary(
                    dictionary
                        .get("Count")
                        .map(|count| format!("Count {}", count.summary()))
                        .unwrap_or_else(|| object_summary(object)),
                );
            if let Some(kids) = dictionary.get("Kids").and_then(PdfValue::as_array) {
                for kid in kids {
                    if let Some(kid_reference) = kid.as_reference() {
                        node.children
                            .push(page_tree_node(pdf, kid_reference, referenced, visited));
                    } else {
                        node.children.push(value_node("Kid", kid, pdf, referenced));
                    }
                }
            }
            node
        }
        Some(other) => ObjectTreeNode::object(
            "Unexpected page tree node",
            ObjectTreeNodeKind::Object,
            reference,
        )
        .with_summary(format!("/Type /{other}")),
    }
}

fn page_node(
    pdf: &ParsedPdf,
    reference: ObjectRef,
    dictionary: &PdfDictionary,
    referenced: &mut BTreeSet<ObjectRef>,
) -> ObjectTreeNode {
    let mut node = ObjectTreeNode::object("Page", ObjectTreeNodeKind::Page, reference)
        .with_summary(page_summary(dictionary));

    if let Some(resources) = dictionary.get("Resources") {
        node.children
            .push(resources_node(pdf, resources, referenced));
    }
    if let Some(annotations) = dictionary.get("Annots") {
        node.children
            .push(annotations_node(pdf, annotations, referenced));
    }
    if let Some(contents) = dictionary.get("Contents") {
        node.children.push(named_value_node(
            "Contents",
            ObjectTreeNodeKind::Reference,
            contents,
            pdf,
            referenced,
        ));
    }

    node
}

fn resources_node(
    pdf: &ParsedPdf,
    resources: &PdfValue,
    referenced: &mut BTreeSet<ObjectRef>,
) -> ObjectTreeNode {
    let mut node = named_value_node(
        "Resources",
        ObjectTreeNodeKind::Resources,
        resources,
        pdf,
        referenced,
    );
    if let Some(dictionary) = resolve_dictionary(pdf, resources, referenced) {
        if let Some(fonts) = dictionary.get("Font") {
            node.children.push(resource_map_node(
                "Fonts",
                ObjectTreeNodeKind::Fonts,
                ObjectTreeNodeKind::Font,
                fonts,
                pdf,
                referenced,
            ));
        }
        if let Some(xobjects) = dictionary.get("XObject") {
            let xobject_node = resource_map_node(
                "XObjects",
                ObjectTreeNodeKind::XObjects,
                ObjectTreeNodeKind::XObject,
                xobjects,
                pdf,
                referenced,
            );
            let image_children = xobject_node
                .children
                .iter()
                .filter(|child| child.kind == ObjectTreeNodeKind::Image)
                .cloned()
                .collect::<Vec<_>>();
            node.children.push(xobject_node);
            if !image_children.is_empty() {
                node.children.push(
                    ObjectTreeNode::new("Images", ObjectTreeNodeKind::Images)
                        .with_children(image_children),
                );
            }
        }
    }
    node
}

fn resource_map_node(
    label: &str,
    map_kind: ObjectTreeNodeKind,
    item_kind: ObjectTreeNodeKind,
    value: &PdfValue,
    pdf: &ParsedPdf,
    referenced: &mut BTreeSet<ObjectRef>,
) -> ObjectTreeNode {
    let mut node = named_value_node(label, map_kind, value, pdf, referenced);
    if let Some(dictionary) = resolve_dictionary(pdf, value, referenced) {
        for (name, resource_value) in dictionary {
            let kind = resource_value
                .as_reference()
                .and_then(|reference| pdf.object(reference))
                .and_then(PdfObject::dictionary)
                .and_then(|dict| dict.get("Subtype").and_then(PdfValue::as_name))
                .map(|subtype| {
                    if subtype == "Image" {
                        ObjectTreeNodeKind::Image
                    } else {
                        item_kind.clone()
                    }
                })
                .unwrap_or_else(|| item_kind.clone());
            node.children.push(named_value_node(
                format!("/{name}"),
                kind,
                resource_value,
                pdf,
                referenced,
            ));
        }
    }
    node
}

fn annotations_node(
    pdf: &ParsedPdf,
    annotations: &PdfValue,
    referenced: &mut BTreeSet<ObjectRef>,
) -> ObjectTreeNode {
    let mut node = named_value_node(
        "Annotations",
        ObjectTreeNodeKind::Annotations,
        annotations,
        pdf,
        referenced,
    );
    if let Some(values) = annotations.as_array() {
        for value in values {
            node.children.push(named_value_node(
                "Annotation",
                ObjectTreeNodeKind::Annotation,
                value,
                pdf,
                referenced,
            ));
        }
    }
    node
}

fn add_embedded_files_node(
    catalog: &mut ObjectTreeNode,
    names: &PdfValue,
    pdf: &ParsedPdf,
    referenced: &mut BTreeSet<ObjectRef>,
) {
    if let Some(dictionary) = resolve_dictionary(pdf, names, referenced) {
        if let Some(embedded_files) = dictionary.get("EmbeddedFiles") {
            catalog.children.push(named_value_node(
                "Embedded files",
                ObjectTreeNodeKind::EmbeddedFiles,
                embedded_files,
                pdf,
                referenced,
            ));
        }
    }
}

fn named_value_node(
    label: impl Into<String>,
    kind: ObjectTreeNodeKind,
    value: &PdfValue,
    pdf: &ParsedPdf,
    referenced: &mut BTreeSet<ObjectRef>,
) -> ObjectTreeNode {
    match value {
        PdfValue::Reference(reference) => {
            referenced.insert(*reference);
            let mut node = ObjectTreeNode::object(label, kind, *reference);
            node.summary = pdf
                .object(*reference)
                .map(object_summary)
                .or_else(|| Some("missing reference".to_string()));
            node
        }
        _ => ObjectTreeNode::new(label, kind).with_summary(value.summary()),
    }
}

fn value_node(
    label: impl Into<String>,
    value: &PdfValue,
    pdf: &ParsedPdf,
    referenced: &mut BTreeSet<ObjectRef>,
) -> ObjectTreeNode {
    named_value_node(label, ObjectTreeNodeKind::Reference, value, pdf, referenced)
}

fn xref_node(pdf: &ParsedPdf) -> ObjectTreeNode {
    let invalid_count = pdf
        .xref_entries
        .iter()
        .filter(|entry| entry.in_use && entry.valid_offset == Some(false))
        .count();
    let mut node =
        ObjectTreeNode::new("Cross-reference", ObjectTreeNodeKind::Xref).with_summary(format!(
            "{} entrie(s), {} invalid in-use offset(s)",
            pdf.metadata.xref_entry_count, invalid_count
        ));

    if pdf.metadata.has_xref_stream {
        node.children.push(
            ObjectTreeNode::new("Xref stream detected", ObjectTreeNodeKind::Xref)
                .with_summary("full expansion deferred"),
        );
    }
    for entry in pdf
        .xref_entries
        .iter()
        .filter(|entry| entry.in_use)
        .take(256)
    {
        node.children.push(
            ObjectTreeNode::object(
                entry.reference.to_string(),
                ObjectTreeNodeKind::Object,
                entry.reference,
            )
            .with_summary(format!(
                "offset {}, valid offset: {}",
                entry.offset,
                entry
                    .valid_offset
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "unknown".to_string())
            )),
        );
    }

    node
}

fn orphaned_objects_node(pdf: &ParsedPdf, referenced: &BTreeSet<ObjectRef>) -> ObjectTreeNode {
    let mut node = ObjectTreeNode::new("Orphaned objects", ObjectTreeNodeKind::OrphanedObjects);
    for object in pdf.objects.values() {
        if !referenced.contains(&object.reference) {
            node.children.push(
                ObjectTreeNode::object(
                    object.reference.to_string(),
                    ObjectTreeNodeKind::Object,
                    object.reference,
                )
                .with_summary(object_summary(object)),
            );
        }
    }
    node.summary = Some(format!("{} object(s)", node.children.len()));
    node
}

fn dictionary_entries(value: &PdfValue) -> Vec<(&str, &PdfValue)> {
    value
        .as_dictionary()
        .map(|dictionary| {
            dictionary
                .iter()
                .map(|(key, value)| (key.as_str(), value))
                .collect()
        })
        .unwrap_or_default()
}

fn resolve_dictionary<'a>(
    pdf: &'a ParsedPdf,
    value: &'a PdfValue,
    referenced: &mut BTreeSet<ObjectRef>,
) -> Option<&'a PdfDictionary> {
    match value {
        PdfValue::Dictionary(dictionary) => Some(dictionary),
        PdfValue::Reference(reference) => {
            referenced.insert(*reference);
            pdf.object(*reference)?.dictionary()
        }
        _ => None,
    }
}

fn object_summary(object: &PdfObject) -> String {
    let mut parts = Vec::new();
    if let Some(dictionary) = object.dictionary() {
        if let Some(object_type) = dictionary.get("Type").and_then(PdfValue::as_name) {
            parts.push(format!("/Type /{object_type}"));
        }
        if let Some(subtype) = dictionary.get("Subtype").and_then(PdfValue::as_name) {
            parts.push(format!("/Subtype /{subtype}"));
        }
    }
    if let Some(stream) = &object.stream {
        parts.push(format!("stream {} byte(s)", stream.actual_length));
        if !stream.filters.is_empty() {
            parts.push(format!("filters {}", stream.filters.join(" -> ")));
        }
    }
    if parts.is_empty() {
        object.value.summary()
    } else {
        parts.join(", ")
    }
}

fn page_summary(dictionary: &PdfDictionary) -> String {
    let mut parts = Vec::new();
    if let Some(media_box) = dictionary.get("MediaBox") {
        parts.push(format!("MediaBox {}", media_box.summary()));
    }
    if let Some(crop_box) = dictionary.get("CropBox") {
        parts.push(format!("CropBox {}", crop_box.summary()));
    }
    if let Some(rotation) = dictionary.get("Rotate") {
        parts.push(format!("Rotate {}", rotation.summary()));
    }
    if parts.is_empty() {
        "/Type /Page".to_string()
    } else {
        parts.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf_parser::parse_bytes;

    #[test]
    fn builds_basic_tree_with_required_nodes() {
        let pdf = parse_bytes(minimal_pdf().as_bytes(), "tree.pdf");
        let tree = build_object_tree(&pdf);
        let labels = flatten_labels(&tree);

        assert!(labels.iter().any(|label| label == "Trailer"));
        assert!(labels.iter().any(|label| label == "Catalog"));
        assert!(labels.iter().any(|label| label == "Pages"));
        assert!(labels.iter().any(|label| label == "Page"));
        assert!(labels.iter().any(|label| label == "Resources"));
        assert!(labels.iter().any(|label| label == "Fonts"));
        assert!(labels.iter().any(|label| label == "Cross-reference"));
    }

    fn flatten_labels(node: &ObjectTreeNode) -> Vec<String> {
        let mut labels = vec![node.label.clone()];
        for child in &node.children {
            labels.extend(flatten_labels(child));
        }
        labels
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
