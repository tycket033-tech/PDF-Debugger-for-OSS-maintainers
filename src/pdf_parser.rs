use crate::pdf_model::{
    ByteRange, ObjectRef, ParseWarning, ParsedPdf, PdfDictionary, PdfMetadata, PdfObject,
    PdfStream, PdfValue, XrefEntry,
};
use crate::pdf_string::decode_pdf_string;
use crate::stream_decode::{decode_stream_with_limit, filter_names_from_dictionary};
use crate::{PdfDebuggerError, Result};
use memchr::memmem;
use memmap2::Mmap;
use regex::bytes::Regex;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::path::Path;
use std::time::Instant;

pub const MAX_FULL_PARSE_FILE_SIZE: u64 = 1024 * 1024 * 1024;
const OBJECT_STREAM_DECODE_LIMIT: usize = 32 * 1024 * 1024;

#[derive(Clone, Copy, Debug)]
struct ParseOptions {
    load_stream_bytes: bool,
    skip_invalid_xref_objects: bool,
}

const DEFAULT_PARSE_OPTIONS: ParseOptions = ParseOptions {
    load_stream_bytes: true,
    skip_invalid_xref_objects: false,
};

const GUI_PARSE_OPTIONS: ParseOptions = ParseOptions {
    load_stream_bytes: false,
    skip_invalid_xref_objects: true,
};

pub fn parse_file(path: &Path) -> Result<ParsedPdf> {
    parse_file_with_options(path, DEFAULT_PARSE_OPTIONS)
}

pub fn parse_file_without_stream_bytes(path: &Path) -> Result<ParsedPdf> {
    parse_file_with_options(path, GUI_PARSE_OPTIONS)
}

fn parse_file_with_options(path: &Path, options: ParseOptions) -> Result<ParsedPdf> {
    let metadata = std::fs::metadata(path)?;
    if metadata.len() > MAX_FULL_PARSE_FILE_SIZE {
        return Err(PdfDebuggerError::FileTooLarge {
            size: metadata.len(),
            limit: MAX_FULL_PARSE_FILE_SIZE,
        });
    }

    let map_started = Instant::now();
    let file = File::open(path)?;
    let mapped = if metadata.len() == 0 {
        None
    } else {
        // SAFETY: the mapping is read-only and lives only for this parse call. ParsedPdf owns
        // all data it keeps, so no references to the mapping escape this function.
        Some(unsafe { Mmap::map(&file)? })
    };
    parser_perf_log("parse_file.map", &path.display().to_string(), map_started);
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string());
    let parse_started = Instant::now();
    let bytes = mapped.as_deref().unwrap_or(&[]);
    let parsed = parse_bytes_with_options(bytes, file_name, options);
    parser_perf_log("parse_file.parse_bytes", &path.display().to_string(), parse_started);
    Ok(parsed)
}

pub fn parse_bytes(bytes: &[u8], file_name: impl Into<String>) -> ParsedPdf {
    parse_bytes_with_options(bytes, file_name, DEFAULT_PARSE_OPTIONS)
}

fn parse_bytes_with_options(
    bytes: &[u8],
    file_name: impl Into<String>,
    options: ParseOptions,
) -> ParsedPdf {
    let file_name = file_name.into();
    let mut warnings = Vec::new();
    let header_started = Instant::now();
    let pdf_version = parse_pdf_version(bytes, &mut warnings);
    let linearized = bytes
        .get(..bytes.len().min(1024))
        .is_some_and(|prefix| find_subslice(prefix, b"/Linearized").is_some());
    parser_perf_log("parse_bytes.header_scan", &file_name, header_started);

    let xref_started = Instant::now();
    let (mut xref_entries, classic_trailer, xref_section_count) =
        parse_xref_sections(bytes, &mut warnings);
    let incremental_update_count = xref_section_count.saturating_sub(1);
    parser_perf_log("parse_bytes.xref_sections", &file_name, xref_started);

    let objects_started = Instant::now();
    let mut objects = parse_indirect_objects(bytes, &xref_entries, options, &mut warnings);
    parser_perf_log("parse_bytes.indirect_objects", &file_name, objects_started);

    let object_stream_started = Instant::now();
    expand_object_streams(&mut objects, &mut warnings);
    parser_perf_log(
        "parse_bytes.object_streams",
        &file_name,
        object_stream_started,
    );

    let metadata_started = Instant::now();
    let trailer = classic_trailer.or_else(|| xref_stream_trailer(&objects));
    validate_xref_offsets(bytes, &mut xref_entries);

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

    let info_summary = info
        .and_then(|reference| objects.get(&reference))
        .and_then(PdfObject::dictionary)
        .map(summary_dictionary)
        .unwrap_or_default();

    let stream_count = objects
        .values()
        .filter(|object| object.stream.is_some())
        .count();
    let has_xref_stream = objects.values().any(|object| {
        object
            .dictionary()
            .is_some_and(|dict| name_is(dict, "Type", "XRef"))
    });
    let has_object_stream = objects.values().any(|object| {
        object
            .dictionary()
            .is_some_and(|dict| name_is(dict, "Type", "ObjStm"))
    });
    parser_perf_log("parse_bytes.metadata", &file_name, metadata_started);

    if objects.is_empty() {
        warnings.push(ParseWarning {
            offset: None,
            message: "No indirect objects were discovered".to_string(),
        });
    }
    if xref_entries.is_empty() {
        warnings.push(ParseWarning {
            offset: None,
            message: "No classic xref table was discovered".to_string(),
        });
    }
    if trailer.is_none() {
        warnings.push(ParseWarning {
            offset: None,
            message: "No trailer dictionary was discovered".to_string(),
        });
    }

    let mut parsed = ParsedPdf {
        metadata: PdfMetadata {
            file_name: file_name.clone(),
            file_size: bytes.len(),
            pdf_version,
            page_count: None,
            encrypted,
            linearized,
            incremental_update_count,
            root,
            info,
            trailer_keys,
            info_summary,
            object_count: objects.len(),
            stream_count,
            xref_entry_count: xref_entries.len(),
            has_xref_stream,
            has_object_stream,
            parse_warning_count: warnings.len(),
        },
        trailer,
        objects,
        xref_entries,
        warnings,
    };

    let page_count_started = Instant::now();
    parsed.metadata.page_count = detect_page_count(&parsed);
    parsed.metadata.parse_warning_count = parsed.warnings.len();
    parser_perf_log("parse_bytes.page_count", &file_name, page_count_started);
    parsed
}

