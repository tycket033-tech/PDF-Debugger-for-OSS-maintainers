use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct FixtureSet {
    pub dir: PathBuf,
    pub minimal: PathBuf,
    pub sparse_large: PathBuf,
    pub missing_reference: PathBuf,
    pub decode_failure: PathBuf,
    pub unknown_operator: PathBuf,
    pub invalid_xref_offset: PathBuf,
    pub xref_stream: PathBuf,
    pub object_stream: PathBuf,
    pub incremental_prev: PathBuf,
    pub hybrid_xref: PathBuf,
}

impl FixtureSet {
    pub fn create(test_name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "pdf-debugger-{test_name}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time after Unix epoch")
                .as_nanos()
        ));
        fs::create_dir_all(&dir).expect("create fixture directory");

        let minimal = write_fixture(&dir, "minimal.pdf", minimal_pdf(b"BT /F1 12 Tf (Hi) Tj ET"));
        let sparse_large = write_sparse_large_fixture(&dir, "sparse-large.pdf");
        let missing_reference =
            write_fixture(&dir, "missing-reference.pdf", missing_reference_pdf());
        let decode_failure = write_fixture(&dir, "decode-failure.pdf", decode_failure_pdf());
        let unknown_operator = write_fixture(
            &dir,
            "unknown-operator.pdf",
            minimal_pdf(b"BT /F1 12 Tf (Hi) Tj ZZ ET"),
        );
        let invalid_xref_offset =
            write_fixture(&dir, "invalid-xref-offset.pdf", invalid_xref_offset_pdf());
        let xref_stream = write_fixture(&dir, "xref-stream.pdf", xref_stream_pdf());
        let object_stream = write_fixture(&dir, "object-stream.pdf", object_stream_pdf());
        let incremental_prev = write_fixture(&dir, "incremental-prev.pdf", incremental_prev_pdf());
        let hybrid_xref = write_fixture(&dir, "hybrid-xref.pdf", hybrid_xref_pdf());

        Self {
            dir,
            minimal,
            sparse_large,
            missing_reference,
            decode_failure,
            unknown_operator,
            invalid_xref_offset,
            xref_stream,
            object_stream,
            incremental_prev,
            hybrid_xref,
        }
    }
}

impl Drop for FixtureSet {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn write_fixture(dir: &Path, name: &str, bytes: Vec<u8>) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, bytes).expect("write PDF fixture");
    path
}

fn write_sparse_large_fixture(dir: &Path, name: &str) -> PathBuf {
    let path = dir.join(name);
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

    let mut file = fs::File::create(&path).expect("create sparse large fixture");
    use std::io::{Seek, SeekFrom, Write};
    file.write_all(&bytes).expect("write sparse fixture prefix");
    let xref_offset = pdf_debugger::pdf_parser::MAX_FULL_PARSE_FILE_SIZE + 4096;
    file.seek(SeekFrom::Start(xref_offset))
        .expect("seek sparse xref");
    file.write_all(b"xref\n0 6\n0000000000 65535 f \n")
        .expect("write xref header");
    for offset in offsets {
        file.write_all(format!("{offset:010} 00000 n \n").as_bytes())
            .expect("write xref entry");
    }
    file.write_all(
        format!("trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n").as_bytes(),
    )
    .expect("write sparse trailer");
    path
}

