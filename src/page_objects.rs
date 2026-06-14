use crate::content_ops::{analyze_content_stream, ContentOperator, ContentToken, ContentTokenKind};
use crate::page_index::{build_page_index, PageBox, PageSummary};
use crate::pdf_model::{ByteRange, ObjectRef, ParsedPdf, PdfDictionary, PdfValue};
use crate::stream_decode::decode_stream;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

const MAX_FORM_DEPTH: usize = 4;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageObjectInspection {
    pub page_number: u32,
    pub page_reference: ObjectRef,
    pub page_box: Option<PageBox>,
    pub objects: Vec<PageObject>,
    pub warnings: Vec<PageObjectWarning>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PageObjectKind {
    Text,
    Path,
    Image,
    Form,
    XObject,
    Annotation,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageObject {
    pub id: String,
    pub kind: PageObjectKind,
    pub label: String,
    pub summary: String,
    pub bbox: Option<PageObjectBounds>,
    pub reference: Option<ObjectRef>,
    pub content_stream: Option<ObjectRef>,
    pub byte_range: Option<ByteRange>,
    pub properties: Vec<PageObjectProperty>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageObjectBounds {
    pub coordinate_space: &'static str,
    pub lower_left_x: f64,
    pub lower_left_y: f64,
    pub upper_right_x: f64,
    pub upper_right_y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageObjectProperty {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageObjectWarning {
    pub rule_id: String,
    pub object_id: Option<String>,
    pub source: Option<ObjectRef>,
    pub message: String,
}

pub fn inspect_page_objects(pdf: &ParsedPdf, page_number: u32) -> Option<PageObjectInspection> {
    let page_index = build_page_index(pdf);
    let page = page_index
        .pages
        .iter()
        .find(|page| page.page_number == page_number)?;
    let page_object = pdf.object(page.reference)?;
    let page_dictionary = page_object.dictionary()?;
    let mut inspector = PageObjectInspector::new(pdf, page);

    inspector.extract_annotations(page_dictionary);
    inspector.extract_content_streams(page_dictionary);

    Some(PageObjectInspection {
        page_number,
        page_reference: page.reference,
        page_box: page.media_box,
        objects: inspector.objects,
        warnings: inspector.warnings,
    })
}

struct PageObjectInspector<'a> {
    pdf: &'a ParsedPdf,
    page: &'a PageSummary,
    objects: Vec<PageObject>,
    warnings: Vec<PageObjectWarning>,
    next_id: usize,
    resources: ResourceContext,
}

impl<'a> PageObjectInspector<'a> {
    fn new(pdf: &'a ParsedPdf, page: &'a PageSummary) -> Self {
        let page_dictionary = pdf
            .object(page.reference)
            .and_then(|object| object.dictionary());
        let resources = page_dictionary
            .and_then(|dictionary| inherited_resources(pdf, dictionary))
            .as_ref()
            .map(|value| resource_context_from_value(pdf, value))
            .unwrap_or_default();

        Self {
            pdf,
            page,
            objects: Vec::new(),
            warnings: Vec::new(),
            next_id: 1,
            resources,
        }
    }

    fn extract_annotations(&mut self, page_dictionary: &PdfDictionary) {
        let Some(annotations) = page_dictionary.get("Annots") else {
            return;
        };

        for reference in collect_direct_references(annotations) {
            let Some(annotation) = self.pdf.object(reference) else {
                self.warn(
                    "page_object.missing_annotation",
                    None,
                    Some(reference),
                    format!("Annotation object {reference} was referenced by the page but was not found"),
                );
                continue;
            };
            let Some(dictionary) = annotation.dictionary() else {
                self.warn(
                    "page_object.invalid_annotation",
                    None,
                    Some(reference),
                    format!("Annotation object {reference} does not contain a dictionary"),
                );
                continue;
            };

            let subtype = dictionary
                .get("Subtype")
                .and_then(PdfValue::as_name)
                .unwrap_or("Unknown");
            let bbox = dictionary.get("Rect").and_then(bounds_from_pdf_value);
            let id = self.id("annotation");
            let mut warnings = Vec::new();
            if bbox.is_none() {
                warnings.push("Annotation has no usable /Rect bbox.".to_string());
            }

            self.objects.push(PageObject {
                id,
                kind: PageObjectKind::Annotation,
                label: format!("Annotation /{subtype}"),
                summary: format!("Annotation {reference} /{subtype}"),
                bbox,
                reference: Some(reference),
                content_stream: None,
                byte_range: None,
                properties: vec![
                    prop("Subtype", format!("/{subtype}")),
                    prop("Reference", reference.to_string()),
                    prop(
                        "Rect",
                        dictionary
                            .get("Rect")
                            .map(PdfValue::summary)
                            .unwrap_or_else(|| "-".to_string()),
                    ),
                    prop(
                        "Flags",
                        dictionary
                            .get("F")
                            .map(PdfValue::summary)
                            .unwrap_or_else(|| "-".to_string()),
                    ),
                ],
                warnings,
            });
        }
    }

    fn extract_content_streams(&mut self, page_dictionary: &PdfDictionary) {
        let Some(contents) = page_dictionary.get("Contents") else {
            self.warn(
                "page_object.no_contents",
                None,
                Some(self.page.reference),
                format!("Page {} has no /Contents entry", self.page.page_number),
            );
            return;
        };
        let references = collect_references(self.pdf, contents);
        if references.is_empty() {
            self.warn(
                "page_object.unsupported_contents",
                None,
                Some(self.page.reference),
                "Page /Contents could not be resolved to stream references".to_string(),
            );
            return;
        }

        for reference in references {
            let Some(stream) = self.pdf.stream(reference) else {
                self.warn(
                    "page_object.content_not_stream",
                    None,
                    Some(reference),
                    format!("Page content object {reference} is not a stream"),
                );
                continue;
            };
            let decode = decode_stream(stream);
            if decode.has_issues() {
                let message = decode
                    .issues
                    .iter()
                    .map(|issue| format!("/{}: {}", issue.filter, issue.message))
                    .collect::<Vec<_>>()
                    .join("; ");
                self.warn(
                    "page_object.content_decode_failed",
                    None,
                    Some(reference),
                    if message.is_empty() {
                        format!("Content stream {reference} could not be decoded")
                    } else {
                        format!("Content stream {reference} decode issue: {message}")
                    },
                );
                continue;
            }

            self.extract_decoded_content_stream(
                reference,
                &decode.decoded,
                ExtractionContext::page(self.resources.clone()),
            );
        }
    }

    fn extract_decoded_content_stream(
        &mut self,
        content_stream: ObjectRef,
        decoded: &[u8],
        context: ExtractionContext,
    ) {
        let analysis = analyze_content_stream(decoded);
        let mut state = GraphicsState {
            ctm: context.initial_ctm,
            ..GraphicsState::default()
        };
        let mut stack = Vec::new();
        let mut path = ActivePath::default();

        for operator in &analysis.operators {
            self.apply_operator(
                content_stream,
                operator,
                &mut state,
                &mut stack,
                &mut path,
                &context,
            );
        }
    }

    fn apply_operator(
        &mut self,
        content_stream: ObjectRef,
        operator: &ContentOperator,
        state: &mut GraphicsState,
        stack: &mut Vec<GraphicsState>,
        path: &mut ActivePath,
        context: &ExtractionContext,
    ) {
        match operator.name.as_str() {
            "q" => stack.push(state.clone()),
            "Q" => {
                if let Some(previous) = stack.pop() {
                    *state = previous;
                }
            }
            "cm" => {
                if let Some(matrix) = matrix_from_operands(operator) {
                    state.ctm = state.ctm.multiply(matrix);
                }
            }
            "Tc" => {
                if let Some(value) = number_operand(operator, 0) {
                    state.character_spacing = value;
                }
            }
            "Tw" => {
                if let Some(value) = number_operand(operator, 0) {
                    state.word_spacing = value;
                }
            }
            "Tz" => {
                if let Some(value) = number_operand(operator, 0) {
                    state.horizontal_scaling = value;
                }
            }
            "TL" => {
                if let Some(value) = number_operand(operator, 0) {
                    state.leading = value;
                }
            }
            "BT" => {
                state.text_matrix = Matrix::identity();
                state.line_matrix = Matrix::identity();
            }
            "Tf" => {
                if let Some(name) = name_operand(operator, 0) {
                    state.font_name = Some(name.to_string());
                }
                if let Some(size) = number_operand(operator, 1) {
                    state.font_size = Some(size);
                }
            }
            "Tm" => {
                if let Some(matrix) = matrix_from_operands(operator) {
                    state.text_matrix = matrix;
                    state.line_matrix = matrix;
                }
            }
            "Td" | "TD" => {
                if let (Some(tx), Some(ty)) =
                    (number_operand(operator, 0), number_operand(operator, 1))
                {
                    if operator.name == "TD" {
                        state.leading = -ty;
                    }
                    state.text_matrix = state.text_matrix.translate(tx, ty);
                    state.line_matrix = state.text_matrix;
                }
            }
            "T*" => {
                state.text_matrix = state.line_matrix.translate(0.0, -state.leading);
                state.line_matrix = state.text_matrix;
            }
            "'" => {
                state.text_matrix = state.line_matrix.translate(0.0, -state.leading);
                state.line_matrix = state.text_matrix;
                self.add_text_object(content_stream, operator, state, context);
            }
            "\"" => {
                if let Some(word_spacing) = number_operand(operator, 0) {
                    state.word_spacing = word_spacing;
                }
                if let Some(character_spacing) = number_operand(operator, 1) {
                    state.character_spacing = character_spacing;
                }
                state.text_matrix = state.line_matrix.translate(0.0, -state.leading);
                state.line_matrix = state.text_matrix;
                self.add_text_object(content_stream, operator, state, context);
            }
            "Tj" | "TJ" => {
                self.add_text_object(content_stream, operator, state, context);
            }
            "m" => path.move_to(operator, state.ctm),
            "l" => path.line_to(operator, state.ctm),
            "c" | "v" | "y" => path.curve_to(operator, state.ctm),
            "re" => path.rectangle(operator, state.ctm),
            "h" => path.close(),
            "S" | "s" | "f" | "F" | "f*" | "B" | "B*" | "b" | "b*" | "n" => {
                self.add_path_object(content_stream, operator, path, context);
                path.clear();
            }
            "Do" => self.add_xobject_object(content_stream, operator, state, context),
            _ => {}
        }
    }

    fn add_text_object(
        &mut self,
        content_stream: ObjectRef,
        operator: &ContentOperator,
        state: &mut GraphicsState,
        context: &ExtractionContext,
    ) {
        let font = state
            .font_name
            .as_deref()
            .and_then(|name| context.resources.fonts.get(name));
        let decoded = decode_text_operator(operator, font);
        let text = decoded.text;
        if text.is_empty() {
            return;
        }

        let font_size = state.font_size.unwrap_or(12.0).abs().max(1.0);
        let text_width = text_advance_width(
            operator,
            &decoded.glyph_bytes,
            &text,
            font,
            state,
            font_size,
        );
        let text_matrix = state.ctm.multiply(state.text_matrix);
        let bbox = bounds_from_points(&[
            text_matrix.transform_point(0.0, 0.0),
            text_matrix.transform_point(text_width, 0.0),
            text_matrix.transform_point(0.0, font_size),
            text_matrix.transform_point(text_width, font_size),
        ]);
        let id = self.id("text");
        let snippet = truncate(&text, 80);
        let mut warnings = decoded.warnings;
        if font.is_none() {
            warnings.push("Active font resource was not found; text decoding and width use fallback byte mapping.".to_string());
        }
        warnings.push(
            "Text bbox remains best-effort; full shaping, kerning, vertical writing, and renderer glyph rasterization are not simulated."
                .to_string(),
        );
        let mut properties = vec![
            prop("Text", snippet.clone()),
            prop("Raw glyph bytes", hex_bytes(&decoded.glyph_bytes)),
            prop(
                "Font resource",
                state.font_name.clone().unwrap_or_else(|| "-".to_string()),
            ),
            prop(
                "Font reference",
                font.and_then(|font| font.reference.map(|reference| reference.to_string()))
                    .unwrap_or_else(|| "-".to_string()),
            ),
            prop(
                "Base font",
                font.and_then(|font| font.base_font.clone())
                    .unwrap_or_else(|| "-".to_string()),
            ),
            prop(
                "Font subtype",
                font.and_then(|font| font.subtype.clone())
                    .unwrap_or_else(|| "-".to_string()),
            ),
            prop(
                "Encoding source",
                font.map(|font| font.encoding_source.clone())
                    .unwrap_or_else(|| "fallback".to_string()),
            ),
            prop(
                "Width source",
                font.map(|font| font.width_source.clone())
                    .unwrap_or_else(|| "fallback".to_string()),
            ),
            prop("Font size", format_number(font_size)),
            prop(
                "Horizontal scaling",
                format!("{}%", format_number(state.horizontal_scaling)),
            ),
            prop("Character spacing", format_number(state.character_spacing)),
            prop("Word spacing", format_number(state.word_spacing)),
            prop("Estimated advance", format_number(text_width)),
            prop("Text matrix", state.text_matrix.to_string()),
            prop("CTM", state.ctm.to_string()),
            prop("Operator", operator.name.clone()),
            prop("Byte range", format_range(operator.byte_range)),
        ];
        add_context_properties(&mut properties, context);

        self.objects.push(PageObject {
            id,
            kind: PageObjectKind::Text,
            label: format!("Text: {snippet}"),
            summary: format!("Text run from operator {}", operator.name),
            bbox,
            reference: None,
            content_stream: Some(content_stream),
            byte_range: Some(operator.byte_range),
            properties,
            warnings,
        });

        state.text_matrix = state.text_matrix.translate(text_width, 0.0);
    }

    fn add_path_object(
        &mut self,
        content_stream: ObjectRef,
        operator: &ContentOperator,
        path: &ActivePath,
        context: &ExtractionContext,
    ) {
        if path.points.is_empty() {
            return;
        }
        let id = self.id("path");
        let bbox = bounds_from_points(&path.points);
        let mut properties = vec![
            prop("Paint operator", operator.name.clone()),
            prop("Command count", path.command_count.to_string()),
            prop("Point count", path.points.len().to_string()),
            prop("Byte range", format_range(operator.byte_range)),
        ];
        add_context_properties(&mut properties, context);

        self.objects.push(PageObject {
            id,
            kind: PageObjectKind::Path,
            label: format!("Path: {} point(s)", path.points.len()),
            summary: format!("Path painted with {}", operator.name),
            bbox,
            reference: None,
            content_stream: Some(content_stream),
            byte_range: Some(operator.byte_range),
            properties,
            warnings: Vec::new(),
        });
    }

    fn add_xobject_object(
        &mut self,
        content_stream: ObjectRef,
        operator: &ContentOperator,
        state: &GraphicsState,
        context: &ExtractionContext,
    ) {
        let Some(name) = name_operand(operator, operator.operands.len().saturating_sub(1)) else {
            return;
        };
        let reference = context.resources.xobjects.get(name).copied();
        let mut warnings = Vec::new();
        let Some(reference) = reference else {
            self.warn(
                "page_object.missing_xobject_resource",
                None,
                Some(content_stream),
                format!("XObject /{name} was invoked but was not found in page resources"),
            );
            return;
        };
        let dictionary = self
            .pdf
            .object(reference)
            .and_then(|object| object.dictionary());
        let subtype = dictionary
            .and_then(|dictionary| dictionary.get("Subtype"))
            .and_then(PdfValue::as_name)
            .unwrap_or("Unknown");
        let kind = match subtype {
            "Image" => PageObjectKind::Image,
            "Form" => PageObjectKind::Form,
            _ => PageObjectKind::XObject,
        };
        if kind == PageObjectKind::Form {
            warnings.push(
                "Form XObject bbox uses invocation CTM unit bounds; supported child content is expanded as separate page objects."
                    .to_string(),
            );
        }
        let form_matrix = dictionary
            .and_then(|dictionary| dictionary.get("Matrix"))
            .and_then(matrix_from_pdf_value)
            .unwrap_or_else(Matrix::identity);
        let bbox = if subtype == "Form" {
            dictionary
                .and_then(|dictionary| dictionary.get("BBox"))
                .and_then(|value| {
                    transformed_bounds_from_pdf_value(value, state.ctm.multiply(form_matrix))
                })
        } else {
            bounds_from_points(&[
                state.ctm.transform_point(0.0, 0.0),
                state.ctm.transform_point(1.0, 0.0),
                state.ctm.transform_point(0.0, 1.0),
                state.ctm.transform_point(1.0, 1.0),
            ])
        };
        let id = self.id("xobject");
        let image_width = dictionary
            .and_then(|dictionary| dictionary.get("Width"))
            .and_then(number_from_pdf_value);
        let image_height = dictionary
            .and_then(|dictionary| dictionary.get("Height"))
            .and_then(number_from_pdf_value);
        let mut properties = vec![
            prop("XObject name", format!("/{name}")),
            prop("Subtype", format!("/{subtype}")),
            prop("Reference", reference.to_string()),
            prop(
                "Width",
                image_width
                    .map(format_number)
                    .unwrap_or_else(|| "-".to_string()),
            ),
            prop(
                "Height",
                image_height
                    .map(format_number)
                    .unwrap_or_else(|| "-".to_string()),
            ),
            prop(
                "ColorSpace",
                dictionary
                    .and_then(|dictionary| dictionary.get("ColorSpace"))
                    .map(PdfValue::summary)
                    .unwrap_or_else(|| "-".to_string()),
            ),
            prop(
                "BitsPerComponent",
                dictionary
                    .and_then(|dictionary| dictionary.get("BitsPerComponent"))
                    .map(PdfValue::summary)
                    .unwrap_or_else(|| "-".to_string()),
            ),
            prop("CTM", state.ctm.to_string()),
            prop("Byte range", format_range(operator.byte_range)),
        ];
        add_context_properties(&mut properties, context);

        self.objects.push(PageObject {
            id,
            kind,
            label: format!("/{name} {subtype}"),
            summary: format!("Paint XObject /{name} ({reference})"),
            bbox,
            reference: Some(reference),
            content_stream: Some(content_stream),
            byte_range: Some(operator.byte_range),
            properties,
            warnings,
        });

        if subtype == "Form" {
            self.expand_form_xobject(content_stream, name, reference, state, context);
        }
    }

    fn expand_form_xobject(
        &mut self,
        parent_stream: ObjectRef,
        name: &str,
        reference: ObjectRef,
        state: &GraphicsState,
        context: &ExtractionContext,
    ) {
        if context.form_depth >= MAX_FORM_DEPTH {
            self.warn(
                "page_object.form_recursion_limit",
                None,
                Some(reference),
                format!("Form XObject /{name} reached recursion depth limit {MAX_FORM_DEPTH}"),
            );
            return;
        }
        if context.form_stack.contains(&reference) {
            self.warn(
                "page_object.form_recursion_cycle",
                None,
                Some(reference),
                format!(
                    "Form XObject /{name} references an ancestor form; recursive expansion stopped"
                ),
            );
            return;
        }

        let Some(stream) = self.pdf.stream(reference) else {
            self.warn(
                "page_object.form_not_stream",
                None,
                Some(reference),
                format!("Form XObject /{name} at {reference} is not a stream"),
            );
            return;
        };
        let decode = decode_stream(stream);
        if decode.has_issues() {
            let message = decode
                .issues
                .iter()
                .map(|issue| format!("/{}: {}", issue.filter, issue.message))
                .collect::<Vec<_>>()
                .join("; ");
            self.warn(
                "page_object.form_decode_failed",
                None,
                Some(reference),
                if message.is_empty() {
                    format!("Form XObject /{name} at {reference} could not be decoded")
                } else {
                    format!("Form XObject /{name} at {reference} decode issue: {message}")
                },
            );
            return;
        }

        let form_matrix = stream
            .dictionary
            .get("Matrix")
            .and_then(matrix_from_pdf_value)
            .unwrap_or_else(Matrix::identity);
        let form_resources = stream
            .dictionary
            .get("Resources")
            .map(|value| resource_context_from_value(self.pdf, value));
        let mut resources = context.resources.clone();
        if let Some(form_resources) = form_resources {
            resources = resources.overlay(form_resources);
        }

        let mut form_stack = context.form_stack.clone();
        form_stack.insert(reference);
        let child_context = ExtractionContext {
            resources,
            initial_ctm: state.ctm.multiply(form_matrix),
            parent_form: Some(FormContext {
                name: name.to_string(),
                reference,
                depth: context.form_depth + 1,
                invoked_from: parent_stream,
                bbox: stream
                    .dictionary
                    .get("BBox")
                    .and_then(bounds_from_pdf_value),
            }),
            form_stack,
            form_depth: context.form_depth + 1,
        };
        self.extract_decoded_content_stream(reference, &decode.decoded, child_context);
    }

    fn id(&mut self, prefix: &str) -> String {
        let id = format!("page-{}-{prefix}-{}", self.page.page_number, self.next_id);
        self.next_id += 1;
        id
    }

    fn warn(
        &mut self,
        rule_id: &str,
        object_id: Option<String>,
        source: Option<ObjectRef>,
        message: String,
    ) {
        self.warnings.push(PageObjectWarning {
            rule_id: rule_id.to_string(),
            object_id,
            source,
            message,
        });
    }
}

#[derive(Clone, Debug)]
struct GraphicsState {
    ctm: Matrix,
    text_matrix: Matrix,
    line_matrix: Matrix,
    font_name: Option<String>,
    font_size: Option<f64>,
    leading: f64,
    character_spacing: f64,
    word_spacing: f64,
    horizontal_scaling: f64,
}

impl Default for GraphicsState {
    fn default() -> Self {
        Self {
            ctm: Matrix::identity(),
            text_matrix: Matrix::identity(),
            line_matrix: Matrix::identity(),
            font_name: None,
            font_size: None,
            leading: 0.0,
            character_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 100.0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Matrix {
    a: f64,
    b: f64,
    c: f64,
    d: f64,
    e: f64,
    f: f64,
}

impl Matrix {
    fn identity() -> Self {
        Self {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: 0.0,
            f: 0.0,
        }
    }

    fn from_values(values: &[f64]) -> Option<Self> {
        if values.len() < 6 {
            return None;
        }
        Some(Self {
            a: values[0],
            b: values[1],
            c: values[2],
            d: values[3],
            e: values[4],
            f: values[5],
        })
    }

    fn multiply(self, other: Self) -> Self {
        Self {
            a: self.a * other.a + self.b * other.c,
            b: self.a * other.b + self.b * other.d,
            c: self.c * other.a + self.d * other.c,
            d: self.c * other.b + self.d * other.d,
            e: self.e * other.a + self.f * other.c + other.e,
            f: self.e * other.b + self.f * other.d + other.f,
        }
    }

    fn translate(self, tx: f64, ty: f64) -> Self {
        self.multiply(Self {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: tx,
            f: ty,
        })
    }

    fn transform_point(self, x: f64, y: f64) -> (f64, f64) {
        (
            x * self.a + y * self.c + self.e,
            x * self.b + y * self.d + self.f,
        )
    }
}

impl std::fmt::Display for Matrix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{} {} {} {} {} {}]",
            format_number(self.a),
            format_number(self.b),
            format_number(self.c),
            format_number(self.d),
            format_number(self.e),
            format_number(self.f)
        )
    }
}

#[derive(Default)]
struct ActivePath {
    points: Vec<(f64, f64)>,
    command_count: usize,
}

impl ActivePath {
    fn move_to(&mut self, operator: &ContentOperator, ctm: Matrix) {
        self.add_last_pairs(operator, ctm, 1);
    }

    fn line_to(&mut self, operator: &ContentOperator, ctm: Matrix) {
        self.add_last_pairs(operator, ctm, 1);
    }

    fn curve_to(&mut self, operator: &ContentOperator, ctm: Matrix) {
        self.add_last_pairs(operator, ctm, 3);
    }

    fn rectangle(&mut self, operator: &ContentOperator, ctm: Matrix) {
        let values = number_operands(operator);
        if values.len() < 4 {
            return;
        }
        let (x, y, width, height) = (values[0], values[1], values[2], values[3]);
        self.command_count += 1;
        self.points.extend([
            ctm.transform_point(x, y),
            ctm.transform_point(x + width, y),
            ctm.transform_point(x, y + height),
            ctm.transform_point(x + width, y + height),
        ]);
    }

    fn close(&mut self) {
        self.command_count += 1;
    }

    fn clear(&mut self) {
        self.points.clear();
        self.command_count = 0;
    }

    fn add_last_pairs(&mut self, operator: &ContentOperator, ctm: Matrix, count: usize) {
        let values = number_operands(operator);
        if values.len() < count * 2 {
            return;
        }
        self.command_count += 1;
        for pair in values[values.len() - count * 2..].chunks(2) {
            self.points.push(ctm.transform_point(pair[0], pair[1]));
        }
    }
}

#[derive(Clone, Debug, Default)]
struct ResourceContext {
    fonts: BTreeMap<String, FontResource>,
    xobjects: BTreeMap<String, ObjectRef>,
}

impl ResourceContext {
    fn overlay(mut self, other: Self) -> Self {
        self.fonts.extend(other.fonts);
        self.xobjects.extend(other.xobjects);
        self
    }
}

#[derive(Clone, Debug)]
struct FontResource {
    reference: Option<ObjectRef>,
    subtype: Option<String>,
    base_font: Option<String>,
    encoding: BTreeMap<u16, String>,
    encoding_source: String,
    widths: BTreeMap<u16, f64>,
    missing_width: Option<f64>,
    width_source: String,
    to_unicode: BTreeMap<Vec<u8>, String>,
}

#[derive(Clone, Debug)]
struct ExtractionContext {
    resources: ResourceContext,
    initial_ctm: Matrix,
    parent_form: Option<FormContext>,
    form_stack: BTreeSet<ObjectRef>,
    form_depth: usize,
}

impl ExtractionContext {
    fn page(resources: ResourceContext) -> Self {
        Self {
            resources,
            initial_ctm: Matrix::identity(),
            parent_form: None,
            form_stack: BTreeSet::new(),
            form_depth: 0,
        }
    }
}

#[derive(Clone, Debug)]
struct FormContext {
    name: String,
    reference: ObjectRef,
    depth: usize,
    invoked_from: ObjectRef,
    bbox: Option<PageObjectBounds>,
}

struct DecodedText {
    text: String,
    glyph_bytes: Vec<u8>,
    warnings: Vec<String>,
}

fn inherited_resources(pdf: &ParsedPdf, page_dictionary: &PdfDictionary) -> Option<PdfValue> {
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

fn resource_context_from_value(pdf: &ParsedPdf, value: &PdfValue) -> ResourceContext {
    let Some(dictionary) = resolve_dictionary(pdf, value) else {
        return ResourceContext::default();
    };

    let fonts = dictionary
        .get("Font")
        .and_then(|value| resolve_dictionary(pdf, value))
        .map(|dictionary| {
            dictionary
                .iter()
                .filter_map(|(name, value)| {
                    let (reference, font_dictionary) = match value {
                        PdfValue::Reference(reference) => {
                            (*reference, pdf.object(*reference)?.dictionary()?)
                        }
                        PdfValue::Dictionary(dictionary) => (ObjectRef::new(0, 0), dictionary),
                        _ => return None,
                    };
                    let font = font_resource_from_dictionary(
                        pdf,
                        font_dictionary,
                        matches!(value, PdfValue::Reference(_)).then_some(reference),
                    );
                    Some((name.clone(), font))
                })
                .collect()
        })
        .unwrap_or_default();

    let xobjects = dictionary
        .get("XObject")
        .and_then(|value| resolve_dictionary(pdf, value))
        .map(resource_reference_map)
        .unwrap_or_default();

    ResourceContext { fonts, xobjects }
}

fn font_resource_from_dictionary(
    pdf: &ParsedPdf,
    dictionary: &PdfDictionary,
    reference: Option<ObjectRef>,
) -> FontResource {
    let subtype = dictionary
        .get("Subtype")
        .and_then(PdfValue::as_name)
        .map(ToString::to_string);
    let base_font = dictionary
        .get("BaseFont")
        .and_then(PdfValue::as_name)
        .map(ToString::to_string);
    let (encoding, encoding_source) = font_encoding(pdf, dictionary);
    let (widths, missing_width, width_source) = font_widths(pdf, dictionary);
    let to_unicode = dictionary
        .get("ToUnicode")
        .and_then(PdfValue::as_reference)
        .and_then(|reference| pdf.stream(reference))
        .map(decode_stream)
        .and_then(|decode| (!decode.has_issues()).then(|| parse_to_unicode_cmap(&decode.decoded)))
        .unwrap_or_default();

    FontResource {
        reference,
        subtype,
        base_font,
        encoding,
        encoding_source,
        widths,
        missing_width,
        width_source,
        to_unicode,
    }
}

fn font_encoding(pdf: &ParsedPdf, dictionary: &PdfDictionary) -> (BTreeMap<u16, String>, String) {
    let mut encoding = builtin_encoding("StandardEncoding");
    let mut source = "StandardEncoding fallback".to_string();

    if let Some(value) = dictionary.get("Encoding") {
        match value {
            PdfValue::Name(name) => {
                encoding = builtin_encoding(name);
                source = format!("/{name}");
            }
            PdfValue::Dictionary(dictionary) => {
                if let Some(base) = dictionary.get("BaseEncoding").and_then(PdfValue::as_name) {
                    encoding = builtin_encoding(base);
                    source = format!("/{base} with Differences");
                } else {
                    source = "Encoding dictionary Differences".to_string();
                }
                apply_encoding_differences(&mut encoding, dictionary.get("Differences"));
            }
            PdfValue::Reference(reference) => {
                if let Some(dictionary) = pdf
                    .object(*reference)
                    .and_then(|object| object.dictionary())
                {
                    if let Some(base) = dictionary.get("BaseEncoding").and_then(PdfValue::as_name) {
                        encoding = builtin_encoding(base);
                        source = format!("{reference} /{base} with Differences");
                    } else {
                        source = format!("{reference} Differences");
                    }
                    apply_encoding_differences(&mut encoding, dictionary.get("Differences"));
                }
            }
            _ => {}
        }
    }

    (encoding, source)
}

fn apply_encoding_differences(
    encoding: &mut BTreeMap<u16, String>,
    differences: Option<&PdfValue>,
) {
    let Some(values) = differences.and_then(PdfValue::as_array) else {
        return;
    };
    let mut code = None::<u16>;
    for value in values {
        match value {
            PdfValue::Number(number) if *number >= 0.0 && *number <= u16::MAX as f64 => {
                code = Some(*number as u16);
            }
            PdfValue::Name(name) => {
                if let Some(current) = code {
                    encoding.insert(current, glyph_name_to_unicode(name));
                    code = current.checked_add(1);
                }
            }
            _ => {}
        }
    }
}

fn font_widths(
    pdf: &ParsedPdf,
    dictionary: &PdfDictionary,
) -> (BTreeMap<u16, f64>, Option<f64>, String) {
    let first_char = dictionary
        .get("FirstChar")
        .and_then(number_from_pdf_value)
        .unwrap_or(0.0)
        .max(0.0) as u16;
    let widths = dictionary
        .get("Widths")
        .and_then(|value| resolve_array(pdf, value))
        .map(|values| {
            values
                .iter()
                .enumerate()
                .filter_map(|(index, value)| {
                    number_from_pdf_value(value)
                        .map(|width| (first_char.saturating_add(index as u16), width))
                })
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default();
    let missing_width = dictionary
        .get("FontDescriptor")
        .and_then(|value| resolve_dictionary(pdf, value))
        .and_then(|dictionary| dictionary.get("MissingWidth"))
        .and_then(number_from_pdf_value);
    let source = if widths.is_empty() {
        "fallback".to_string()
    } else {
        format!("/Widths from FirstChar {first_char}")
    };
    (widths, missing_width, source)
}

fn resolve_dictionary<'a>(pdf: &'a ParsedPdf, value: &'a PdfValue) -> Option<&'a PdfDictionary> {
    match value {
        PdfValue::Dictionary(dictionary) => Some(dictionary),
        PdfValue::Reference(reference) => pdf.object(*reference)?.dictionary(),
        _ => None,
    }
}

fn resolve_array<'a>(pdf: &'a ParsedPdf, value: &'a PdfValue) -> Option<&'a [PdfValue]> {
    match value {
        PdfValue::Array(values) => Some(values),
        PdfValue::Reference(reference) => pdf.object(*reference)?.value.as_array(),
        _ => None,
    }
}

fn matrix_from_pdf_value(value: &PdfValue) -> Option<Matrix> {
    let values = value.as_array()?;
    let numbers = values
        .iter()
        .filter_map(number_from_pdf_value)
        .collect::<Vec<_>>();
    Matrix::from_values(&numbers)
}

fn resource_reference_map(dictionary: &PdfDictionary) -> BTreeMap<String, ObjectRef> {
    dictionary
        .iter()
        .filter_map(|(name, value)| {
            value
                .as_reference()
                .map(|reference| (name.clone(), reference))
        })
        .collect()
}

fn collect_references(pdf: &ParsedPdf, value: &PdfValue) -> Vec<ObjectRef> {
    match value {
        PdfValue::Reference(reference) => {
            if pdf
                .object(*reference)
                .is_some_and(|object| object.stream.is_some())
            {
                vec![*reference]
            } else if let Some(object) = pdf.object(*reference) {
                collect_references(pdf, &object.value)
            } else {
                vec![*reference]
            }
        }
        PdfValue::Array(values) => values
            .iter()
            .flat_map(|value| collect_references(pdf, value))
            .collect(),
        _ => Vec::new(),
    }
}

fn collect_direct_references(value: &PdfValue) -> Vec<ObjectRef> {
    match value {
        PdfValue::Reference(reference) => vec![*reference],
        PdfValue::Array(values) => values.iter().flat_map(collect_direct_references).collect(),
        _ => Vec::new(),
    }
}

fn matrix_from_operands(operator: &ContentOperator) -> Option<Matrix> {
    Matrix::from_values(&number_operands(operator))
}

fn number_operands(operator: &ContentOperator) -> Vec<f64> {
    operator.operands.iter().filter_map(token_number).collect()
}

fn number_operand(operator: &ContentOperator, index: usize) -> Option<f64> {
    operator.operands.get(index).and_then(token_number)
}

fn name_operand(operator: &ContentOperator, index: usize) -> Option<&str> {
    operator
        .operands
        .get(index)
        .filter(|token| token.kind == ContentTokenKind::Name)
        .map(|token| token.lexeme.as_str())
}

fn token_number(token: &ContentToken) -> Option<f64> {
    (token.kind == ContentTokenKind::Number)
        .then(|| token.lexeme.parse::<f64>().ok())
        .flatten()
}

fn decode_text_operator(operator: &ContentOperator, font: Option<&FontResource>) -> DecodedText {
    let mut glyph_bytes = Vec::new();
    let mut warnings = Vec::new();

    for token in &operator.operands {
        match token.kind {
            ContentTokenKind::String => glyph_bytes.extend(decode_literal_string_bytes(token)),
            ContentTokenKind::HexString => glyph_bytes.extend(decode_hex_string_bytes(token)),
            _ => {}
        }
    }

    if glyph_bytes.is_empty() {
        return DecodedText {
            text: String::new(),
            glyph_bytes,
            warnings,
        };
    }

    let text = if let Some(font) = font {
        if !font.to_unicode.is_empty() {
            decode_with_to_unicode(&glyph_bytes, &font.to_unicode, &mut warnings)
        } else {
            glyph_bytes
                .iter()
                .map(|byte| {
                    font.encoding
                        .get(&u16::from(*byte))
                        .cloned()
                        .unwrap_or_else(|| char::from(*byte).to_string())
                })
                .collect::<String>()
        }
    } else {
        String::from_utf8_lossy(&glyph_bytes).into_owned()
    };

    DecodedText {
        text,
        glyph_bytes,
        warnings,
    }
}

fn add_context_properties(properties: &mut Vec<PageObjectProperty>, context: &ExtractionContext) {
    if let Some(form) = &context.parent_form {
        properties.push(prop(
            "Parent form",
            format!("/{} {}", form.name, form.reference),
        ));
        properties.push(prop("Form depth", form.depth.to_string()));
        properties.push(prop("Form invoked from", form.invoked_from.to_string()));
        properties.push(prop(
            "Form BBox",
            form.bbox
                .as_ref()
                .map(format_page_object_bounds)
                .unwrap_or_else(|| "-".to_string()),
        ));
    }
}

fn format_page_object_bounds(bounds: &PageObjectBounds) -> String {
    format!(
        "[{} {} {} {}]",
        format_number(bounds.lower_left_x),
        format_number(bounds.lower_left_y),
        format_number(bounds.upper_right_x),
        format_number(bounds.upper_right_y)
    )
}

fn hex_bytes(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return "-".to_string();
    }
    bytes
        .iter()
        .take(64)
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn decode_literal_string_bytes(token: &ContentToken) -> Vec<u8> {
    let raw = token.raw_bytes.as_slice();
    let inner = raw
        .strip_prefix(b"(")
        .and_then(|value| value.strip_suffix(b")"))
        .unwrap_or(raw);
    let mut output = Vec::new();
    let mut index = 0usize;

    while index < inner.len() {
        let byte = inner[index];
        if byte != b'\\' {
            output.push(byte);
            index += 1;
            continue;
        }

        index += 1;
        let Some(escaped) = inner.get(index).copied() else {
            break;
        };
        match escaped {
            b'n' => output.push(b'\n'),
            b'r' => output.push(b'\r'),
            b't' => output.push(b'\t'),
            b'b' => output.push(0x08),
            b'f' => output.push(0x0c),
            b'\r' => {
                if inner.get(index + 1) == Some(&b'\n') {
                    index += 1;
                }
            }
            b'\n' => {}
            b'0'..=b'7' => {
                let start = index;
                let mut end = index + 1;
                while end < inner.len() && end - start < 3 && matches!(inner[end], b'0'..=b'7') {
                    end += 1;
                }
                let value = inner[start..end]
                    .iter()
                    .fold(0u16, |acc, byte| acc * 8 + u16::from(byte - b'0'));
                output.push(value.min(255) as u8);
                index = end - 1;
            }
            other => output.push(other),
        }
        index += 1;
    }

    output
}

fn decode_hex_string_bytes(token: &ContentToken) -> Vec<u8> {
    let raw = token.raw_bytes.as_slice();
    let inner = raw
        .strip_prefix(b"<")
        .and_then(|value| value.strip_suffix(b">"))
        .unwrap_or(raw);
    let mut nibbles = inner
        .iter()
        .copied()
        .filter(|byte| !byte.is_ascii_whitespace())
        .collect::<Vec<_>>();
    if nibbles.len() % 2 == 1 {
        nibbles.push(b'0');
    }

    nibbles
        .chunks(2)
        .filter_map(|pair| {
            let high = hex_nibble(pair[0])?;
            let low = hex_nibble(pair[1])?;
            Some((high << 4) | low)
        })
        .collect()
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn parse_to_unicode_cmap(bytes: &[u8]) -> BTreeMap<Vec<u8>, String> {
    let mut map = BTreeMap::new();
    let text = String::from_utf8_lossy(bytes);
    let lines = text.lines().collect::<Vec<_>>();
    let mut index = 0usize;

    while index < lines.len() {
        let line = lines[index].trim();
        if line.ends_with("beginbfchar") {
            index += 1;
            while index < lines.len() && !lines[index].trim().ends_with("endbfchar") {
                let hex = hex_tokens(lines[index]);
                if hex.len() >= 2 {
                    map.insert(hex[0].clone(), utf16be_hex_to_string(&hex[1]));
                }
                index += 1;
            }
        } else if line.ends_with("beginbfrange") {
            index += 1;
            while index < lines.len() && !lines[index].trim().ends_with("endbfrange") {
                parse_bfrange_line(lines[index], &mut map);
                index += 1;
            }
        }
        index += 1;
    }

    map
}

fn parse_bfrange_line(line: &str, map: &mut BTreeMap<Vec<u8>, String>) {
    let hex = hex_tokens(line);
    if hex.len() < 3 {
        return;
    }
    let start = bytes_to_code(&hex[0]);
    let end = bytes_to_code(&hex[1]);
    let Some((start, end)) = start.zip(end) else {
        return;
    };

    if line.contains('[') {
        for (offset, destination) in hex.iter().skip(2).enumerate() {
            let source = code_to_bytes(start + offset as u32, hex[0].len());
            map.insert(source, utf16be_hex_to_string(destination));
            if start + offset as u32 >= end {
                break;
            }
        }
    } else if let Some(base) = bytes_to_code(&hex[2]) {
        for code in start..=end {
            let source = code_to_bytes(code, hex[0].len());
            let destination = code_to_bytes(base + (code - start), hex[2].len());
            map.insert(source, utf16be_hex_to_string(&destination));
        }
    }
}

fn hex_tokens(line: &str) -> Vec<Vec<u8>> {
    let bytes = line.as_bytes();
    let mut output = Vec::new();
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] != b'<' || bytes.get(index + 1) == Some(&b'<') {
            index += 1;
            continue;
        }
        index += 1;
        let start = index;
        while index < bytes.len() && bytes[index] != b'>' {
            index += 1;
        }
        let raw = bytes[start..index]
            .iter()
            .copied()
            .filter(|byte| !byte.is_ascii_whitespace())
            .collect::<Vec<_>>();
        let token = ContentToken {
            kind: ContentTokenKind::HexString,
            lexeme: String::from_utf8_lossy(&raw).into_owned(),
            byte_range: ByteRange { start: 0, end: 0 },
            raw_bytes: [b"<".as_slice(), raw.as_slice(), b">".as_slice()].concat(),
        };
        output.push(decode_hex_string_bytes(&token));
        index += 1;
    }
    output
}

fn bytes_to_code(bytes: &[u8]) -> Option<u32> {
    (!bytes.is_empty()).then(|| {
        bytes
            .iter()
            .fold(0u32, |acc, byte| (acc << 8) | u32::from(*byte))
    })
}

fn code_to_bytes(code: u32, len: usize) -> Vec<u8> {
    (0..len)
        .rev()
        .map(|index| ((code >> (index * 8)) & 0xff) as u8)
        .collect()
}

fn utf16be_hex_to_string(bytes: &[u8]) -> String {
    if bytes.len() >= 2 && bytes.len() % 2 == 0 {
        let units = bytes
            .chunks(2)
            .map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
            .collect::<Vec<_>>();
        String::from_utf16(&units).unwrap_or_else(|_| String::from_utf8_lossy(bytes).into_owned())
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    }
}

fn decode_with_to_unicode(
    bytes: &[u8],
    cmap: &BTreeMap<Vec<u8>, String>,
    warnings: &mut Vec<String>,
) -> String {
    let max_key_len = cmap.keys().map(Vec::len).max().unwrap_or(1).max(1);
    let mut output = String::new();
    let mut index = 0usize;
    let mut missed = 0usize;

    while index < bytes.len() {
        let mut matched = None;
        for len in (1..=max_key_len).rev() {
            if index + len > bytes.len() {
                continue;
            }
            if let Some(value) = cmap.get(&bytes[index..index + len]) {
                matched = Some((len, value));
                break;
            }
        }

        if let Some((len, value)) = matched {
            output.push_str(value);
            index += len;
        } else {
            output.push(char::from(bytes[index]));
            index += 1;
            missed += 1;
        }
    }

    if missed > 0 {
        warnings.push(format!(
            "{missed} glyph byte(s) were not covered by the font /ToUnicode CMap."
        ));
    }

    output
}

fn text_advance_width(
    operator: &ContentOperator,
    glyph_bytes: &[u8],
    text: &str,
    font: Option<&FontResource>,
    state: &GraphicsState,
    font_size: f64,
) -> f64 {
    let glyph_count = glyph_bytes.len().max(text.chars().count()).max(1);
    let mut text_space_width = 0.0;
    let mut used_widths = false;

    if let Some(font) = font {
        for byte in glyph_bytes {
            if let Some(width) = font
                .widths
                .get(&u16::from(*byte))
                .copied()
                .or(font.missing_width)
            {
                text_space_width += width / 1000.0 * font_size;
                used_widths = true;
            }
        }
    }

    if !used_widths {
        text_space_width = glyph_count as f64 * font_size * 0.5;
    }

    let space_count = glyph_bytes.iter().filter(|byte| **byte == b' ').count();
    text_space_width += glyph_count.saturating_sub(1) as f64 * state.character_spacing;
    text_space_width += space_count as f64 * state.word_spacing;
    text_space_width -= tj_adjustment(operator) / 1000.0 * font_size;
    (text_space_width * (state.horizontal_scaling / 100.0)).max(font_size * 0.25)
}

fn tj_adjustment(operator: &ContentOperator) -> f64 {
    if operator.name != "TJ" {
        return 0.0;
    }
    operator
        .operands
        .iter()
        .filter_map(|token| {
            (token.kind == ContentTokenKind::Number)
                .then(|| token.lexeme.parse::<f64>().ok())
                .flatten()
        })
        .sum()
}

fn builtin_encoding(name: &str) -> BTreeMap<u16, String> {
    let mut map = (0u16..=255)
        .map(|code| (code, latin1_char(code as u8).to_string()))
        .collect::<BTreeMap<_, _>>();

    if matches!(
        name,
        "WinAnsiEncoding" | "MacRomanEncoding" | "StandardEncoding"
    ) {
        for (code, glyph) in STANDARD_GLYPH_NAMES {
            map.insert(*code, glyph_name_to_unicode(glyph));
        }
    }

    map
}

fn latin1_char(byte: u8) -> char {
    match byte {
        0x00..=0x7f => char::from(byte),
        0x80..=0x9f => '\u{fffd}',
        _ => char::from_u32(u32::from(byte)).unwrap_or('\u{fffd}'),
    }
}

fn glyph_name_to_unicode(name: &str) -> String {
    if name.len() == 1 && name.as_bytes()[0].is_ascii_alphabetic() {
        return name.to_string();
    }

    match name {
        "space" => " ",
        "exclam" => "!",
        "quotedbl" => "\"",
        "numbersign" => "#",
        "dollar" => "$",
        "percent" => "%",
        "ampersand" => "&",
        "quotesingle" | "quoteright" => "'",
        "parenleft" => "(",
        "parenright" => ")",
        "asterisk" => "*",
        "plus" => "+",
        "comma" => ",",
        "hyphen" | "minus" => "-",
        "period" => ".",
        "slash" => "/",
        "colon" => ":",
        "semicolon" => ";",
        "less" => "<",
        "equal" => "=",
        "greater" => ">",
        "question" => "?",
        "at" => "@",
        "bracketleft" => "[",
        "backslash" => "\\",
        "bracketright" => "]",
        "asciicircum" => "^",
        "underscore" => "_",
        "quoteleft" | "grave" => "`",
        "braceleft" => "{",
        "bar" => "|",
        "braceright" => "}",
        "asciitilde" => "~",
        "zero" | "one" | "two" | "three" | "four" | "five" | "six" | "seven" | "eight" | "nine" => {
            return glyph_name_basic(name)
        }
        _ if name.starts_with("uni") && name.len() >= 7 => {
            return glyph_name_uni(name).unwrap_or_else(|| format!("/{name}"))
        }
        _ => return format!("/{name}"),
    }
    .to_string()
}

fn glyph_name_basic(name: &str) -> String {
    match name {
        "zero" => "0",
        "one" => "1",
        "two" => "2",
        "three" => "3",
        "four" => "4",
        "five" => "5",
        "six" => "6",
        "seven" => "7",
        "eight" => "8",
        "nine" => "9",
        _ => name,
    }
    .to_string()
}

fn glyph_name_uni(name: &str) -> Option<String> {
    let hex = name.strip_prefix("uni")?;
    let mut output = String::new();
    for chunk in hex.as_bytes().chunks(4) {
        if chunk.len() != 4 {
            return None;
        }
        let value = std::str::from_utf8(chunk)
            .ok()
            .and_then(|value| u16::from_str_radix(value, 16).ok())?;
        output.push(char::from_u32(u32::from(value))?);
    }
    Some(output)
}

const STANDARD_GLYPH_NAMES: &[(u16, &str)] = &[
    (32, "space"),
    (33, "exclam"),
    (34, "quotedbl"),
    (35, "numbersign"),
    (36, "dollar"),
    (37, "percent"),
    (38, "ampersand"),
    (39, "quotesingle"),
    (40, "parenleft"),
    (41, "parenright"),
    (42, "asterisk"),
    (43, "plus"),
    (44, "comma"),
    (45, "hyphen"),
    (46, "period"),
    (47, "slash"),
    (48, "zero"),
    (49, "one"),
    (50, "two"),
    (51, "three"),
    (52, "four"),
    (53, "five"),
    (54, "six"),
    (55, "seven"),
    (56, "eight"),
    (57, "nine"),
    (58, "colon"),
    (59, "semicolon"),
    (60, "less"),
    (61, "equal"),
    (62, "greater"),
    (63, "question"),
    (64, "at"),
    (91, "bracketleft"),
    (92, "backslash"),
    (93, "bracketright"),
    (94, "asciicircum"),
    (95, "underscore"),
    (96, "grave"),
    (123, "braceleft"),
    (124, "bar"),
    (125, "braceright"),
    (126, "asciitilde"),
];

fn bounds_from_pdf_value(value: &PdfValue) -> Option<PageObjectBounds> {
    let values = value.as_array()?;
    if values.len() != 4 {
        return None;
    }
    let x1 = number_from_pdf_value(&values[0])?;
    let y1 = number_from_pdf_value(&values[1])?;
    let x2 = number_from_pdf_value(&values[2])?;
    let y2 = number_from_pdf_value(&values[3])?;
    Some(bounds_from_extents(x1, y1, x2, y2))
}

fn transformed_bounds_from_pdf_value(value: &PdfValue, matrix: Matrix) -> Option<PageObjectBounds> {
    let values = value.as_array()?;
    if values.len() != 4 {
        return None;
    }
    let x1 = number_from_pdf_value(&values[0])?;
    let y1 = number_from_pdf_value(&values[1])?;
    let x2 = number_from_pdf_value(&values[2])?;
    let y2 = number_from_pdf_value(&values[3])?;
    bounds_from_points(&[
        matrix.transform_point(x1, y1),
        matrix.transform_point(x2, y1),
        matrix.transform_point(x1, y2),
        matrix.transform_point(x2, y2),
    ])
}

fn number_from_pdf_value(value: &PdfValue) -> Option<f64> {
    match value {
        PdfValue::Number(value) => Some(*value),
        _ => None,
    }
}

fn bounds_from_points(points: &[(f64, f64)]) -> Option<PageObjectBounds> {
    let first = points.first()?;
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (first.0, first.1, first.0, first.1);
    for (x, y) in points.iter().copied().skip(1) {
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    }
    Some(bounds_from_extents(min_x, min_y, max_x, max_y))
}

fn bounds_from_extents(x1: f64, y1: f64, x2: f64, y2: f64) -> PageObjectBounds {
    let lower_left_x = x1.min(x2);
    let lower_left_y = y1.min(y2);
    let upper_right_x = x1.max(x2);
    let upper_right_y = y1.max(y2);
    PageObjectBounds {
        coordinate_space: "pdf",
        lower_left_x,
        lower_left_y,
        upper_right_x,
        upper_right_y,
        width: (upper_right_x - lower_left_x).abs(),
        height: (upper_right_y - lower_left_y).abs(),
    }
}

fn prop(name: impl Into<String>, value: impl Into<String>) -> PageObjectProperty {
    PageObjectProperty {
        name: name.into(),
        value: value.into(),
    }
}

fn format_range(range: ByteRange) -> String {
    format!("{}..{}", range.start, range.end)
}

fn format_number(value: f64) -> String {
    if !value.is_finite() {
        return "-".to_string();
    }
    if value.fract() == 0.0 {
        format!("{}", value as i64)
    } else {
        format!("{value:.2}")
    }
}

fn truncate(value: &str, max_chars: usize) -> String {
    let mut output = value.chars().take(max_chars).collect::<String>();
    if value.chars().count() > max_chars {
        output.push_str("...");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf_model::ObjectRef;
    use crate::pdf_parser::parse_bytes;

    #[test]
    fn extracts_first_step_page_objects() {
        let pdf = parse_bytes(page_object_pdf().as_bytes(), "page-objects.pdf");
        let inspection = inspect_page_objects(&pdf, 1).expect("page object inspection");

        assert!(inspection
            .objects
            .iter()
            .any(|object| object.kind == PageObjectKind::Text && object.label.contains("Hi")));
        assert!(inspection
            .objects
            .iter()
            .any(|object| object.kind == PageObjectKind::Path && object.bbox.is_some()));
        assert!(inspection.objects.iter().any(|object| {
            object.kind == PageObjectKind::Image
                && object.reference == Some(ObjectRef::new(6, 0))
                && object.bbox.is_some()
        }));
        assert!(inspection.objects.iter().any(|object| {
            object.kind == PageObjectKind::Annotation
                && object.reference == Some(ObjectRef::new(7, 0))
                && object.bbox.is_some()
        }));
    }

    #[test]
    fn decodes_text_with_to_unicode_and_widths() {
        let pdf = parse_bytes(to_unicode_text_pdf().as_bytes(), "to-unicode.pdf");
        let inspection = inspect_page_objects(&pdf, 1).expect("page object inspection");
        let text = inspection
            .objects
            .iter()
            .find(|object| object.kind == PageObjectKind::Text)
            .expect("text object");

        assert!(text.label.contains("AB"));
        assert!(text
            .properties
            .iter()
            .any(|property| property.name == "Raw glyph bytes" && property.value == "01 02"));
        assert!(text
            .properties
            .iter()
            .any(|property| property.name == "Width source" && property.value.contains("/Widths")));
        assert!(text.bbox.is_some());
    }

    #[test]
    fn recursively_expands_form_xobject_children() {
        let pdf = parse_bytes(form_xobject_pdf().as_bytes(), "form-xobject.pdf");
        let inspection = inspect_page_objects(&pdf, 1).expect("page object inspection");

        assert!(inspection.objects.iter().any(|object| {
            object.kind == PageObjectKind::Form && object.reference == Some(ObjectRef::new(5, 0))
        }));
        let form_text = inspection
            .objects
            .iter()
            .find(|object| {
                object.kind == PageObjectKind::Text
                    && object.properties.iter().any(|property| {
                        property.name == "Parent form" && property.value.contains("5 0 R")
                    })
            })
            .expect("text object from form");
        assert!(form_text.label.contains("In form"));
        assert!(form_text.bbox.is_some());
        assert!(inspection.objects.iter().any(|object| {
            object.kind == PageObjectKind::Path
                && object.properties.iter().any(|property| {
                    property.name == "Parent form" && property.value.contains("5 0 R")
                })
        }));
    }

    fn page_object_pdf() -> String {
        let content = b"q 40 0 0 30 20 40 cm /Im1 Do Q\nBT /F1 12 Tf 10 120 Td (Hi) Tj ET\n10 10 50 20 re S\n";
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"%PDF-1.4\n");
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
                b"3 0 obj\n<< /Type /Page /Parent 2 0 R /Resources << /Font << /F1 5 0 R >> /XObject << /Im1 6 0 R >> >> /Contents 4 0 R /Annots [7 0 R] >>\nendobj\n",
            ),
            push(
                &mut bytes,
                format!(
                    "4 0 obj\n<< /Length {} >>\nstream\n{}endstream\nendobj\n",
                    content.len(),
                    String::from_utf8_lossy(content)
                )
                .as_bytes(),
            ),
            push(
                &mut bytes,
                b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n",
            ),
            push(
                &mut bytes,
                b"6 0 obj\n<< /Type /XObject /Subtype /Image /Width 8 /Height 6 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Length 3 >>\nstream\nabc\nendstream\nendobj\n",
            ),
            push(
                &mut bytes,
                b"7 0 obj\n<< /Type /Annot /Subtype /Text /Rect [150 150 180 175] /F 4 >>\nendobj\n",
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

    fn to_unicode_text_pdf() -> String {
        let content = b"BT /F1 20 Tf 10 100 Td <0102> Tj ET\n";
        let cmap = b"/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n1 begincodespacerange\n<01> <02>\nendcodespacerange\n2 beginbfchar\n<01> <0041>\n<02> <0042>\nendbfchar\nendcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n";
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"%PDF-1.4\n");
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
            push(
                &mut bytes,
                format!(
                    "4 0 obj\n<< /Length {} >>\nstream\n{}endstream\nendobj\n",
                    content.len(),
                    String::from_utf8_lossy(content)
                )
                .as_bytes(),
            ),
            push(
                &mut bytes,
                b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /FirstChar 1 /LastChar 2 /Widths [600 700] /ToUnicode 6 0 R >>\nendobj\n",
            ),
            push(
                &mut bytes,
                format!(
                    "6 0 obj\n<< /Length {} >>\nstream\n{}endstream\nendobj\n",
                    cmap.len(),
                    String::from_utf8_lossy(cmap)
                )
                .as_bytes(),
            ),
        ];
        let xref = bytes.len();
        bytes.extend_from_slice(b"xref\n0 7\n0000000000 65535 f \n");
        for offset in offsets {
            bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        bytes.extend_from_slice(
            format!("trailer\n<< /Size 7 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes(),
        );
        String::from_utf8(bytes).unwrap()
    }

    fn form_xobject_pdf() -> String {
        let page_content = b"q 2 0 0 2 20 30 cm /Fm1 Do Q\n";
        let form_content = b"BT /F1 10 Tf 5 6 Td (In form) Tj ET\n1 2 10 12 re S\n";
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"%PDF-1.4\n");
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
                b"3 0 obj\n<< /Type /Page /Parent 2 0 R /Resources << /Font << /F1 6 0 R >> /XObject << /Fm1 5 0 R >> >> /Contents 4 0 R >>\nendobj\n",
            ),
            push(
                &mut bytes,
                format!(
                    "4 0 obj\n<< /Length {} >>\nstream\n{}endstream\nendobj\n",
                    page_content.len(),
                    String::from_utf8_lossy(page_content)
                )
                .as_bytes(),
            ),
            push(
                &mut bytes,
                format!(
                    "5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 50 50] /Matrix [1 0 0 1 3 4] /Resources << /Font << /F1 6 0 R >> >> /Length {} >>\nstream\n{}endstream\nendobj\n",
                    form_content.len(),
                    String::from_utf8_lossy(form_content)
                )
                .as_bytes(),
            ),
            push(
                &mut bytes,
                b"6 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n",
            ),
        ];
        let xref = bytes.len();
        bytes.extend_from_slice(b"xref\n0 7\n0000000000 65535 f \n");
        for offset in offsets {
            bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        bytes.extend_from_slice(
            format!("trailer\n<< /Size 7 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes(),
        );
        String::from_utf8(bytes).unwrap()
    }

    fn push(bytes: &mut Vec<u8>, value: &[u8]) -> usize {
        let offset = bytes.len();
        bytes.extend_from_slice(value);
        offset
    }
}