fn parse_pdf_version(bytes: &[u8], warnings: &mut Vec<ParseWarning>) -> Option<String> {
    let search_end = bytes.len().min(1024);
    if let Some(offset) = find_subslice(&bytes[..search_end], b"%PDF-") {
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

    warnings.push(ParseWarning {
        offset: Some(0),
        message: "PDF header was not found in the first 1024 bytes".to_string(),
    });
    None
}

fn parse_object_header_at(bytes: &[u8], offset: usize) -> Option<(usize, ObjectRef)> {
    let mut pos = skip_whitespace_and_comments(bytes, offset);
    let object = parse_u32_at(bytes, &mut pos)?;
    pos = skip_ascii_whitespace(bytes, pos);
    let generation = parse_u16_at(bytes, &mut pos)?;
    pos = skip_ascii_whitespace(bytes, pos);
    if !bytes.get(pos..).is_some_and(|tail| tail.starts_with(b"obj")) {
        return None;
    }
    let end = pos + b"obj".len();
    if bytes
        .get(end)
        .is_some_and(|byte| !byte.is_ascii_whitespace() && !is_delimiter(*byte))
    {
        return None;
    }
    Some((end, ObjectRef::new(object, generation)))
}

fn parse_indirect_objects(
    bytes: &[u8],
    xref_entries: &[XrefEntry],
    options: ParseOptions,
    warnings: &mut Vec<ParseWarning>,
) -> BTreeMap<ObjectRef, PdfObject> {
    if let Some(objects) = parse_indirect_objects_from_xref(bytes, xref_entries, options, warnings)
    {
        return objects;
    }

    parse_indirect_objects_by_scan(bytes, options, warnings)
}

#[derive(Clone, Copy)]
struct ObjectStart {
    raw_start: usize,
    header_end: usize,
    reference: ObjectRef,
}

fn parse_indirect_objects_from_xref(
    bytes: &[u8],
    xref_entries: &[XrefEntry],
    options: ParseOptions,
    warnings: &mut Vec<ParseWarning>,
) -> Option<BTreeMap<ObjectRef, PdfObject>> {
    let in_use_count = xref_entries.iter().filter(|entry| entry.in_use).count();
    if in_use_count == 0 {
        return None;
    }

    let mut starts = Vec::with_capacity(in_use_count);
    for entry in xref_entries.iter().filter(|entry| entry.in_use) {
        let Some((header_end, reference)) = parse_object_header_at(bytes, entry.offset) else {
            continue;
        };
        if reference == entry.reference {
            starts.push(ObjectStart {
                raw_start: entry.offset,
                header_end,
                reference,
            });
        }
    }

    let skipped_count = in_use_count.saturating_sub(starts.len());
    if options.skip_invalid_xref_objects && !starts.is_empty() {
        if skipped_count > 0 {
            warnings.push(ParseWarning {
                offset: None,
                message: format!(
                    "Skipped {skipped_count} in-use xref object(s) whose offsets did not resolve to the expected object"
                ),
            });
        }
    } else if starts.len() < 2 || starts.len().saturating_mul(2) < in_use_count {
        warnings.push(ParseWarning {
            offset: None,
            message: format!(
                "Classic xref offsets parsed only {} of {} in-use object(s); falling back to full object scan",
                starts.len(),
                in_use_count
            ),
        });
        return None;
    }

    starts.sort_by_key(|start| start.raw_start);
    starts.dedup_by_key(|start| start.reference);
    Some(parse_indirect_object_starts(
        bytes, &starts, options, warnings,
    ))
}

fn parse_indirect_objects_by_scan(
    bytes: &[u8],
    options: ParseOptions,
    warnings: &mut Vec<ParseWarning>,
) -> BTreeMap<ObjectRef, PdfObject> {
    let object_re = Regex::new(r"(?m)(\d+)\s+(\d+)\s+obj\b").expect("valid object regex");
    let mut starts = Vec::new();

    for captures in object_re.captures_iter(bytes) {
        let Some(full_match) = captures.get(0) else {
            continue;
        };
        let object = parse_u32(captures.get(1).unwrap().as_bytes());
        let generation = parse_u16(captures.get(2).unwrap().as_bytes());
        if let (Some(object), Some(generation)) = (object, generation) {
            starts.push(ObjectStart {
                raw_start: full_match.start(),
                header_end: full_match.end(),
                reference: ObjectRef::new(object, generation),
            });
        }
    }

    parse_indirect_object_starts(bytes, &starts, options, warnings)
}

fn parse_indirect_object_starts(
    bytes: &[u8],
    starts: &[ObjectStart],
    options: ParseOptions,
    warnings: &mut Vec<ParseWarning>,
) -> BTreeMap<ObjectRef, PdfObject> {
    let mut objects = BTreeMap::new();
    for (index, start) in starts.iter().copied().enumerate() {
        let search_end = starts
            .get(index + 1)
            .map(|next| next.raw_start)
            .unwrap_or(bytes.len());
        let body = &bytes[start.header_end..search_end];

        match parse_object_body(
            bytes,
            body,
            start.raw_start,
            start.header_end,
            start.reference,
            options,
        ) {
            Some(object) => {
                objects.insert(start.reference, object);
            }
            None => warnings.push(ParseWarning {
                offset: Some(start.raw_start),
                message: format!("Could not parse indirect object {}", start.reference),
            }),
        }
    }

    objects
}

fn parse_object_body(
    full_bytes: &[u8],
    body: &[u8],
    raw_start: usize,
    body_offset: usize,
    reference: ObjectRef,
    options: ParseOptions,
) -> Option<PdfObject> {
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
        let (data_end, stream_marker) =
            match declared_length.and_then(|length| endstream_after_declared_length(body, data_start, length)) {
                Some((data_end, marker)) => (data_end, marker),
                None => {
                    let marker = find_subslice(&body[data_start..], b"endstream")
                        .map(|relative| data_start + relative)
                        .unwrap_or_else(|| body.len());
                    (trim_single_stream_eol(body, data_start, marker), marker)
                }
            };
        let raw_range = ByteRange {
            start: body_offset + data_start,
            end: body_offset + data_end,
        };
        let filters = filter_names_from_dictionary(&dictionary);
        let actual_length = raw_range.len();
        let load_stream_bytes =
            options.load_stream_bytes || name_is(&dictionary, "Type", "ObjStm");
        let raw_bytes = if load_stream_bytes {
            full_bytes[raw_range.start..raw_range.end.min(full_bytes.len())].to_vec()
        } else {
            Vec::new()
        };
        let raw_end = stream_object_raw_end(body, body_offset, stream_marker)
            .unwrap_or_else(|| body_offset + body.len());

        Some(PdfObject {
            reference,
            value: PdfValue::Dictionary(dictionary.clone()),
            stream: Some(PdfStream {
                dictionary,
                declared_length,
                actual_length,
                filters,
                raw_range,
                raw_bytes,
            }),
            raw_range: ByteRange {
                start: raw_start,
                end: raw_end,
            },
            raw_bytes: Vec::new(),
        })
    } else {
        let relative_end = find_subslice(&body[value_end..], b"endobj").map(|end| value_end + end);
        let body_end = relative_end.unwrap_or(body.len());
        let raw_end = relative_end
            .map(|end| body_offset + end + b"endobj".len())
            .unwrap_or_else(|| body_offset + body_end);
        Some(PdfObject {
            reference,
            value,
            stream: None,
            raw_range: ByteRange {
                start: raw_start,
                end: raw_end,
            },
            raw_bytes: Vec::new(),
        })
    }
}

