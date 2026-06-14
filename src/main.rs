use clap::{Parser, Subcommand};
use pdf_debugger::pdf_model::{DiagnosticSummary, ObjectRef};
use pdf_debugger::report::{
    build_lazy_report, build_lazy_report_with_enrichments, build_report, render_json_report,
    render_lazy_json_report, render_lazy_markdown_report, render_markdown_report,
};
use pdf_debugger::stream_decode::decode_stream;
use pdf_debugger::{
    build_lazy_page_index, build_lazy_page_list, build_object_tree, hex_dump,
    inspect_content_stream, inspect_lazy_object, inspect_object, inspect_stream,
    lazy_pdf_to_object_tree, open_lazy_pdf, parse_file, read_lazy_stream_decoded_bytes,
    read_lazy_stream_for_content, read_lazy_stream_raw_bytes, run_diagnostics,
    run_lazy_deep_diagnostics, run_lazy_fast_diagnostics, view_lazy_stream,
    LazyDeepDiagnosticRequest, LazyReportEnrichment, PdfDebuggerError, Result,
};
use std::io::Write;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Debug, Parser)]
#[command(name = "pdf-debugger")]
#[command(about = "Developer-focused PDF inspection CLI for OSS maintainers")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Inspect {
        file: PathBuf,
        #[arg(long)]
        structure: bool,
    },
    LazyInspect {
        file: PathBuf,
        #[arg(long)]
        structure: bool,
        #[arg(long)]
        pages: bool,
        #[arg(long)]
        page_list: bool,
        #[arg(long)]
        diagnostics: bool,
        #[arg(long, value_names = ["OBJECT", "GENERATION"], num_args = 2)]
        object: Option<Vec<u32>>,
        #[arg(long, value_names = ["OBJECT", "GENERATION"], num_args = 2)]
        stream: Option<Vec<u32>>,
        #[arg(long)]
        deep_page: Option<u32>,
        #[arg(long, value_names = ["OBJECT", "GENERATION"], num_args = 2)]
        deep_stream: Option<Vec<u32>>,
        #[arg(long, value_names = ["OBJECT", "GENERATION"], num_args = 2)]
        deep_object: Option<Vec<u32>>,
    },
    Check {
        file: PathBuf,
        #[arg(long)]
        json: Option<PathBuf>,
        #[arg(long)]
        markdown: Option<PathBuf>,
        #[arg(long)]
        lazy_page: Vec<u32>,
        #[arg(long, value_names = ["OBJECT", "GENERATION"], num_args = 2)]
        lazy_stream: Vec<u32>,
        #[arg(long, value_names = ["OBJECT", "GENERATION"], num_args = 2)]
        lazy_object: Vec<u32>,
    },
    DumpObject {
        file: PathBuf,
        object: u32,
        generation: u16,
        #[arg(long)]
        json: bool,
    },
    DumpStream {
        file: PathBuf,
        object: u32,
        generation: u16,
        #[arg(long)]
        decoded: Option<PathBuf>,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        hex: bool,
        #[arg(long)]
        content_json: bool,
    },
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<ExitCode> {
    let cli = Cli::parse();

    match cli.command {
        Command::Inspect { file, structure } => inspect(file, structure),
        Command::LazyInspect {
            file,
            structure,
            pages,
            page_list,
            diagnostics,
            object,
            stream,
            deep_page,
            deep_stream,
            deep_object,
        } => lazy_inspect(
            file,
            structure,
            pages,
            page_list,
            diagnostics,
            object,
            stream,
            deep_page,
            deep_stream,
            deep_object,
        ),
        Command::Check {
            file,
            json,
            markdown,
            lazy_page,
            lazy_stream,
            lazy_object,
        } => check(file, json, markdown, lazy_page, lazy_stream, lazy_object),
        Command::DumpObject {
            file,
            object,
            generation,
            json,
        } => dump_object(file, ObjectRef::new(object, generation), json),
        Command::DumpStream {
            file,
            object,
            generation,
            decoded,
            json,
            hex,
            content_json,
        } => dump_stream(
            file,
            ObjectRef::new(object, generation),
            decoded,
            json,
            hex,
            content_json,
        ),
    }
}

fn inspect(file: PathBuf, structure: bool) -> Result<ExitCode> {
    let pdf = parse_file(&file)?;
    if structure {
        println!(
            "{}",
            serde_json::to_string_pretty(&build_object_tree(&pdf))?
        );
    } else {
        println!("{}", serde_json::to_string_pretty(&pdf.metadata)?);
    }
    Ok(ExitCode::SUCCESS)
}

