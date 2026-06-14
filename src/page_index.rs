use crate::pdf_model::{ObjectRef, ParsedPdf, PdfDictionary, PdfValue};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct PageIndex {
    pub pages: Vec<PageSummary>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PageSummary {
    pub page_number: u32,
    pub reference: ObjectRef,
    pub media_box: Option<PageBox>,
    pub crop_box: Option<PageBox>,
    pub bleed_box: Option<PageBox>,
    pub trim_box: Option<PageBox>,
    pub art_box: Option<PageBox>,
    pub rotation: Option<i32>,
    pub resources: PageResourceSummary,
    pub links: Vec<PageObjectLink>,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct PageBox {
    pub lower_left_x: f64,
    pub lower_left_y: f64,
    pub upper_right_x: f64,
    pub upper_right_y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct PageResourceSummary {
    pub fonts: usize,
    pub xobjects: usize,
    pub images: usize,
    pub contents: usize,
    pub annotations: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PageObjectLink {
    pub label: String,
    pub kind: PageObjectLinkKind,
    pub reference: ObjectRef,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PageObjectLinkKind {
    Page,
    Parent,
    Resources,
    Contents,
    Font,
    XObject,
    Image,
    Annotation,
}

#[derive(Clone, Default)]
struct InheritedPageAttributes {
    media_box: Option<PageBox>,
    crop_box: Option<PageBox>,
    bleed_box: Option<PageBox>,
    trim_box: Option<PageBox>,
    art_box: Option<PageBox>,
    rotation: Option<i32>,
    resources: Option<PdfValue>,
}

pub fn build_page_index(pdf: &ParsedPdf) -> PageIndex {
    let mut page_index = PageIndex::default();
    let Some(root) = pdf.root_catalog() else {
        return page_index;
    };
    let Some(pages_reference) = root
        .dictionary()
        .and_then(|dict| dict.get("Pages"))
        .and_then(PdfValue::as_reference)
    else {
        return page_index;
    };

    let mut visited = BTreeSet::new();
    walk_page_node(
        pdf,
        pages_reference,
        InheritedPageAttributes::default(),
        &mut visited,
        &mut page_index.pages,
    );
    page_index
}

fn walk_page_node(
    pdf: &ParsedPdf,
    reference: ObjectRef,
    inherited: InheritedPageAttributes,
    visited: &mut BTreeSet<ObjectRef>,
    pages: &mut Vec<PageSummary>,
) {
    if !visited.insert(reference) {
        return;
    }

    let Some(object) = pdf.object(reference) else {
        return;
    };
    let Some(dictionary) = object.dictionary() else {
        return;
    };

    let inherited = inherit_attributes(pdf, dictionary, inherited);
    match dictionary.get("Type").and_then(PdfValue::as_name) {
        Some("Pages") | None => {
            let Some(kids) = dictionary.get("Kids").and_then(PdfValue::as_array) else {
                return;
            };
            for kid in kids {
                if let Some(kid_reference) = kid.as_reference() {
                    walk_page_node(pdf, kid_reference, inherited.clone(), visited, pages);
                }
            }
        }
        Some("Page") => {
            let page_number = pages.len() as u32 + 1;
            pages.push(page_summary(
                pdf,
                page_number,
                reference,
                dictionary,
                &inherited,
            ));
        }
        _ => {}
    }
}

fn inherit_attributes(
    pdf: &ParsedPdf,
    dictionary: &PdfDictionary,
    mut inherited: InheritedPageAttributes,
) -> InheritedPageAttributes {
    if let Some(value) = dictionary.get("MediaBox") {
        inherited.media_box = page_box_from_value(pdf, value);
    }
    if let Some(value) = dictionary.get("CropBox") {
        inherited.crop_box = page_box_from_value(pdf, value);
    }
    if let Some(value) = dictionary.get("BleedBox") {
        inherited.bleed_box = page_box_from_value(pdf, value);
    }
    if let Some(value) = dictionary.get("TrimBox") {
        inherited.trim_box = page_box_from_value(pdf, value);
    }
    if let Some(value) = dictionary.get("ArtBox") {
        inherited.art_box = page_box_from_value(pdf, value);
    }
    if let Some(value) = dictionary.get("Rotate") {
        inherited.rotation = rotation_from_value(pdf, value);
    }
    if let Some(value) = dictionary.get("Resources") {
        inherited.resources = Some(value.clone());
    }
    inherited
}

fn page_summary(
    pdf: &ParsedPdf,
    page_number: u32,
    reference: ObjectRef,
    dictionary: &PdfDictionary,
    inherited: &InheritedPageAttributes,
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
        collect_content_links(contents, &mut links);
        resources.contents = count_references(contents);
    }
    if let Some(annotations) = dictionary.get("Annots") {
        collect_annotation_links(annotations, &mut links);
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
        add_resource_links(pdf, value, &mut resources, &mut links);
    }

    dedupe_links(&mut links);

    PageSummary {
        page_number,
        reference,
        media_box: dictionary
            .get("MediaBox")
            .and_then(|value| page_box_from_value(pdf, value))
            .or(inherited.media_box),
        crop_box: dictionary
            .get("CropBox")
            .and_then(|value| page_box_from_value(pdf, value))
            .or(inherited.crop_box),
        bleed_box: dictionary
            .get("BleedBox")
            .and_then(|value| page_box_from_value(pdf, value))
            .or(inherited.bleed_box),
        trim_box: dictionary
            .get("TrimBox")
            .and_then(|value| page_box_from_value(pdf, value))
            .or(inherited.trim_box),
        art_box: dictionary
            .get("ArtBox")
            .and_then(|value| page_box_from_value(pdf, value))
            .or(inherited.art_box),
        rotation: dictionary
            .get("Rotate")
            .and_then(|value| rotation_from_value(pdf, value))
            .or(inherited.rotation),
        resources,
        links,
    }
}

fn add_resource_links(
    pdf: &ParsedPdf,
    resources_value: &PdfValue,
    resources: &mut PageResourceSummary,
    links: &mut Vec<PageObjectLink>,
) {
    let Some(resource_dictionary) = resolve_dictionary(pdf, resources_value) else {
        return;
    };

    if let Some(fonts) = resource_dictionary.get("Font") {
        if let Some(font_dictionary) = resolve_dictionary(pdf, fonts) {
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

    if let Some(xobjects) = resource_dictionary.get("XObject") {
        if let Some(xobject_dictionary) = resolve_dictionary(pdf, xobjects) {
            resources.xobjects = xobject_dictionary.len();
            for (name, value) in xobject_dictionary {
                let Some(reference) = value.as_reference() else {
                    continue;
                };
                let is_image = pdf
                    .object(reference)
                    .and_then(|object| object.dictionary())
                    .and_then(|dict| dict.get("Subtype"))
                    .and_then(PdfValue::as_name)
                    .is_some_and(|subtype| subtype == "Image");
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

fn collect_content_links(value: &PdfValue, links: &mut Vec<PageObjectLink>) {
    collect_links(value, "Contents", PageObjectLinkKind::Contents, links);
}

fn collect_annotation_links(value: &PdfValue, links: &mut Vec<PageObjectLink>) {
    collect_links(value, "Annotation", PageObjectLinkKind::Annotation, links);
}

fn collect_links(
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
                collect_links(value, label, kind.clone(), links);
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

fn dedupe_links(links: &mut Vec<PageObjectLink>) {
    let mut seen = BTreeSet::new();
    links.retain(|link| {
        seen.insert((
            link.kind_label().to_string(),
            link.label.clone(),
            link.reference,
        ))
    });
}

impl PageObjectLink {
    fn kind_label(&self) -> &'static str {
        match self.kind {
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
}

fn page_box_from_value(pdf: &ParsedPdf, value: &PdfValue) -> Option<PageBox> {
    match value {
        PdfValue::Array(values) => page_box_from_array(values),
        PdfValue::Reference(reference) => page_box_from_value(pdf, &pdf.object(*reference)?.value),
        _ => None,
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

fn rotation_from_value(pdf: &ParsedPdf, value: &PdfValue) -> Option<i32> {
    match value {
        PdfValue::Number(value) if value.fract() == 0.0 => Some(*value as i32),
        PdfValue::Reference(reference) => rotation_from_value(pdf, &pdf.object(*reference)?.value),
        _ => None,
    }
}

fn number(value: &PdfValue) -> Option<f64> {
    match value {
        PdfValue::Number(value) => Some(*value),
        _ => None,
    }
}

fn resolve_dictionary<'a>(pdf: &'a ParsedPdf, value: &'a PdfValue) -> Option<&'a PdfDictionary> {
    match value {
        PdfValue::Dictionary(dictionary) => Some(dictionary),
        PdfValue::Reference(reference) => pdf.object(*reference)?.dictionary(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf_parser::parse_bytes;

    #[test]
    fn builds_page_index_with_inherited_boxes_and_resources() {
        let pdf = parse_bytes(page_pdf().as_bytes(), "pages.pdf");
        let index = build_page_index(&pdf);

        assert_eq!(index.pages.len(), 1);
        let page = &index.pages[0];
        assert_eq!(page.page_number, 1);
        assert_eq!(page.reference, ObjectRef::new(3, 0));
        assert_eq!(page.media_box.unwrap().width, 200.0);
        assert_eq!(page.media_box.unwrap().height, 300.0);
        assert_eq!(page.crop_box.unwrap().lower_left_x, 10.0);
        assert_eq!(page.bleed_box.unwrap().upper_right_y, 290.0);
        assert_eq!(page.rotation, Some(90));
        assert_eq!(page.resources.fonts, 1);
        assert_eq!(page.resources.xobjects, 1);
        assert_eq!(page.resources.images, 1);
        assert_eq!(page.resources.contents, 1);
        assert_eq!(page.resources.annotations, 1);
        assert!(page
            .links
            .iter()
            .any(|link| link.kind == PageObjectLinkKind::Image
                && link.reference == ObjectRef::new(6, 0)));
    }

    fn page_pdf() -> String {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"%PDF-1.4\n");
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
        String::from_utf8(bytes).unwrap()
    }

    fn push(bytes: &mut Vec<u8>, value: &[u8]) -> usize {
        let offset = bytes.len();
        bytes.extend_from_slice(value);
        offset
    }
}