fn endstream_after_declared_length(
    body: &[u8],
    data_start: usize,
    declared_length: usize,
) -> Option<(usize, usize)> {
    let data_end = data_start.checked_add(declared_length)?;
    if data_end > body.len() {
        return None;
    }
    let marker = skip_ascii_whitespace(body, data_end);
    body.get(marker..)
        .is_some_and(|tail| tail.starts_with(b"endstream"))
        .then_some((data_end, marker))
}

fn stream_object_raw_end(body: &[u8], body_offset: usize, stream_marker: usize) -> Option<usize> {
    let after_endstream = stream_marker.checked_add(b"endstream".len())?;
    let marker = find_subslice(body.get(after_endstream..)?, b"endobj")?;
    Some(body_offset + after_endstream + marker + b"endobj".len())
}

fn expand_object_streams(
    objects: &mut BTreeMap<ObjectRef, PdfObject>,
    warnings: &mut Vec<ParseWarning>,
) {
    let object_streams = objects
        .values()
        .filter(|object| {
            object
                .stream
                .as_ref()
                .is_some_and(|stream| name_is(&stream.dictionary, "Type", "ObjStm"))
        })
        .cloned()
        .collect::<Vec<_>>();

    for object in object_streams {
        let Some(stream) = object.stream.as_ref() else {
            continue;
        };
        let Some(first) = stream.dictionary.get("First").and_then(PdfValue::as_usize) else {
            warnings.push(ParseWarning {
                offset: Some(object.raw_range.start),
                message: format!(
                    "Object stream {} is missing numeric /First",
                    object.reference
                ),
            });
            continue;
        };
        let Some(count) = stream.dictionary.get("N").and_then(PdfValue::as_usize) else {
            warnings.push(ParseWarning {
                offset: Some(object.raw_range.start),
                message: format!("Object stream {} is missing numeric /N", object.reference),
            });
            continue;
        };
        let decode = decode_stream_with_limit(stream, Some(OBJECT_STREAM_DECODE_LIMIT));
        if decode.has_issues() {
            let message = decode
                .issues
                .iter()
                .map(|issue| format!("/{}: {}", issue.filter, issue.message))
                .collect::<Vec<_>>()
                .join("; ");
            warnings.push(ParseWarning {
                offset: Some(object.raw_range.start),
                message: format!(
                    "Could not decode object stream {}: {message}",
                    object.reference
                ),
            });
            continue;
        }
        let members = match parse_object_stream_member_table(&decode.decoded, first, count) {
            Some(members) => members,
            None => {
                warnings.push(ParseWarning {
                    offset: Some(object.raw_range.start),
                    message: format!(
                        "Could not parse object stream {} member table",
                        object.reference
                    ),
                });
                continue;
            }
        };
        for (object_number, body_offset) in members {
            let Some(body_start) = first.checked_add(body_offset) else {
                warnings.push(ParseWarning {
                    offset: Some(object.raw_range.start),
                    message: format!(
                        "Object stream {} member {object_number} has an overflowing body offset",
                        object.reference
                    ),
                });
                continue;
            };
            if body_start >= decode.decoded.len() {
                warnings.push(ParseWarning {
                    offset: Some(object.raw_range.start),
                    message: format!(
                        "Object stream {} member {object_number} starts outside decoded bytes",
                        object.reference
                    ),
                });
                continue;
            }
            let body = &decode.decoded[body_start..];
            let Some((value, consumed)) = parse_pdf_value_with_consumed(body) else {
                warnings.push(ParseWarning {
                    offset: Some(object.raw_range.start),
                    message: format!(
                        "Could not parse object stream {} member {object_number}",
                        object.reference
                    ),
                });
                continue;
            };
            let reference = ObjectRef::new(object_number, 0);
            objects.entry(reference).or_insert_with(|| PdfObject {
                reference,
                value,
                stream: None,
                raw_range: object.raw_range,
                raw_bytes: body[..consumed.min(body.len())].to_vec(),
            });
        }
    }
}

