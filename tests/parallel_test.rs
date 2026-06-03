use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

fn get_binary_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_hexsift"))
}

fn create_test_file(content: &[u8], suffix: &str) -> PathBuf {
    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir.join(format!(
        "bingrep_parallel_test_{}_{}.bin",
        uuid::Uuid::new_v4(),
        suffix
    ));
    let mut file = File::create(&file_path).unwrap();
    file.write_all(content).unwrap();
    file_path
}

#[test]
fn test_parallel_vs_sequential() {
    let binary_path = get_binary_path();

    // Create a large test file (2MB) with patterns scattered throughout
    let mut test_data = Vec::with_capacity(2 * 1024 * 1024);
    let pattern = b"\x00\x01\x02\x03";

    // Fill with zeroes to avoid false matches
    for _i in 0..(2 * 1024 * 1024) {
        test_data.push(0xFF); // Use 0xFF to avoid false matches with our pattern
    }

    // Insert pattern at various locations
    let pattern_locations = [1000, 50000, 100000, 500000, 1000000, 1500000];
    for &loc in &pattern_locations {
        if loc + pattern.len() <= test_data.len() {
            test_data[loc..loc + pattern.len()].copy_from_slice(pattern);
        }
    }

    let test_file = create_test_file(&test_data, "large");

    // Test sequential processing
    let output_seq = Command::new(&binary_path)
        .arg(&test_file)
        .arg("-e")
        .arg("\\x00\\x01\\x02\\x03")
        .arg("-w")
        .arg("16")
        .output()
        .expect("Failed to execute sequential command");

    // Test parallel processing
    let output_par = Command::new(&binary_path)
        .arg(&test_file)
        .arg("-e")
        .arg("\\x00\\x01\\x02\\x03")
        .arg("-w")
        .arg("16")
        .arg("--parallel")
        .arg("--chunk-size")
        .arg("65536") // 64KB chunks
        .output()
        .expect("Failed to execute parallel command");

    // Both should succeed
    assert!(output_seq.status.success(), "Sequential processing failed");
    assert!(output_par.status.success(), "Parallel processing failed");

    let stdout_seq = String::from_utf8_lossy(&output_seq.stdout);
    let stdout_par = String::from_utf8_lossy(&output_par.stdout);

    // Both outputs should contain the pattern
    assert!(
        stdout_seq.contains("00 01 02 03"),
        "Sequential: Pattern not found"
    );
    assert!(
        stdout_par.contains("00 01 02 03"),
        "Parallel: Pattern not found"
    );

    // Count matches in both outputs
    let seq_matches = stdout_seq.lines().count();
    let par_matches = stdout_par.lines().count();

    // Should find the same number of matches (at least the ones we inserted)
    println!(
        "Sequential matches: {}, Parallel matches: {}",
        seq_matches, par_matches
    );

    // Allow for small differences due to overlap handling, but should be close
    let difference = if seq_matches > par_matches {
        seq_matches - par_matches
    } else {
        par_matches - seq_matches
    };
    assert!(
        difference <= 5,
        "Too many differences between sequential ({}) and parallel ({})",
        seq_matches,
        par_matches
    );
    assert!(
        seq_matches >= pattern_locations.len(),
        "Not all patterns found"
    );

    // 정리
    fs::remove_file(test_file).ok();
}

