use hexsift::structured_output::{BinaryMatch, HexDumpLine, OutputFormat, StructuredFormatter};

#[test]
fn test_binary_match_creation() {
    let m = BinaryMatch::new(
        "test.bin".to_string(),
        0x100,
        "48 65 6C 6C 6F".to_string(), // "Hello"
        5,
    );

    assert_eq!(m.file_path, "test.bin");
    assert_eq!(m.offset, 0x100);
    assert_eq!(m.length, 5);
    assert_eq!(m.ascii_data, Some("Hello".to_string()));
}

#[test]
fn test_hex_dump_line_creation() {
    let line = HexDumpLine::new(
        "test.bin".to_string(),
        0x0,
        "48 65 6C 6C 6F 20 57 6F 72 6C 64 21".to_string(), // "Hello World!"
        12,
    );

    assert_eq!(line.file_path, "test.bin");
    assert_eq!(line.offset, 0x0);
    assert_eq!(line.byte_count, 12);
    assert_eq!(line.ascii_data, Some("Hello World!".to_string()));
}

#[test]
fn test_output_format_parsing() {
    assert!(matches!(
        OutputFormat::from_str("hex"),
        Some(OutputFormat::Hex)
    ));
    assert!(matches!(
        OutputFormat::from_str("json"),
        Some(OutputFormat::Json)
    ));
    assert!(matches!(
        OutputFormat::from_str("csv"),
        Some(OutputFormat::Csv)
    ));
    assert!(matches!(
        OutputFormat::from_str("plain"),
        Some(OutputFormat::Plain)
    ));
    assert!(matches!(
        OutputFormat::from_str("HEX"),
        Some(OutputFormat::Hex)
    )); // Case insensitive
    assert!(matches!(OutputFormat::from_str("invalid"), None));
}

#[test]
fn test_json_output_matches() {
    let matches = vec![
        BinaryMatch::new("test.bin".to_string(), 0, "48 65 6C 6C 6F".to_string(), 5),
        BinaryMatch::new(
            "test2.bin".to_string(),
            0x100,
            "57 6F 72 6C 64".to_string(),
            5,
        ),
    ];

    let formatter = StructuredFormatter::new(OutputFormat::Json);
    let mut output = Vec::new();
    formatter.output_matches(&matches, &mut output).unwrap();

    let output_str = String::from_utf8(output).unwrap();

    // Should be valid JSON
    let parsed: serde_json::Value = serde_json::from_str(&output_str).unwrap();
    assert!(parsed.is_array());

    // Should contain our data
    assert!(output_str.contains("test.bin"));
    assert!(output_str.contains("test2.bin"));
    assert!(output_str.contains("48 65 6C 6C 6F"));
    assert!(output_str.contains("Hello"));
}

#[test]
fn test_csv_output_matches() {
    let matches = vec![
        BinaryMatch::new("test.bin".to_string(), 0, "48 65 6C 6C 6F".to_string(), 5),
        BinaryMatch::new(
            "test2.bin".to_string(),
            0x100,
            "57 6F 72 6C 64".to_string(),
            5,
        ),
    ];

    let formatter = StructuredFormatter::new(OutputFormat::Csv);
    let mut output = Vec::new();
    formatter.output_matches(&matches, &mut output).unwrap();

    let output_str = String::from_utf8(output).unwrap();

    // Should have CSV header
    assert!(output_str.contains("file_path,offset,hex_data,length,ascii_data"));

    // Should contain our data
    assert!(output_str.contains("test.bin,0,"));
    assert!(output_str.contains("test2.bin,256,"));
    assert!(output_str.contains("48 65 6C 6C 6F"));
    assert!(output_str.contains("Hello"));
}

#[test]
fn test_plain_output_matches() {
    let matches = vec![BinaryMatch::new(
        "test.bin".to_string(),
        0,
        "48 65 6C 6C 6F".to_string(),
        5,
    )];

    let formatter = StructuredFormatter::new(OutputFormat::Plain);
    let mut output = Vec::new();
    formatter.output_matches(&matches, &mut output).unwrap();

    let output_str = String::from_utf8(output).unwrap();

    // Should have plain format: filename:offset hex_data
    assert!(output_str.contains("test.bin:0 48 65 6C 6C 6F"));
}

#[test]
fn test_hex_output_matches() {
    let matches = vec![BinaryMatch::new(
        "test.bin".to_string(),
        0x100,
        "48 65 6C 6C 6F".to_string(),
        5,
    )];

    let formatter = StructuredFormatter::new(OutputFormat::Hex);
    let mut output = Vec::new();
    formatter.output_matches(&matches, &mut output).unwrap();

    let output_str = String::from_utf8(output).unwrap();

    // Should have hex format with offset
    assert!(output_str.contains("100h : 48 65 6C 6C 6F"));
}

#[test]
fn test_json_output_hex_dump() {
    let lines = vec![HexDumpLine::new(
        "test.bin".to_string(),
        0,
        "48 65 6C 6C 6F 20 57 6F 72 6C 64 21".to_string(),
        12,
    )];

    let formatter = StructuredFormatter::new(OutputFormat::Json);
    let mut output = Vec::new();
    formatter.output_hex_dump(&lines, &mut output).unwrap();

    let output_str = String::from_utf8(output).unwrap();

    // Should be valid JSON
    let parsed: serde_json::Value = serde_json::from_str(&output_str).unwrap();
    assert!(parsed.is_array());

    // Should contain our data
    assert!(output_str.contains("test.bin"));
    assert!(output_str.contains("48 65 6C 6C 6F 20 57 6F 72 6C 64 21"));
    assert!(output_str.contains("Hello World!"));
}

#[test]
fn test_csv_output_hex_dump() {
    let lines = vec![
        HexDumpLine::new("test.bin".to_string(), 0, "48 65 6C 6C 6F".to_string(), 5),
        HexDumpLine::new(
            "test.bin".to_string(),
            5,
            "20 57 6F 72 6C 64".to_string(),
            6,
        ),
    ];

    let formatter = StructuredFormatter::new(OutputFormat::Csv);
    let mut output = Vec::new();
    formatter.output_hex_dump(&lines, &mut output).unwrap();

    let output_str = String::from_utf8(output).unwrap();

    // Should have CSV header
    assert!(output_str.contains("file_path,offset,hex_data,byte_count,ascii_data"));

    // Should contain our data
    assert!(output_str.contains("test.bin,0,"));
    assert!(output_str.contains("test.bin,5,"));
    assert!(output_str.contains("48 65 6C 6C 6F"));
    assert!(output_str.contains("20 57 6F 72 6C 64"));
}

#[test]
fn test_ascii_conversion() {
    // Printable ASCII should be converted
    let printable = BinaryMatch::new("test.bin".to_string(), 0, "48 65 6C 6C 6F".to_string(), 5);
    assert_eq!(printable.ascii_data, Some("Hello".to_string()));

    // Non-printable bytes should not be converted
    let non_printable = BinaryMatch::new("test.bin".to_string(), 0, "00 01 02 FF".to_string(), 4);
    assert_eq!(non_printable.ascii_data, None);

    // Mixed printable/non-printable should not be converted
    let mixed = BinaryMatch::new("test.bin".to_string(), 0, "48 65 00 6C 6F".to_string(), 5);
    assert_eq!(mixed.ascii_data, None);

    // Space character should be allowed
    let with_space = BinaryMatch::new("test.bin".to_string(), 0, "48 65 20 6C 6F".to_string(), 5);
    assert_eq!(with_space.ascii_data, Some("He lo".to_string()));
}