fn parse_object_stream_member_table(
    decoded: &[u8],
    first: usize,
    count: usize,
) -> Option<Vec<(u32, usize)>> {
    let table = decoded.get(..first)?;
    let text = String::from_utf8_lossy(table);
    let numbers = text.split_whitespace().collect::<Vec<_>>();
    if numbers.len() < count.saturating_mul(2) {
        return None;
    }
    let mut members = Vec::new();
    for pair in numbers.chunks(2).take(count) {
        let object_number = pair[0].parse::<u32>().ok()?;
        let offset = pair[1].parse::<usize>().ok()?;
        members.push((object_number, offset));
    }
    Some(members)
}

fn xref_stream_trailer(objects: &BTreeMap<ObjectRef, PdfObject>) -> Option<PdfValue> {
    objects
        .values()
        .filter_map(|object| object.stream.as_ref())
        .find(|stream| name_is(&stream.dictionary, "Type", "XRef"))
        .map(|stream| PdfValue::Dictionary(stream.dictionary.clone()))
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

fn parse_xref_sections(
    bytes: &[u8],
    warnings: &mut Vec<ParseWarning>,
) -> (Vec<XrefEntry>, Option<PdfValue>, usize) {
    if let Some(parsed) = parse_xref_sections_from_startxref(bytes) {
        return parsed;
    }

    parse_xref_sections_by_scan(bytes, warnings)
}

fn parse_xref_sections_from_startxref(
    bytes: &[u8],
) -> Option<(Vec<XrefEntry>, Option<PdfValue>, usize)> {
    let mut offset = parse_startxref(bytes)?;
    let mut seen_offsets = BTreeSet::new();
    let mut entries = BTreeMap::<ObjectRef, XrefEntry>::new();
    let mut trailer = None;

    while seen_offsets.insert(offset) {
        let (section_entries, section_trailer, prev) = parse_xref_section_at(bytes, offset, None)?;
        for entry in section_entries {
            entries.entry(entry.reference).or_insert(entry);
        }
        if trailer.is_none() {
            trailer = section_trailer;
        }
        let Some(prev) = prev else {
            break;
        };
        offset = prev;
    }

    let section_count = seen_offsets.len();
    Some((entries.into_values().collect(), trailer, section_count))
}

fn parse_startxref(bytes: &[u8]) -> Option<usize> {
    let marker = memmem::rfind(bytes, b"startxref")?;
    let mut pos = marker + b"startxref".len();
    pos = skip_ascii_whitespace(bytes, pos);
    parse_usize_at(bytes, &mut pos)
}

fn parse_xref_sections_by_scan(
    bytes: &[u8],
    warnings: &mut Vec<ParseWarning>,
) -> (Vec<XrefEntry>, Option<PdfValue>, usize) {
    let mut entries = BTreeMap::<ObjectRef, XrefEntry>::new();
    let mut trailer = None;
    let mut section_count = 0usize;
    let mut cursor = 0;

    while let Some(relative) = find_subslice(&bytes[cursor..], b"xref") {
        let xref_start = cursor + relative;
        if !is_line_start(bytes, xref_start)
            || bytes
                .get(xref_start + b"xref".len())
                .is_some_and(|byte| !byte.is_ascii_whitespace())
        {
            cursor = xref_start + b"xref".len();
            continue;
        }

        if let Some((section_entries, section_trailer, _)) =
            parse_xref_section_at(bytes, xref_start, Some(warnings))
        {
            section_count += 1;
            for entry in section_entries {
                entries.insert(entry.reference, entry);
            }
            if section_trailer.is_some() {
                trailer = section_trailer;
            }
        }

        cursor = xref_start + b"xref".len();
    }

    (entries.into_values().collect(), trailer, section_count)
}

fn parse_xref_section_at(
    bytes: &[u8],
    xref_start: usize,
    mut warnings: Option<&mut Vec<ParseWarning>>,
) -> Option<(Vec<XrefEntry>, Option<PdfValue>, Option<usize>)> {
    if !bytes
        .get(xref_start..)
        .is_some_and(|tail| tail.starts_with(b"xref"))
    {
        return None;
    }

    let mut entries = Vec::new();
    let mut trailer = None;
    let mut prev = None;
    let mut line_cursor = line_end(bytes, xref_start) + 1;
    loop {
        line_cursor = skip_blank_lines(bytes, line_cursor);
        if line_cursor >= bytes.len() {
            break;
        }
        if bytes[line_cursor..].starts_with(b"trailer") {
            let trailer_start = line_cursor + b"trailer".len();
            if let Some((value, _)) = parse_pdf_value_with_consumed(&bytes[trailer_start..]) {
                prev = value
                    .as_dictionary()
                    .and_then(|dictionary| dictionary.get("Prev"))
                    .and_then(PdfValue::as_usize);
                trailer = Some(value);
            } else if let Some(warnings) = warnings.as_deref_mut() {
                warnings.push(ParseWarning {
                    offset: Some(trailer_start),
                    message: "Could not parse trailer dictionary".to_string(),
                });
            }
            break;
        }

        let current_end = line_end(bytes, line_cursor);
        let mut header_pos = line_cursor;
        let Some(first_object) = parse_u32_at(bytes, &mut header_pos) else {
            break;
        };
        header_pos = skip_ascii_whitespace(bytes, header_pos);
        let Some(count) = parse_usize_at(bytes, &mut header_pos) else {
            break;
        };

        line_cursor = current_end + 1;
        for index in 0..count {
            line_cursor = skip_blank_lines(bytes, line_cursor);
            let entry_end = line_end(bytes, line_cursor);
            if let Some(entry) = parse_xref_entry_line(
                bytes,
                line_cursor,
                entry_end,
                first_object + index as u32,
            ) {
                entries.push(entry);
            }
            line_cursor = entry_end + 1;
        }
    }

    Some((entries, trailer, prev))
}

fn parse_xref_entry_line(
    bytes: &[u8],
    start: usize,
    end: usize,
    object_number: u32,
) -> Option<XrefEntry> {
    let mut pos = skip_ascii_whitespace(bytes, start);
    if pos >= end {
        return None;
    }
    let offset = parse_usize_at(bytes, &mut pos)?;
    pos = skip_ascii_whitespace(bytes, pos);
    let generation = parse_u16_at(bytes, &mut pos)?;
    pos = skip_ascii_whitespace(bytes, pos);
    let in_use = *bytes.get(pos)? == b'n';
    let reference = ObjectRef::new(object_number, generation);
    Some(XrefEntry {
        reference,
        offset,
        in_use,
        valid_offset: None,
    })
}

fn validate_xref_offsets(bytes: &[u8], entries: &mut [XrefEntry]) {
    for entry in entries.iter_mut().filter(|entry| entry.in_use) {
        if entry.offset >= bytes.len() {
            entry.valid_offset = Some(false);
            continue;
        }

        let valid = parse_object_header_at(bytes, entry.offset)
            .map(|(_, reference)| reference == entry.reference)
            .unwrap_or(false);
        entry.valid_offset = Some(valid);
    }
}

pub fn parse_pdf_value(input: &[u8]) -> Option<PdfValue> {
    parse_pdf_value_with_consumed(input).map(|(value, _)| value)
}

pub fn parse_pdf_value_with_consumed(input: &[u8]) -> Option<(PdfValue, usize)> {
    let mut parser = ValueParser::new(input);
    let value = parser.parse_value()?;
    Some((value, parser.pos))
}

struct ValueParser<'a> {
    input: &'a [u8],
    pos: usize,
}