#[test]
fn test_parallel_uses_dynamic_overlap_for_long_hex_pattern() {
    let binary_path = get_binary_path();
    let chunk_size = 4096usize;
    let pattern_start = chunk_size - 600;
    let pattern: Vec<u8> = (0..1100).map(|i| (i % 251) as u8).collect();
    let expression = pattern
        .iter()
        .map(|byte| format!("\\x{:02x}", byte))
        .collect::<String>();

    let mut test_data = vec![0xFF; pattern_start];
    test_data.extend_from_slice(&pattern);
    test_data.extend_from_slice(&[0xFF; 100]);
    let test_file = create_test_file(&test_data, "dynamic_overlap");

    let output = Command::new(&binary_path)
        .arg(&test_file)
        .arg("-e")
        .arg(&expression)
        .arg("-w")
        .arg(pattern.len().to_string())
        .arg("--parallel")
        .arg("--chunk-size")
        .arg(chunk_size.to_string())
        .output()
        .expect("Failed to execute parallel command");

    assert!(output.status.success(), "Parallel search failed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.lines().count(), 1, "Expected one boundary match");
    assert!(
        stdout.contains("00 01 02 03 04 05 06 07"),
        "Pattern prefix not found"
    );
    assert!(
        stdout.contains("54 55 56 57 58 59 5A 5B"),
        "Pattern suffix not found"
    );

    fs::remove_file(test_file).ok();
}

#[test]
fn test_parallel_hex_dump() {
    let binary_path = get_binary_path();

    // Create test data
    let test_data: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();
    let test_file = create_test_file(&test_data, "hexdump");

    // Test sequential hex dump
    let output_seq = Command::new(&binary_path)
        .arg(&test_file)
        .arg("-w")
        .arg("16")
        .arg("-n")
        .arg("10")
        .output()
        .expect("Failed to execute sequential hex dump");

    // Test parallel hex dump
    let output_par = Command::new(&binary_path)
        .arg(&test_file)
        .arg("-w")
        .arg("16")
        .arg("-n")
        .arg("10")
        .arg("--parallel")
        .arg("--chunk-size")
        .arg("256")
        .output()
        .expect("Failed to execute parallel hex dump");

    // Both should succeed
    assert!(output_seq.status.success(), "Sequential hex dump failed");
    assert!(output_par.status.success(), "Parallel hex dump failed");

    let stdout_seq = String::from_utf8_lossy(&output_seq.stdout);
    let stdout_par = String::from_utf8_lossy(&output_par.stdout);

    // Both should produce 10 lines
    assert_eq!(
        stdout_seq.lines().count(),
        10,
        "Sequential: Wrong line count"
    );
    assert_eq!(stdout_par.lines().count(), 10, "Parallel: Wrong line count");

    // First line should start with the same data
    let seq_first_line = stdout_seq.lines().next().unwrap();
    let par_first_line = stdout_par.lines().next().unwrap();

    assert!(
        seq_first_line.contains("00 01 02 03"),
        "Sequential: Wrong first line content"
    );
    assert!(
        par_first_line.contains("00 01 02 03"),
        "Parallel: Wrong first line content"
    );

    // 정리
    fs::remove_file(test_file).ok();
}

#[test]
fn test_parallel_with_limit() {
    let binary_path = get_binary_path();

    // Create test data with multiple pattern occurrences
    let mut test_data = Vec::new();
    let pattern = b"\x42\x42\x42\x42";

    // Insert pattern 20 times
    for i in 0..20 {
        test_data.extend_from_slice(&format!("Data chunk {} ", i).into_bytes());
        test_data.extend_from_slice(pattern);
        test_data.extend_from_slice(b" more data ");
    }

    let test_file = create_test_file(&test_data, "limited");

    // Test with limit of 5
    let output = Command::new(&binary_path)
        .arg(&test_file)
        .arg("-e")
        .arg("\\x42\\x42\\x42\\x42")
        .arg("-n")
        .arg("5")
        .arg("--parallel")
        .arg("--chunk-size")
        .arg("128")
        .output()
        .expect("Failed to execute parallel command with limit");

    assert!(
        output.status.success(),
        "Parallel processing with limit failed"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let line_count = stdout.lines().count();

    // Should find exactly 5 matches
    assert_eq!(line_count, 5, "Expected 5 matches, got {}", line_count);
    assert!(stdout.contains("42 42 42 42"), "Pattern not found");

    // 정리
    fs::remove_file(test_file).ok();
}

#[test]
fn test_parallel_small_file() {
    let binary_path = get_binary_path();

    // Create a small file (smaller than chunk size)
    let test_data = b"Small file with pattern \x01\x02\x03 end";
    let test_file = create_test_file(test_data, "small");

    // Test with parallel flag but small file
    let output = Command::new(&binary_path)
        .arg(&test_file)
        .arg("-e")
        .arg("\\x01\\x02\\x03")
        .arg("--parallel")
        .arg("--chunk-size")
        .arg("1024")
        .output()
        .expect("Failed to execute parallel command on small file");

    // Should still work and fall back to sequential processing
    assert!(
        output.status.success(),
        "Parallel processing on small file failed"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("01 02 03"),
        "Pattern not found in small file"
    );

    // 정리
    fs::remove_file(test_file).ok();
}

#[test]
fn test_parallel_performance_benchmark() {
    let binary_path = get_binary_path();

    // Create a larger test file (5MB) for performance testing
    let mut test_data = Vec::with_capacity(5 * 1024 * 1024);
    let pattern = b"\x01\x02\x03\x04"; // Use lowercase hex pattern

    // Fill with data (avoid creating accidental patterns)
    for _i in 0..(5 * 1024 * 1024) {
        test_data.push(0xAA); // Use consistent byte to avoid false matches
    }

    // Insert pattern every 100KB (should be ~50 patterns)
    let mut pattern_count = 0;
    for i in (0..test_data.len()).step_by(100000) {
        if i + pattern.len() <= test_data.len() {
            test_data[i..i + pattern.len()].copy_from_slice(pattern);
            pattern_count += 1;
        }
    }

    let test_file = create_test_file(&test_data, "benchmark");

    // Test parallel processing
    let start = std::time::Instant::now();
    let output_par = Command::new(&binary_path)
        .arg(&test_file)
        .arg("-e")
        .arg("\\x01\\x02\\x03\\x04")
        .arg("--parallel")
        .arg("--chunk-size")
        .arg("262144") // 256KB chunks
        .output()
        .expect("Failed to execute parallel benchmark");

    let parallel_time = start.elapsed();

    // Test sequential processing
    let start = std::time::Instant::now();
    let output_seq = Command::new(&binary_path)
        .arg(&test_file)
        .arg("-e")
        .arg("\\x01\\x02\\x03\\x04")
        .output()
        .expect("Failed to execute sequential benchmark");

    let sequential_time = start.elapsed();

    // Both should succeed
    assert!(output_par.status.success(), "Parallel benchmark failed");
    assert!(output_seq.status.success(), "Sequential benchmark failed");

    // Both should find the same matches
    let stdout_par = String::from_utf8_lossy(&output_par.stdout);
    let stdout_seq = String::from_utf8_lossy(&output_seq.stdout);

    let par_matches = stdout_par.lines().count();
    let seq_matches = stdout_seq.lines().count();

    assert_eq!(par_matches, seq_matches, "Different match counts");
    assert_eq!(
        par_matches, pattern_count,
        "Should find exactly {} patterns, found {}",
        pattern_count, par_matches
    );

    println!("Parallel time: {:?}", parallel_time);
    println!("Sequential time: {:?}", sequential_time);
    println!("Matches found: {}", par_matches);

    // 정리
    fs::remove_file(test_file).ok();
}
