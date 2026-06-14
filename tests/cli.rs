mod fixtures;

use fixtures::FixtureSet;
use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_pdf-debugger")
}

#[test]
fn inspect_outputs_metadata_for_valid_pdf() {
    let fixtures = FixtureSet::create("inspect");
    let output = run(["inspect", path(&fixtures.minimal)]);

    assert_success(&output);
    let json = stdout_json(&output);
    assert_eq!(json["pdf_version"], "1.4");
    assert_eq!(json["page_count"], 1);
    assert_eq!(json["object_count"], 5);
}

#[test]
fn inspect_structure_outputs_required_nodes() {
    let fixtures = FixtureSet::create("structure");
    let output = run(["inspect", path(&fixtures.minimal), "--structure"]);

    assert_success(&output);
    let stdout = stdout(&output);
    for label in [
        "Trailer",
        "Catalog",
        "Pages",
        "Page",
        "Resources",
        "Fonts",
        "Cross-reference",
    ] {
        assert!(
            stdout.contains(label),
            "structure output should contain {label}"
        );
    }
}

#[test]
fn lazy_inspect_outputs_metadata_structure_and_object() {
    let fixtures = FixtureSet::create("lazy-inspect");
    let metadata_output = run(["lazy-inspect", path(&fixtures.minimal)]);

    assert_success(&metadata_output);
    let metadata = stdout_json(&metadata_output);
    assert_eq!(metadata["pdf_version"], "1.4");
    assert_eq!(metadata["page_count"], 1);
    assert_eq!(metadata["object_count"], 5);

    let structure_output = run(["lazy-inspect", path(&fixtures.minimal), "--structure"]);
    assert_success(&structure_output);
    assert!(stdout(&structure_output).contains("lazy open"));

    let pages_output = run(["lazy-inspect", path(&fixtures.minimal), "--pages"]);
    assert_success(&pages_output);
    let pages = stdout_json(&pages_output);
    assert_eq!(pages["pages"].as_array().unwrap().len(), 1);
    assert_eq!(pages["pages"][0]["reference"]["object"], 3);

    let diagnostics_output = run(["lazy-inspect", path(&fixtures.minimal), "--diagnostics"]);
    assert_success(&diagnostics_output);
    let diagnostics = stdout_json(&diagnostics_output);
    assert!(diagnostics["findings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|finding| finding["rule_id"] == "lazy.deep_diagnostics_deferred"));
    assert_eq!(diagnostics["diagnostics"]["info"], 1);

    let object_output = run([
        "lazy-inspect",
        path(&fixtures.minimal),
        "--object",
        "1",
        "0",
    ]);
    assert_success(&object_output);
    let object = stdout_json(&object_output);
    assert_eq!(object["reference"]["object"], 1);
    assert_eq!(object["object_type"], "dictionary");

    let deep_stream_output = run([
        "lazy-inspect",
        path(&fixtures.minimal),
        "--deep-stream",
        "4",
        "0",
    ]);
    assert_success(&deep_stream_output);
    let deep_stream = stdout_json(&deep_stream_output);
    assert_eq!(deep_stream["scope"], "stream");
    assert_eq!(deep_stream["reference"]["object"], 4);
    assert!(deep_stream["details"]
        .as_array()
        .unwrap()
        .iter()
        .any(|detail| detail["name"] == "Content operators"));
}

#[test]
fn lazy_inspect_supports_xref_stream_and_object_stream_members() {
    let fixtures = FixtureSet::create("lazy-xref-object-streams");

    let metadata_output = run(["lazy-inspect", path(&fixtures.xref_stream)]);
    assert_success(&metadata_output);
    let metadata = stdout_json(&metadata_output);
    assert_eq!(metadata["pdf_version"], "1.5");
    assert_eq!(metadata["has_xref_stream"], true);
    assert_eq!(metadata["page_count"], 1);

    let page_output = run([
        "lazy-inspect",
        path(&fixtures.xref_stream),
        "--object",
        "3",
        "0",
    ]);
    assert_success(&page_output);
    let page = stdout_json(&page_output);
    assert_eq!(page["reference"]["object"], 3);
    assert_eq!(page["object_type"], "dictionary");

    let object_stream_metadata = run(["lazy-inspect", path(&fixtures.object_stream)]);
    assert_success(&object_stream_metadata);
    let object_stream_metadata = stdout_json(&object_stream_metadata);
    assert_eq!(object_stream_metadata["has_object_stream"], true);

    let compressed_object = run([
        "lazy-inspect",
        path(&fixtures.object_stream),
        "--object",
        "5",
        "0",
    ]);
    assert_success(&compressed_object);
    let compressed_object = stdout_json(&compressed_object);
    assert_eq!(compressed_object["reference"]["object"], 5);
    assert_eq!(compressed_object["object_type"], "dictionary");
    assert!(compressed_object["dictionary_keys"]
        .as_array()
        .unwrap()
        .iter()
        .any(|key| key == "BaseFont"));
}

#[test]
fn lazy_inspect_supports_incremental_prev_and_hybrid_xref() {
    let fixtures = FixtureSet::create("lazy-prev-hybrid");

    let previous_object = run([
        "lazy-inspect",
        path(&fixtures.incremental_prev),
        "--object",
        "3",
        "0",
    ]);
    assert_success(&previous_object);
    let previous_object = stdout_json(&previous_object);
    assert_eq!(previous_object["reference"]["object"], 3);
    assert_eq!(previous_object["object_type"], "dictionary");

    let updated_object = run([
        "lazy-inspect",
        path(&fixtures.incremental_prev),
        "--object",
        "5",
        "0",
    ]);
    assert_success(&updated_object);
    let updated_object = stdout_json(&updated_object);
    assert!(updated_object["dictionary_keys"]
        .as_array()
        .unwrap()
        .iter()
        .any(|key| key == "Updated"));

    let new_object = run([
        "lazy-inspect",
        path(&fixtures.incremental_prev),
        "--object",
        "6",
        "0",
    ]);
    assert_success(&new_object);
    let new_object = stdout_json(&new_object);
    assert!(new_object["dictionary_keys"]
        .as_array()
        .unwrap()
        .iter()
        .any(|key| key == "Added"));

    let hybrid_metadata = run(["lazy-inspect", path(&fixtures.hybrid_xref)]);
    assert_success(&hybrid_metadata);
    let hybrid_metadata = stdout_json(&hybrid_metadata);
    assert_eq!(hybrid_metadata["has_xref_stream"], true);
    assert_eq!(hybrid_metadata["has_object_stream"], true);

    let hybrid_object = run([
        "lazy-inspect",
        path(&fixtures.hybrid_xref),
        "--object",
        "5",
        "0",
    ]);
    assert_success(&hybrid_object);
    let hybrid_object = stdout_json(&hybrid_object);
    assert_eq!(hybrid_object["object_type"], "dictionary");
    assert!(hybrid_object["dictionary_keys"]
        .as_array()
        .unwrap()
        .iter()
        .any(|key| key == "BaseFont"));
}

#[test]
fn check_reports_clean_pdf_with_zero_exit() {
    let fixtures = FixtureSet::create("clean-check");
    let report = fixtures.dir.join("report.json");
    let markdown = fixtures.dir.join("report.md");
    let output = run([
        "check",
        path(&fixtures.minimal),
        "--json",
        path(&report),
        "--markdown",
        path(&markdown),
    ]);

    assert_success(&output);
    let json: Value =
        serde_json::from_str(&std::fs::read_to_string(report).expect("read JSON report"))
            .expect("parse JSON report");
    assert_eq!(json["diagnostics"]["errors"], 0);
    assert_eq!(json["findings"].as_array().unwrap().len(), 0);
    let markdown = std::fs::read_to_string(markdown).expect("read Markdown report");
    assert!(markdown.contains("PDF Debug Report"));
}

#[test]
fn check_falls_back_to_lazy_report_for_sparse_large_pdf() {
    let fixtures = FixtureSet::create("lazy-check-report");
    let report = fixtures.dir.join("lazy-report.json");
    let markdown = fixtures.dir.join("lazy-report.md");
    let output = run([
        "check",
        path(&fixtures.sparse_large),
        "--json",
        path(&report),
        "--markdown",
        path(&markdown),
    ]);

    assert_success(&output);
    let json: Value =
        serde_json::from_str(&std::fs::read_to_string(report).expect("read lazy JSON report"))
            .expect("parse lazy JSON report");
    assert_eq!(json["open_mode"], "lazy");
    assert_eq!(json["report_kind"], "lazy_fast_diagnostics");
    assert_eq!(json["diagnostics"]["errors"], 0);
    assert!(json["findings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|finding| finding["rule_id"] == "lazy.deep_diagnostics_deferred"));
    assert!(json["deferred_deep_diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item == "stream decode validation"));

    let markdown = std::fs::read_to_string(markdown).expect("read lazy Markdown report");
    assert!(markdown.contains("PDF Lazy Debug Report"));
    assert!(markdown.contains("Fast diagnostics only"));
    assert!(markdown.contains("Deep diagnostics deferred"));
    assert!(!markdown.contains("BT /F1 12 Tf"));
}

#[test]
fn check_can_enrich_lazy_report_for_sparse_large_pdf() {
    let fixtures = FixtureSet::create("lazy-check-enriched-report");
    let report = fixtures.dir.join("lazy-enriched-report.json");
    let markdown = fixtures.dir.join("lazy-enriched-report.md");
    let output = run([
        "check",
        path(&fixtures.sparse_large),
        "--json",
        path(&report),
        "--markdown",
        path(&markdown),
        "--lazy-stream",
        "4",
        "0",
        "--lazy-object",
        "1",
        "0",
        "--lazy-page",
        "1",
    ]);

    assert_success(&output);
    let json: Value = serde_json::from_str(
        &std::fs::read_to_string(report).expect("read enriched lazy JSON report"),
    )
    .expect("parse enriched lazy JSON report");
    let enrichments = json["enrichments"]
        .as_array()
        .expect("lazy report enrichments");
    assert_eq!(enrichments.len(), 3);
    assert!(enrichments
        .iter()
        .any(|enrichment| enrichment["scope"] == "stream"));
    assert!(enrichments
        .iter()
        .any(|enrichment| enrichment["scope"] == "object"));
    assert!(enrichments
        .iter()
        .any(|enrichment| enrichment["scope"] == "page"));

    let markdown = std::fs::read_to_string(markdown).expect("read enriched lazy Markdown report");
    assert!(markdown.contains("Deep Diagnostics Enrichment"));
    assert!(markdown.contains("Stream 4 0 R"));
    assert!(markdown.contains("Object 1 0 R"));
    assert!(markdown.contains("Page 1"));
    assert!(!markdown.contains("BT /F1 12 Tf"));
}

#[test]
fn check_returns_one_for_missing_reference() {
    let fixtures = FixtureSet::create("missing-reference");
    let report = fixtures.dir.join("missing.json");
    let output = run([
        "check",
        path(&fixtures.missing_reference),
        "--json",
        path(&report),
    ]);

    assert_eq!(output.status.code(), Some(1), "stderr: {}", stderr(&output));
    let json: Value =
        serde_json::from_str(&std::fs::read_to_string(report).expect("read JSON report"))
            .expect("parse JSON report");
    assert_rule(&json, "object.missing_reference");
}

#[test]
fn check_reports_decode_failure() {
    let fixtures = FixtureSet::create("decode-failure");
    let report = fixtures.dir.join("decode-failure.json");
    let output = run([
        "check",
        path(&fixtures.decode_failure),
        "--json",
        path(&report),
    ]);

    assert_eq!(output.status.code(), Some(1), "stderr: {}", stderr(&output));
    let json: Value =
        serde_json::from_str(&std::fs::read_to_string(report).expect("read JSON report"))
            .expect("parse JSON report");
    assert_rule(&json, "stream.decode_failed");
}

#[test]
fn check_reports_unknown_content_operator() {
    let fixtures = FixtureSet::create("unknown-operator");
    let report = fixtures.dir.join("unknown.json");
    let output = run([
        "check",
        path(&fixtures.unknown_operator),
        "--json",
        path(&report),
    ]);

    assert_success(&output);
    let json: Value =
        serde_json::from_str(&std::fs::read_to_string(report).expect("read JSON report"))
            .expect("parse JSON report");
    assert_rule(&json, "content.unknown_operator");
}

#[test]
fn check_reports_invalid_xref_offset() {
    let fixtures = FixtureSet::create("invalid-xref");
    let report = fixtures.dir.join("xref.json");
    let output = run([
        "check",
        path(&fixtures.invalid_xref_offset),
        "--json",
        path(&report),
    ]);

    assert_eq!(output.status.code(), Some(1), "stderr: {}", stderr(&output));
    let json: Value =
        serde_json::from_str(&std::fs::read_to_string(report).expect("read JSON report"))
            .expect("parse JSON report");
    assert_rule(&json, "xref.invalid_offset");
}

#[test]
fn dump_object_json_inspects_object() {
    let fixtures = FixtureSet::create("dump-object-json");
    let output = run(["dump-object", path(&fixtures.minimal), "4", "0", "--json"]);

    assert_success(&output);
    let json = stdout_json(&output);
    assert_eq!(json["reference"]["object"], 4);
    assert_eq!(json["object_type"], "stream");
    assert_eq!(json["stream"]["decoded_length"], 23);

    let catalog_output = run(["dump-object", path(&fixtures.minimal), "1", "0", "--json"]);
    assert_success(&catalog_output);
    let catalog = stdout_json(&catalog_output);
    assert_eq!(catalog["references"][0]["object"], 2);
    assert_eq!(catalog["references"][0]["generation"], 0);
}

#[test]
fn dump_stream_modes_work() {
    let fixtures = FixtureSet::create("dump-stream");

    let json_output = run(["dump-stream", path(&fixtures.minimal), "4", "0", "--json"]);
    assert_success(&json_output);
    let json = stdout_json(&json_output);
    assert_eq!(json["decoded_length"], 23);

    let hex_output = run(["dump-stream", path(&fixtures.minimal), "4", "0", "--hex"]);
    assert_success(&hex_output);
    assert!(stdout(&hex_output).contains("42 54 20 2f"));

    let content_output = run([
        "dump-stream",
        path(&fixtures.unknown_operator),
        "4",
        "0",
        "--content-json",
    ]);
    assert_success(&content_output);
    let content = stdout_json(&content_output);
    assert_eq!(
        content["analysis"]["warnings"][0]["rule_id"],
        "content.unknown_operator"
    );

    let decoded = fixtures.dir.join("decoded.txt");
    let decoded_output = run([
        "dump-stream",
        path(&fixtures.minimal),
        "4",
        "0",
        "--decoded",
        path(&decoded),
    ]);
    assert_success(&decoded_output);
    assert_eq!(
        std::fs::read_to_string(decoded).expect("read decoded stream"),
        "BT /F1 12 Tf (Hi) Tj ET"
    );
}

#[test]
fn lazy_stream_modes_work_for_sparse_large_pdf() {
    let fixtures = FixtureSet::create("lazy-stream");

    let lazy_output = run([
        "lazy-inspect",
        path(&fixtures.sparse_large),
        "--stream",
        "4",
        "0",
    ]);
    assert_success(&lazy_output);
    let lazy = stdout_json(&lazy_output);
    assert_eq!(lazy["reference"]["object"], 4);
    assert_eq!(lazy["actual_length"], 23);
    assert_eq!(lazy["decoded_length"], 23);
    assert_eq!(lazy["can_export_raw"], true);
    assert_eq!(lazy["can_export_decoded"], true);

    let json_output = run([
        "dump-stream",
        path(&fixtures.sparse_large),
        "4",
        "0",
        "--json",
    ]);
    assert_success(&json_output);
    let json = stdout_json(&json_output);
    assert_eq!(json["decoded_length"], 23);
    assert!(json["raw_text"].as_str().unwrap().contains("BT /F1"));

    let hex_output = run([
        "dump-stream",
        path(&fixtures.sparse_large),
        "4",
        "0",
        "--hex",
    ]);
    assert_success(&hex_output);
    assert!(stdout(&hex_output).contains("42 54 20 2f"));

    let decoded = fixtures.dir.join("lazy-decoded.txt");
    let decoded_output = run([
        "dump-stream",
        path(&fixtures.sparse_large),
        "4",
        "0",
        "--decoded",
        path(&decoded),
    ]);
    assert_success(&decoded_output);
    assert_eq!(
        std::fs::read_to_string(decoded).expect("read lazy decoded stream"),
        "BT /F1 12 Tf (Hi) Tj ET"
    );

    let raw_output = run(["dump-stream", path(&fixtures.sparse_large), "4", "0"]);
    assert_success(&raw_output);
    assert_eq!(stdout(&raw_output), "BT /F1 12 Tf (Hi) Tj ET");
}

fn run<const N: usize>(args: [&str; N]) -> Output {
    Command::new(bin())
        .args(args)
        .output()
        .expect("run pdf-debugger")
}

fn path(path: &Path) -> &str {
    path.to_str().expect("fixture path is valid UTF-8")
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "expected success, got {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        stdout(output),
        stderr(output)
    );
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn stdout_json(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).expect("parse stdout JSON")
}

fn assert_rule(report: &Value, rule_id: &str) {
    let findings = report["findings"]
        .as_array()
        .expect("report findings should be an array");
    assert!(
        findings.iter().any(|finding| finding["rule_id"] == rule_id),
        "expected finding {rule_id}, got {findings:#?}"
    );
}