impl<'a> ValueParser<'a> {
    fn new(input: &'a [u8]) -> Self {
        Self { input, pos: 0 }
    }

    fn parse_value(&mut self) -> Option<PdfValue> {
        self.skip_ws_comments();
        let byte = *self.input.get(self.pos)?;

        match byte {
            b'<' if self.peek(1) == Some(b'<') => self.parse_dictionary(),
            b'<' => self.parse_hex_string(),
            b'[' => self.parse_array(),
            b'(' => self.parse_literal_string(),
            b'/' => self.parse_name().map(PdfValue::Name),
            b't' if self.starts_with(b"true") => {
                self.pos += 4;
                Some(PdfValue::Boolean(true))
            }
            b'f' if self.starts_with(b"false") => {
                self.pos += 5;
                Some(PdfValue::Boolean(false))
            }
            b'n' if self.starts_with(b"null") => {
                self.pos += 4;
                Some(PdfValue::Null)
            }
            b'+' | b'-' | b'.' | b'0'..=b'9' => self.parse_number_or_reference(),
            _ => self.parse_raw_token().map(PdfValue::Raw),
        }
    }

    fn parse_dictionary(&mut self) -> Option<PdfValue> {
        self.pos += 2;
        let mut dictionary = PdfDictionary::new();

        loop {
            self.skip_ws_comments();
            if self.starts_with(b">>") {
                self.pos += 2;
                return Some(PdfValue::Dictionary(dictionary));
            }
            let key = self.parse_name()?;
            let value = self.parse_value()?;
            dictionary.insert(key, value);
        }
    }

    fn parse_array(&mut self) -> Option<PdfValue> {
        self.pos += 1;
        let mut values = Vec::new();
        loop {
            self.skip_ws_comments();
            if self.peek(0) == Some(b']') {
                self.pos += 1;
                return Some(PdfValue::Array(values));
            }
            values.push(self.parse_value()?);
        }
    }