fn lazy_inspect(
    file: PathBuf,
    structure: bool,
    pages: bool,
    page_list: bool,
    diagnostics: bool,
    object: Option<Vec<u32>>,
    stream: Option<Vec<u32>>,
    deep_page: Option<u32>,
    deep_stream: Option<Vec<u32>>,
    deep_object: Option<Vec<u32>>,
) -> Result<ExitCode> {
    let selected_modes = usize::from(structure)
        + usize::from(pages)
        + usize::from(page_list)
        + usize::from(diagnostics)
        + usize::from(object.is_some())
        + usize::from(stream.is_some())
        + usize::from(deep_page.is_some())
        + usize::from(deep_stream.is_some())
        + usize::from(deep_object.is_some());
    if selected_modes > 1 {
        return Err(PdfDebuggerError::ConflictingOutputModes {
            message:
                "choose only one of --structure, --pages, --diagnostics, --object, --stream, --deep-page, --deep-stream, or --deep-object"
                    .to_string(),
        });
    }

    if let Some(parts) = object {
        let object_number = parts[0];
        let generation = u16::try_from(parts[1]).map_err(|_| PdfDebuggerError::Parse {
            offset: 0,
            message: "generation number is outside u16 range".to_string(),
        })?;
        let inspection = inspect_lazy_object(&file, ObjectRef::new(object_number, generation))?;
        println!("{}", serde_json::to_string_pretty(&inspection)?);
        return Ok(ExitCode::SUCCESS);
    }

    if let Some(parts) = stream {
        let object_number = parts[0];
        let generation = u16::try_from(parts[1]).map_err(|_| PdfDebuggerError::Parse {
            offset: 0,
            message: "generation number is outside u16 range".to_string(),
        })?;
        let view = view_lazy_stream(&file, ObjectRef::new(object_number, generation))?;
        println!("{}", serde_json::to_string_pretty(&view)?);
        return Ok(ExitCode::SUCCESS);
    }

    let deep_request = if let Some(page_number) = deep_page {
        Some(LazyDeepDiagnosticRequest::Page { page_number })
    } else if let Some(parts) = deep_stream {
        Some(LazyDeepDiagnosticRequest::Stream {
            reference: object_ref_from_parts(&parts)?,
        })
    } else if let Some(parts) = deep_object {
        Some(LazyDeepDiagnosticRequest::Object {
            reference: object_ref_from_parts(&parts)?,
        })
    } else {
        None
    };
    if let Some(request) = deep_request {
        let mut pdf = open_lazy_pdf(&file)?;
        let enrichment = run_lazy_deep_diagnostics(&mut pdf, request)?;
        println!("{}", serde_json::to_string_pretty(&enrichment)?);
        return Ok(ExitCode::SUCCESS);
    }

    let mut pdf = open_lazy_pdf(&file)?;
    if structure {
        println!(
            "{}",
            serde_json::to_string_pretty(&lazy_pdf_to_object_tree(&pdf))?
        );
    } else if pages {
        println!(
            "{}",
            serde_json::to_string_pretty(&build_lazy_page_index(&mut pdf))?
        );
    } else if page_list {
        println!(
            "{}",
            serde_json::to_string_pretty(&build_lazy_page_list(&mut pdf))?
        );
    } else if diagnostics {
        let findings = run_lazy_fast_diagnostics(&mut pdf);
        let report = LazyDiagnosticsCliReport {
            diagnostics: DiagnosticSummary::from_findings(&findings),
            findings,
        };
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("{}", serde_json::to_string_pretty(&pdf.metadata)?);
    }
    Ok(ExitCode::SUCCESS)
}

#[derive(serde::Serialize)]
struct LazyDiagnosticsCliReport {
    diagnostics: DiagnosticSummary,
    findings: Vec<pdf_debugger::Finding>,
}