fn minimal_pdf(content: &[u8]) -> Vec<u8> {
    let mut pdf = PdfBuilder::new();
    pdf.add_object(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    pdf.add_object(
        b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 200 200] >>\nendobj\n",
    );
    pdf.add_object(b"3 0 obj\n<< /Type /Page /Parent 2 0 R /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>\nendobj\n");
    pdf.add_object(&stream_object(4, content, b"<< /Length {len} >>"));
    pdf.add_object(b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n");
    pdf.finish(1, 6, false)
}

fn missing_reference_pdf() -> Vec<u8> {
    let mut pdf = PdfBuilder::new();
    pdf.add_object(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    pdf.add_object(
        b"2 0 obj\n<< /Type /Pages /Kids [99 0 R] /Count 1 /MediaBox [0 0 200 200] >>\nendobj\n",
    );
    pdf.finish(1, 3, false)
}

fn decode_failure_pdf() -> Vec<u8> {
    let mut pdf = PdfBuilder::new();
    pdf.add_object(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    pdf.add_object(
        b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 200 200] >>\nendobj\n",
    );
    pdf.add_object(b"3 0 obj\n<< /Type /Page /Parent 2 0 R /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>\nendobj\n");
    pdf.add_object(&stream_object(
        4,
        b"not valid zlib bytes",
        b"<< /Length {len} /Filter /FlateDecode >>",
    ));
    pdf.add_object(b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n");
    pdf.finish(1, 6, false)
}

fn invalid_xref_offset_pdf() -> Vec<u8> {
    let mut pdf = PdfBuilder::new();
    pdf.add_object(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    pdf.add_object(
        b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 200 200] >>\nendobj\n",
    );
    pdf.add_object(b"3 0 obj\n<< /Type /Page /Parent 2 0 R /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>\nendobj\n");
    pdf.add_object(&stream_object(
        4,
        b"BT /F1 12 Tf (Hi) Tj ET",
        b"<< /Length {len} >>",
    ));
    pdf.add_object(b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n");
    pdf.finish(1, 6, true)
}

fn xref_stream_pdf() -> Vec<u8> {
    let mut bytes = b"%PDF-1.5\n".to_vec();
    let object_1 = push_object(
        &mut bytes,
        b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n",
    );
    let object_2 = push_object(
        &mut bytes,
        b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 200 200] >>\nendobj\n",
    );
    let object_3 = push_object(
        &mut bytes,
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /Contents 4 0 R >>\nendobj\n",
    );
    let object_4 = push_object(
        &mut bytes,
        b"4 0 obj\n<< /Length 23 >>\nstream\nBT /F1 12 Tf (Hi) Tj ET\nendstream\nendobj\n",
    );
    let xref_offset = bytes.len();
    let mut xref_data = Vec::new();
    push_xref_stream_entry(&mut xref_data, 0, 0, 65535);
    for offset in [object_1, object_2, object_3, object_4, xref_offset] {
        push_xref_stream_entry(&mut xref_data, 1, offset, 0);
    }
    bytes.extend_from_slice(
        format!(
            "5 0 obj\n<< /Type /XRef /Size 6 /Root 1 0 R /W [1 4 2] /Length {} >>\nstream\n",
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

fn object_stream_pdf() -> Vec<u8> {
    let mut bytes = b"%PDF-1.5\n".to_vec();
    let object_1 = push_object(
        &mut bytes,
        b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n",
    );
    let object_2 = push_object(
        &mut bytes,
        b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 200 200] >>\nendobj\n",
    );
    let object_3 = push_object(
        &mut bytes,
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>\nendobj\n",
    );
    let object_4 = push_object(
        &mut bytes,
        b"4 0 obj\n<< /Length 23 >>\nstream\nBT /F1 12 Tf (Hi) Tj ET\nendstream\nendobj\n",
    );
    let compressed = b"5 0 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>";
    let object_6 = push_object(
        &mut bytes,
        format!(
            "6 0 obj\n<< /Type /ObjStm /N 1 /First 4 /Length {} >>\nstream\n",
            compressed.len()
        )
        .as_bytes(),
    );
    bytes.extend_from_slice(compressed);
    bytes.extend_from_slice(b"\nendstream\nendobj\n");

    let xref_offset = bytes.len();
    let mut xref_data = Vec::new();
    push_xref_stream_entry(&mut xref_data, 0, 0, 65535);
    for offset in [object_1, object_2, object_3, object_4] {
        push_xref_stream_entry(&mut xref_data, 1, offset, 0);
    }
    push_xref_stream_entry(&mut xref_data, 2, 6, 0);
    push_xref_stream_entry(&mut xref_data, 1, object_6, 0);
    push_xref_stream_entry(&mut xref_data, 1, xref_offset, 0);
    bytes.extend_from_slice(
        format!(
            "7 0 obj\n<< /Type /XRef /Size 8 /Root 1 0 R /W [1 4 2] /Length {} >>\nstream\n",
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

fn incremental_prev_pdf() -> Vec<u8> {
    let mut bytes = b"%PDF-1.4\n".to_vec();
    let object_1 = push_object(
        &mut bytes,
        b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n",
    );
    let object_2 = push_object(
        &mut bytes,
        b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 200 200] >>\nendobj\n",
    );
    let object_3 = push_object(
        &mut bytes,
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /Resources << /Font << /F1 5 0 R /F2 6 0 R >> >> /Contents 4 0 R >>\nendobj\n",
    );
    let object_4 = push_object(
        &mut bytes,
        b"4 0 obj\n<< /Length 23 >>\nstream\nBT /F1 12 Tf (Hi) Tj ET\nendstream\nendobj\n",
    );
    let object_5_original = push_object(
        &mut bytes,
        b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n",
    );
    let first_xref = bytes.len();
    bytes.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
    for offset in [object_1, object_2, object_3, object_4, object_5_original] {
        bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    bytes.extend_from_slice(
        format!("trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{first_xref}\n%%EOF\n").as_bytes(),
    );

    let object_5_updated = push_object(
        &mut bytes,
        b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Courier /Updated true >>\nendobj\n",
    );
    let object_6 = push_object(
        &mut bytes,
        b"6 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Times-Roman /Added true >>\nendobj\n",
    );
    let second_xref = bytes.len();
    bytes.extend_from_slice(b"xref\n5 2\n");
    for offset in [object_5_updated, object_6] {
        bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    bytes.extend_from_slice(
        format!(
            "trailer\n<< /Size 7 /Root 1 0 R /Prev {first_xref} >>\nstartxref\n{second_xref}\n%%EOF\n"
        )
        .as_bytes(),
    );
    bytes
}

fn hybrid_xref_pdf() -> Vec<u8> {
    let mut bytes = b"%PDF-1.5\n".to_vec();
    let object_1 = push_object(
        &mut bytes,
        b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n",
    );
    let object_2 = push_object(
        &mut bytes,
        b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 200 200] >>\nendobj\n",
    );
    let object_3 = push_object(
        &mut bytes,
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>\nendobj\n",
    );
    let object_4 = push_object(
        &mut bytes,
        b"4 0 obj\n<< /Length 23 >>\nstream\nBT /F1 12 Tf (Hi) Tj ET\nendstream\nendobj\n",
    );
    let compressed = b"5 0 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>";
    let object_6 = push_object(
        &mut bytes,
        format!(
            "6 0 obj\n<< /Type /ObjStm /N 1 /First 4 /Length {} >>\nstream\n",
            compressed.len()
        )
        .as_bytes(),
    );
    bytes.extend_from_slice(compressed);
    bytes.extend_from_slice(b"\nendstream\nendobj\n");

    let xref_stream_offset = bytes.len();
    let mut xref_data = Vec::new();
    push_xref_stream_entry(&mut xref_data, 2, 6, 0);
    bytes.extend_from_slice(
        format!(
            "7 0 obj\n<< /Type /XRef /Size 8 /Index [5 1] /W [1 4 2] /Length {} >>\nstream\n",
            xref_data.len()
        )
        .as_bytes(),
    );
    bytes.extend_from_slice(&xref_data);
    bytes.extend_from_slice(b"\nendstream\nendobj\n");

    let classic_xref = bytes.len();
    bytes.extend_from_slice(b"xref\n0 8\n0000000000 65535 f \n");
    for offset in [object_1, object_2, object_3, object_4] {
        bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    bytes.extend_from_slice(b"0000000000 00000 f \n");
    for offset in [object_6, xref_stream_offset] {
        bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    bytes.extend_from_slice(
        format!(
            "trailer\n<< /Size 8 /Root 1 0 R /XRefStm {xref_stream_offset} >>\nstartxref\n{classic_xref}\n%%EOF\n"
        )
        .as_bytes(),
    );
    bytes
}

fn stream_object(object_number: u32, content: &[u8], dictionary_template: &[u8]) -> Vec<u8> {
    let dictionary =
        String::from_utf8_lossy(dictionary_template).replace("{len}", &content.len().to_string());
    let mut object = Vec::new();
    object.extend_from_slice(format!("{object_number} 0 obj\n{dictionary}\nstream\n").as_bytes());
    object.extend_from_slice(content);
    object.extend_from_slice(b"\nendstream\nendobj\n");
    object
}

struct PdfBuilder {
    bytes: Vec<u8>,
    offsets: Vec<usize>,
}

impl PdfBuilder {
    fn new() -> Self {
        Self {
            bytes: b"%PDF-1.4\n".to_vec(),
            offsets: Vec::new(),
        }
    }

    fn add_object(&mut self, object: &[u8]) {
        self.offsets.push(self.bytes.len());
        self.bytes.extend_from_slice(object);
    }

    fn finish(mut self, root_object: u32, size: usize, corrupt_last_xref: bool) -> Vec<u8> {
        let xref_offset = self.bytes.len();
        self.bytes
            .extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
        for (index, offset) in self.offsets.iter().copied().enumerate() {
            let offset = if corrupt_last_xref && index + 1 == self.offsets.len() {
                offset.saturating_add(7)
            } else {
                offset
            };
            self.bytes
                .extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        self.bytes.extend_from_slice(
            format!("trailer\n<< /Size {size} /Root {root_object} 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n")
                .as_bytes(),
        );
        self.bytes
    }
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