    fn parse_literal_string(&mut self) -> Option<PdfValue> {
        self.pos += 1;
        let mut depth = 1usize;
        let mut output = Vec::new();

        while self.pos < self.input.len() {
            let byte = self.input[self.pos];
            self.pos += 1;
            match byte {
                b'\\' => {
                    let escaped = *self.input.get(self.pos)?;
                    self.pos += 1;
                    match escaped {
                        b'n' => output.push(b'\n'),
                        b'r' => output.push(b'\r'),
                        b't' => output.push(b'\t'),
                        b'b' => output.push(0x08),
                        b'f' => output.push(0x0C),
                        b'(' | b')' | b'\\' => output.push(escaped),
                        b'\r' => {
                            if self.peek(0) == Some(b'\n') {
                                self.pos += 1;
                            }
                        }
                        b'\n' => {}
                        other => output.push(other),
                    }
                }
                b'(' => {
                    depth += 1;
                    output.push(byte);
                }
                b')' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        return Some(PdfValue::String(decode_pdf_string(&output).text));
                    }
                    output.push(byte);
                }
                other => output.push(other),
            }
        }

        Some(PdfValue::String(decode_pdf_string(&output).text))
    }

    fn parse_hex_string(&mut self) -> Option<PdfValue> {
        self.pos += 1;
        let start = self.pos;
        while self.pos < self.input.len() && self.input[self.pos] != b'>' {
            self.pos += 1;
        }
        let end = self.pos;
        if self.peek(0) == Some(b'>') {
            self.pos += 1;
        }
        Some(PdfValue::HexString(
            String::from_utf8_lossy(&self.input[start..end])
                .split_whitespace()
                .collect::<String>(),
        ))
    }

    fn parse_name(&mut self) -> Option<String> {
        if self.peek(0) != Some(b'/') {
            return None;
        }
        self.pos += 1;
        let start = self.pos;
        while self.pos < self.input.len() && !is_delimiter(self.input[self.pos]) {
            self.pos += 1;
        }
        Some(decode_name(&self.input[start..self.pos]))
    }

    fn parse_number_or_reference(&mut self) -> Option<PdfValue> {
        let first_start = self.pos;
        let first_token = self.parse_number_token()?;
        let after_first = self.pos;

        if first_token.is_integer {
            self.skip_ws_comments();
            let second_start = self.pos;
            if let Some(second_token) = self.parse_number_token() {
                if second_token.is_integer {
                    self.skip_ws_comments();
                    if self.peek(0) == Some(b'R') {
                        self.pos += 1;
                        if let (Some(object), Some(generation)) = (
                            parse_u32(&self.input[first_token.start..first_token.end]),
                            parse_u16(&self.input[second_token.start..second_token.end]),
                        )
                        {
                            return Some(PdfValue::Reference(ObjectRef::new(object, generation)));
                        }
                    }
                }
            }
            self.pos = second_start;
        }

        self.pos = after_first;
        std::str::from_utf8(&self.input[first_token.start..first_token.end])
            .ok()
            .and_then(|token| token.parse::<f64>().ok())
            .map(PdfValue::Number)
            .or_else(|| {
                self.pos = first_start;
                self.parse_raw_token().map(PdfValue::Raw)
            })
    }

    fn parse_number_token(&mut self) -> Option<NumberToken> {
        self.skip_ws_comments();
        let start = self.pos;
        if matches!(self.peek(0), Some(b'+') | Some(b'-')) {
            self.pos += 1;
        }
        let mut has_digit = false;
        while matches!(self.peek(0), Some(b'0'..=b'9')) {
            has_digit = true;
            self.pos += 1;
        }
        let mut has_decimal = false;
        if self.peek(0) == Some(b'.') {
            has_decimal = true;
            self.pos += 1;
            while matches!(self.peek(0), Some(b'0'..=b'9')) {
                has_digit = true;
                self.pos += 1;
            }
        }
        if self.pos == start || (self.pos == start + 1 && matches!(self.input[start], b'+' | b'-'))
        {
            self.pos = start;
            return None;
        }
        Some(NumberToken {
            start,
            end: self.pos,
            is_integer: has_digit && !has_decimal,
        })
    }

    fn parse_raw_token(&mut self) -> Option<String> {
        let start = self.pos;
        while self.pos < self.input.len() && !is_delimiter(self.input[self.pos]) {
            self.pos += 1;
        }
        if self.pos == start {
            return None;
        }
        Some(String::from_utf8_lossy(&self.input[start..self.pos]).into_owned())
    }

    fn skip_ws_comments(&mut self) {
        self.pos = skip_whitespace_and_comments(self.input, self.pos);
    }

    fn starts_with(&self, bytes: &[u8]) -> bool {
        self.input
            .get(self.pos..)
            .is_some_and(|tail| tail.starts_with(bytes))
    }

    fn peek(&self, offset: usize) -> Option<u8> {
        self.input.get(self.pos + offset).copied()
    }
}

#[derive(Clone, Copy)]
struct NumberToken {
    start: usize,
    end: usize,
    is_integer: bool,
}

fn detect_page_count(parsed: &ParsedPdf) -> Option<u32> {
    let root = parsed.root_catalog()?;
    let pages = root.dictionary()?.get("Pages")?;
    if let Some(page_tree) = parsed.resolve_reference(pages) {
        if let Some(count) = page_tree
            .dictionary()?
            .get("Count")
            .and_then(PdfValue::as_usize)
        {
            return Some(count as u32);
        }
    }

    let page_count = parsed
        .objects
        .values()
        .filter(|object| {
            object
                .dictionary()
                .is_some_and(|dict| name_is(dict, "Type", "Page"))
        })
        .count();
    (page_count > 0).then_some(page_count as u32)
}

fn summary_dictionary(dictionary: &PdfDictionary) -> BTreeMap<String, String> {
    dictionary
        .iter()
        .map(|(key, value)| (key.clone(), value.summary()))
        .collect()
}

fn name_is(dictionary: &PdfDictionary, key: &str, expected: &str) -> bool {
    dictionary
        .get(key)
        .and_then(PdfValue::as_name)
        .is_some_and(|name| name == expected)
}

fn parse_u32(bytes: &[u8]) -> Option<u32> {
    std::str::from_utf8(bytes).ok()?.parse().ok()
}

fn parse_u16(bytes: &[u8]) -> Option<u16> {
    std::str::from_utf8(bytes).ok()?.parse().ok()
}