fn check(
    file: PathBuf,
    json: Option<PathBuf>,
    markdown: Option<PathBuf>,
    lazy_page: Vec<u32>,
    lazy_stream: Vec<u32>,
    lazy_object: Vec<u32>,
) -> Result<ExitCode> {
    let should_print = json.is_none() && markdown.is_none();

    match parse_file(&file) {
        Ok(pdf) => {
            let findings = run_diagnostics(&pdf);
            let summary = DiagnosticSummary::from_findings(&findings);
            let report = build_report(&pdf, findings);

            if let Some(path) = json {
                std::fs::write(path, render_json_report(&report)?)?;
            }
            if let Some(path) = markdown {
                std::fs::write(path, render_markdown_report(&report))?;
            }

            if should_print {
                println!("{}", render_json_report(&report)?);
            }

            if summary.has_errors() {
                Ok(ExitCode::from(1))
            } else {
                Ok(ExitCode::SUCCESS)
            }
        }
        Err(PdfDebuggerError::FileTooLarge { .. }) => {
            let mut document = open_lazy_pdf(&file)?;
            let enrichments =
                build_lazy_report_enrichments(&mut document, lazy_page, lazy_stream, lazy_object)?;
            let report = if enrichments.is_empty() {
                build_lazy_report(&mut document)
            } else {
                build_lazy_report_with_enrichments(&mut document, enrichments)
            };

            if let Some(path) = json {
                std::fs::write(path, render_lazy_json_report(&report)?)?;
            }
            if let Some(path) = markdown {
                std::fs::write(path, render_lazy_markdown_report(&report))?;
            }

            if should_print {
                println!("{}", render_lazy_json_report(&report)?);
            }

            if report.diagnostics.has_errors() {
                Ok(ExitCode::from(1))
            } else {
                Ok(ExitCode::SUCCESS)
            }
        }
        Err(error) => Err(error),
    }
}

fn build_lazy_report_enrichments(
    document: &mut pdf_debugger::LazyPdfDocument,
    lazy_pages: Vec<u32>,
    lazy_stream_parts: Vec<u32>,
    lazy_object_parts: Vec<u32>,
) -> Result<Vec<LazyReportEnrichment>> {
    let mut enrichments = Vec::new();
    for page_number in lazy_pages {
        enrichments.push(run_lazy_deep_diagnostics(
            document,
            LazyDeepDiagnosticRequest::Page { page_number },
        )?);
    }
    for parts in lazy_stream_parts.chunks_exact(2) {
        enrichments.push(run_lazy_deep_diagnostics(
            document,
            LazyDeepDiagnosticRequest::Stream {
                reference: object_ref_from_parts(parts)?,
            },
        )?);
    }
    for parts in lazy_object_parts.chunks_exact(2) {
        enrichments.push(run_lazy_deep_diagnostics(
            document,
            LazyDeepDiagnosticRequest::Object {
                reference: object_ref_from_parts(parts)?,
            },
        )?);
    }
    Ok(enrichments)
}

fn object_ref_from_parts(parts: &[u32]) -> Result<ObjectRef> {
    let object_number = parts[0];
    let generation = u16::try_from(parts[1]).map_err(|_| PdfDebuggerError::Parse {
        offset: 0,
        message: "generation number is outside u16 range".to_string(),
    })?;
    Ok(ObjectRef::new(object_number, generation))
}

fn dump_object(file: PathBuf, reference: ObjectRef, json: bool) -> Result<ExitCode> {
    let pdf = parse_file(&file)?;
    let object = pdf
        .object(reference)
        .ok_or(PdfDebuggerError::ObjectNotFound { reference })?;
    if json {
        println!("{}", serde_json::to_string_pretty(&inspect_object(object))?);
    } else {
        let raw = if object.raw_bytes.is_empty() && object.raw_range.len() > 0 {
            read_file_range(&file, object.raw_range.start, object.raw_range.len())?
        } else {
            object.raw_bytes.clone()
        };
        std::io::stdout().write_all(&raw)?;
    }
    Ok(ExitCode::SUCCESS)
}

fn read_file_range(path: &PathBuf, start: usize, length: usize) -> std::io::Result<Vec<u8>> {
    let mut file = std::fs::File::open(path)?;
    file.seek(SeekFrom::Start(start as u64))?;
    let mut bytes = vec![0; length];
    let read = file.read(&mut bytes)?;
    bytes.truncate(read);
    Ok(bytes)
}