fn parse_u32_at(bytes: &[u8], pos: &mut usize) -> Option<u32> {
    let start = *pos;
    let mut value: u32 = 0;
    while let Some(byte) = bytes.get(*pos).copied() {
        if !byte.is_ascii_digit() {
            break;
        }
        value = value
            .checked_mul(10)?
            .checked_add((byte - b'0') as u32)?;
        *pos += 1;
    }
    (*pos > start).then_some(value)
}

fn parse_u16_at(bytes: &[u8], pos: &mut usize) -> Option<u16> {
    let value = parse_u32_at(bytes, pos)?;
    u16::try_from(value).ok()
}

fn parse_usize_at(bytes: &[u8], pos: &mut usize) -> Option<usize> {
    let start = *pos;
    let mut value: usize = 0;
    while let Some(byte) = bytes.get(*pos).copied() {
        if !byte.is_ascii_digit() {
            break;
        }
        value = value
            .checked_mul(10)?
            .checked_add((byte - b'0') as usize)?;
        *pos += 1;
    }
    (*pos > start).then_some(value)
}

fn decode_name(bytes: &[u8]) -> String {
    if !bytes.contains(&b'#') {
        return std::str::from_utf8(bytes)
            .map(str::to_owned)
            .unwrap_or_else(|_| String::from_utf8_lossy(bytes).into_owned());
    }

    let mut output = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'#' && index + 2 < bytes.len() {
            if let Ok(hex) = std::str::from_utf8(&bytes[index + 1..index + 3]) {
                if let Ok(value) = u8::from_str_radix(hex, 16) {
                    output.push(value);
                    index += 3;
                    continue;
                }
            }
        }
        output.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&output).into_owned()
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

fn skip_ascii_whitespace(input: &[u8], mut pos: usize) -> usize {
    while input
        .get(pos)
        .is_some_and(|byte| byte.is_ascii_whitespace())
    {
        pos += 1;
    }
    pos
}

fn is_delimiter(byte: u8) -> bool {
    byte.is_ascii_whitespace()
        || matches!(
            byte,
            b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%'
        )
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    memmem::find(haystack, needle)
}

fn is_line_start(bytes: &[u8], pos: usize) -> bool {
    pos == 0 || matches!(bytes.get(pos.saturating_sub(1)), Some(b'\n' | b'\r'))
}

fn line_end(bytes: &[u8], start: usize) -> usize {
    bytes[start..]
        .iter()
        .position(|byte| *byte == b'\n' || *byte == b'\r')
        .map(|relative| start + relative)
        .unwrap_or(bytes.len())
}

fn skip_blank_lines(bytes: &[u8], mut cursor: usize) -> usize {
    loop {
        let end = line_end(bytes, cursor);
        if bytes[cursor..end]
            .iter()
            .all(|byte| byte.is_ascii_whitespace())
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

fn parser_perf_logging_enabled() -> bool {
    std::env::var("PDF_DEBUGGER_PERF_LOG")
        .is_ok_and(|value| value == "1" || value.eq_ignore_ascii_case("true"))
}

fn parser_perf_log(stage: &str, file_name: &str, started: Instant) {
    if !parser_perf_logging_enabled() {
        return;
    }
    eprintln!(
        "[pdf-debugger parser] stage={} file={} elapsed_ms={}",
        stage,
        file_name,
        started.elapsed().as_millis()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_pdf_metadata() {
        let bytes = minimal_pdf();
        let parsed = parse_bytes(&bytes, "minimal.pdf");

        assert_eq!(parsed.metadata.pdf_version.as_deref(), Some("1.4"));
        assert_eq!(parsed.metadata.page_count, Some(1));
        assert_eq!(parsed.metadata.object_count, 5);
        assert_eq!(parsed.metadata.stream_count, 1);
        assert_eq!(parsed.metadata.root, Some(ObjectRef::new(1, 0)));
    }

    #[test]
    fn parse_file_rejects_files_above_full_parse_limit() {
        let directory =
            std::env::temp_dir().join(format!("pdf-debugger-large-{}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("create temp directory");
        let path = directory.join("large.pdf");
        let file = std::fs::File::create(&path).expect("create sparse PDF");
        file.set_len(MAX_FULL_PARSE_FILE_SIZE + 1)
            .expect("set sparse PDF length");

        let error = parse_file(&path).expect_err("large files should fail before full parse");

        match error {
            PdfDebuggerError::FileTooLarge { size, limit } => {
                assert_eq!(size, MAX_FULL_PARSE_FILE_SIZE + 1);
                assert_eq!(limit, MAX_FULL_PARSE_FILE_SIZE);
            }
            other => panic!("expected FileTooLarge, got {other:?}"),
        }

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn parses_references_inside_values() {
        let value = parse_pdf_value(b"<< /Root 1 0 R /Kids [2 0 R] >>").unwrap();
        let dict = value.as_dictionary().unwrap();
        assert_eq!(dict["Root"].as_reference(), Some(ObjectRef::new(1, 0)));
    }

    #[test]
    fn gui_parse_skips_invalid_xref_offsets_without_full_scan() {
        let bytes = b"%PDF-1.7
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj
2 0 obj
<< /Type /Pages /Count 0 /Kids [] >>
endobj
xref
0 3
0000000000 65535 f
0000000009 00000 n
0000000060 00000 n
trailer
<< /Size 3 /Root 1 0 R >>
startxref
111
%%EOF
";

        let default = parse_bytes(bytes, "xref-fallback.pdf");
        assert!(default.object(ObjectRef::new(2, 0)).is_some());

        let gui = parse_bytes_with_options(bytes, "xref-skip.pdf", GUI_PARSE_OPTIONS);
        assert!(gui.object(ObjectRef::new(1, 0)).is_some());
        assert!(gui.object(ObjectRef::new(2, 0)).is_none());
        assert!(gui.warnings.iter().any(|warning| warning
            .message
            .contains("Skipped 1 in-use xref object")));
    }

    #[test]
    fn decodes_utf16be_literal_pdf_strings() {
        let value = parse_pdf_value(b"(\xfe\xff\x00M\x00i\x00c\x00r\x00o\x00s\x00o\x00f\x00t\x00 \x00W\x00o\x00r\x00d\x00 \x002\x000\x001\x006)").unwrap();
        assert_eq!(value.summary(), "(Microsoft Word 2016)");
    }

    #[test]
    fn summarizes_utf16be_hex_pdf_strings() {
        let value = parse_pdf_value(
            b"<FEFF004D006900630072006F0073006F0066007400200057006F0072006400200032003000310036>",
        )
        .unwrap();
        assert_eq!(value.summary(), "(Microsoft Word 2016)");
    }

    #[test]
    fn decodes_info_dictionary_strings_in_metadata_summary() {
        let bytes = info_string_pdf();
        let parsed = parse_bytes(&bytes, "info-string.pdf");

        assert_eq!(
            parsed
                .metadata
                .info_summary
                .get("Creator")
                .map(String::as_str),
            Some("(Microsoft Word 2016)")
        );
        assert_eq!(
            parsed
                .metadata
                .info_summary
                .get("Producer")
                .map(String::as_str),
            Some("(Microsoft Word 2016)")
        );
    }

    #[test]
    fn parses_xref_stream_trailer_and_object_stream_members() {
        let bytes = xref_stream_with_object_stream_pdf();
        let parsed = parse_bytes(&bytes, "xref-object-stream.pdf");

        assert_eq!(parsed.metadata.pdf_version.as_deref(), Some("1.7"));
        assert_eq!(parsed.metadata.root, Some(ObjectRef::new(1, 0)));
        assert_eq!(parsed.metadata.info, Some(ObjectRef::new(3, 0)));
        assert_eq!(parsed.metadata.page_count, Some(1));
        assert!(parsed.metadata.has_xref_stream);
        assert!(parsed.metadata.has_object_stream);
        assert!(parsed.trailer.is_some());
        assert!(parsed.object(ObjectRef::new(2, 0)).is_some());
        assert!(parsed.object(ObjectRef::new(7, 0)).is_some());
        assert!(parsed.object(ObjectRef::new(13, 0)).is_some());

        let root = parsed.root_catalog().expect("root catalog");
        assert_eq!(
            root.dictionary()
                .and_then(|dict| dict.get("Pages"))
                .and_then(PdfValue::as_reference),
            Some(ObjectRef::new(2, 0))
        );
        let acroform = parsed
            .object(ObjectRef::new(7, 0))
            .and_then(PdfObject::dictionary)
            .expect("AcroForm dictionary from object stream");
        assert_eq!(
            acroform
                .get("Fields")
                .and_then(PdfValue::as_array)
                .and_then(|fields| fields.first())
                .and_then(PdfValue::as_reference),
            Some(ObjectRef::new(13, 0))
        );
    }

    fn minimal_pdf() -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"%PDF-1.4\n");
        let offsets = [
            push_object(&mut bytes, b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n"),
            push_object(&mut bytes, b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 200 200] >>\nendobj\n"),
            push_object(&mut bytes, b"3 0 obj\n<< /Type /Page /Parent 2 0 R /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>\nendobj\n"),
            push_object(&mut bytes, b"4 0 obj\n<< /Length 23 >>\nstream\nBT /F1 12 Tf (Hi) Tj ET\nendstream\nendobj\n"),
            push_object(&mut bytes, b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n"),
        ];
        let xref_offset = bytes.len();
        bytes.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
        for offset in offsets {
            bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        bytes.extend_from_slice(
            format!("trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n")
                .as_bytes(),
        );
        bytes
    }

    fn info_string_pdf() -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"%PDF-1.4\n");
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
                b"3 0 obj\n<< /Type /Page /Parent 2 0 R >>\nendobj\n",
            ),
            push_object(
                &mut bytes,
                b"4 0 obj\n<< /Creator (\xfe\xff\x00M\x00i\x00c\x00r\x00o\x00s\x00o\x00f\x00t\x00 \x00W\x00o\x00r\x00d\x00 \x002\x000\x001\x006) /Producer <FEFF004D006900630072006F0073006F0066007400200057006F0072006400200032003000310036> >>\nendobj\n",
            ),
        ];
        let xref = bytes.len();
        bytes.extend_from_slice(b"xref\n0 5\n0000000000 65535 f \n");
        for offset in offsets {
            bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        bytes.extend_from_slice(
            format!("trailer\n<< /Size 5 /Root 1 0 R /Info 4 0 R >>\nstartxref\n{xref}\n%%EOF\n")
                .as_bytes(),
        );
        bytes
    }

    fn xref_stream_with_object_stream_pdf() -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"%PDF-1.7\n");
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

    fn push_xref_stream_entry(bytes: &mut Vec<u8>, entry_type: u8, field_2: usize, field_3: usize) {
        bytes.push(entry_type);
        bytes.extend_from_slice(&(field_2 as u32).to_be_bytes());
        bytes.extend_from_slice(&(field_3 as u16).to_be_bytes());
    }

    fn push_object(bytes: &mut Vec<u8>, object: &[u8]) -> usize {
        let offset = bytes.len();
        bytes.extend_from_slice(object);
        offset
    }
}