fn dump_stream(
    file: PathBuf,
    reference: ObjectRef,
    decoded: Option<PathBuf>,
    json: bool,
    hex: bool,
    content_json: bool,
) -> Result<ExitCode> {
    let selected_modes = usize::from(decoded.is_some())
        + usize::from(json)
        + usize::from(hex)
        + usize::from(content_json);
    if selected_modes > 1 {
        return Err(PdfDebuggerError::ConflictingOutputModes {
            message: "choose only one of --decoded, --json, --hex, or --content-json".to_string(),
        });
    }

    let pdf = match parse_file(&file) {
        Ok(pdf) => Some(pdf),
        Err(PdfDebuggerError::FileTooLarge { .. }) => None,
        Err(error) => return Err(error),
    };

    if let Some(pdf) = pdf {
        let stream = pdf
            .stream(reference)
            .ok_or(PdfDebuggerError::StreamNotFound { reference })?;

        if json {
            println!(
                "{}",
                serde_json::to_string_pretty(&inspect_stream(reference, stream))?
            );
            return Ok(ExitCode::SUCCESS);
        }

        if hex {
            print!("{}", hex_dump(&stream.raw_bytes));
            return Ok(ExitCode::SUCCESS);
        }

        if content_json {
            match inspect_content_stream(reference, stream) {
                Ok(view) => {
                    println!("{}", serde_json::to_string_pretty(&view)?);
                    return Ok(ExitCode::SUCCESS);
                }
                Err(error) => {
                    for issue in &error.issues {
                        eprintln!(
                            "stream {reference}: /{} {}: {}",
                            issue.filter,
                            issue.kind_label(),
                            issue.message
                        );
                    }
                    eprintln!("{}", error.message);
                    return Ok(ExitCode::from(1));
                }
            }
        }

        if let Some(path) = decoded {
            let decode = decode_stream(stream);
            if decode.has_issues() {
                for issue in decode.issues {
                    eprintln!(
                        "stream {reference}: /{} {}: {}",
                        issue.filter,
                        issue.kind_label(),
                        issue.message
                    );
                }
                eprintln!(
                    "raw stream bytes remain available with `pdf-debugger dump-stream <file> {} {}`",
                    reference.object, reference.generation
                );
                return Ok(ExitCode::from(1));
            }
            std::fs::write(path, decode.decoded)?;
            return Ok(ExitCode::SUCCESS);
        }

        std::io::stdout().write_all(&stream.raw_bytes)?;
        return Ok(ExitCode::SUCCESS);
    }

    dump_stream_lazy(file, reference, decoded, json, hex, content_json)
}

fn dump_stream_lazy(
    file: PathBuf,
    reference: ObjectRef,
    decoded: Option<PathBuf>,
    json: bool,
    hex: bool,
    content_json: bool,
) -> Result<ExitCode> {
    if json {
        let view = view_lazy_stream(&file, reference)?;
        println!("{}", serde_json::to_string_pretty(&view)?);
        return Ok(ExitCode::SUCCESS);
    }

    if hex {
        let view = view_lazy_stream(&file, reference)?;
        print!("{}", view.hex_text);
        if view.hex_text_truncated {
            eprintln!(
                "stream {reference}: hex output truncated to {} preview bytes; use raw export for full bytes",
                view.preview_limit
            );
        }
        return Ok(ExitCode::SUCCESS);
    }

    if content_json {
        let stream = read_lazy_stream_for_content(&file, reference)?;
        match inspect_content_stream(reference, &stream) {
            Ok(view) => {
                println!("{}", serde_json::to_string_pretty(&view)?);
                return Ok(ExitCode::SUCCESS);
            }
            Err(error) => {
                for issue in &error.issues {
                    eprintln!(
                        "stream {reference}: /{} {}: {}",
                        issue.filter,
                        issue.kind_label(),
                        issue.message
                    );
                }
                eprintln!("{}", error.message);
                return Ok(ExitCode::from(1));
            }
        }
    }

    if let Some(path) = decoded {
        match read_lazy_stream_decoded_bytes(&file, reference) {
            Ok(bytes) => {
                std::fs::write(path, bytes)?;
                return Ok(ExitCode::SUCCESS);
            }
            Err(PdfDebuggerError::StreamDecode { message, .. }) => {
                eprintln!("stream {reference}: {message}");
                eprintln!(
                    "raw stream bytes remain available with `pdf-debugger dump-stream <file> {} {}`",
                    reference.object, reference.generation
                );
                return Ok(ExitCode::from(1));
            }
            Err(error) => return Err(error),
        }
    }

    std::io::stdout().write_all(&read_lazy_stream_raw_bytes(&file, reference)?)?;
    Ok(ExitCode::SUCCESS)
}

trait DecodeIssueLabel {
    fn kind_label(&self) -> &'static str;
}

impl DecodeIssueLabel for pdf_debugger::stream_decode::DecodeIssue {
    fn kind_label(&self) -> &'static str {
        match self.kind {
            pdf_debugger::stream_decode::DecodeIssueKind::Failed => "failed",
            pdf_debugger::stream_decode::DecodeIssueKind::Unsupported => "unsupported",
        }
    }
}
