use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

fn get_binary_path() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("target");
    path.push("debug");
    path.push("hexsift");
    path
}

fn create_test_file(content: &[u8]) -> PathBuf {
    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir.join(format!("bingrep_test_{}.bin", uuid::Uuid::new_v4()));
    let mut file = File::create(&file_path).unwrap();
    file.write_all(content).unwrap();
    file_path
}

#[test]
fn test_basic_hex_display() {
    let binary_path = get_binary_path();
    let test_data = b"Hello World!";
    let test_file = create_test_file(test_data);

    let output = Command::new(&binary_path)
        .arg(&test_file)
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // 기본 16진수 출력 확인
    assert!(stdout.contains("48 65 6C 6C 6F 20 57 6F 72 6C 64 21"));
    assert!(stdout.contains("h :")); // 오프셋 표시 확인

    // 정리
    fs::remove_file(test_file).ok();
}

#[test]
fn test_hex_display_with_width() {
    let binary_path = get_binary_path();
    let test_data = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let test_file = create_test_file(test_data);

    let output = Command::new(&binary_path)
        .arg(&test_file)
        .arg("-w")
        .arg("8")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();

    // width=8로 설정했으므로 각 줄에 8바이트씩 출력
    assert!(lines[0].contains("41 42 43 44 45 46 47 48")); // ABCDEFGH
    assert!(lines[1].contains("49 4A 4B 4C 4D 4E 4F 50")); // IJKLMNOP

    // 정리
    fs::remove_file(test_file).ok();
}

#[test]
fn test_hex_display_with_limit() {
    let binary_path = get_binary_path();
    let test_data = b"Line1\nLine2\nLine3\nLine4\nLine5\n";
    let test_file = create_test_file(test_data);

    let output = Command::new(&binary_path)
        .arg(&test_file)
        .arg("-n")
        .arg("2")
        .arg("-w")
        .arg("6")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();

    // limit=2로 설정했으므로 2줄만 출력
    assert_eq!(lines.len(), 2);

    // 정리
    fs::remove_file(test_file).ok();
}

#[test]
fn test_no_offset() {
    let binary_path = get_binary_path();
    let test_data = b"Test";
    let test_file = create_test_file(test_data);

    let output = Command::new(&binary_path)
        .arg(&test_file)
        .arg("--no-offset")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // 오프셋이 숨겨졌는지 확인
    assert!(!stdout.contains("h :"));
    assert!(stdout.contains("54 65 73 74")); // "Test"의 hex

    // 정리
    fs::remove_file(test_file).ok();
}

#[test]
fn test_custom_separator() {
    let binary_path = get_binary_path();
    let test_data = b"ABC";
    let test_file = create_test_file(test_data);

    let output = Command::new(&binary_path)
        .arg(&test_file)
        .arg("-t")
        .arg("-")
        .arg("--no-offset")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // 커스텀 구분자 확인
    assert!(stdout.contains("41-42-43"));

    // 정리
    fs::remove_file(test_file).ok();
}

#[test]
fn test_regex_search_basic() {
    let binary_path = get_binary_path();
    let test_data = b"Header\x00\x00\x00\x01\x67Footer";
    let test_file = create_test_file(test_data);

    let output = Command::new(&binary_path)
        .arg(&test_file)
        .arg("-e")
        .arg("\\x00\\x00\\x00\\x01\\x67")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // 패턴이 발견되었는지 확인
    assert!(stdout.contains("00 00 00 01 67"));

    // 오프셋이 올바른지 확인 (Header는 6바이트)
    assert!(stdout.contains("06h") || stdout.contains("6h"));

    // 정리
    fs::remove_file(test_file).ok();
}

#[test]
fn test_regex_search_multiple_matches() {
    let binary_path = get_binary_path();
    let test_data = b"First\x00\x00\x00\x01\x67Middle\x00\x00\x00\x01\x67Last";
    let test_file = create_test_file(test_data);

    let output = Command::new(&binary_path)
        .arg(&test_file)
        .arg("-e")
        .arg("\\x00\\x00\\x00\\x01\\x67")
        .arg("-w")
        .arg("8")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();

    // 두 개의 매치가 발견되었는지 확인
    assert_eq!(lines.len(), 2);
    assert!(stdout.contains("00 00 00 01 67"));

    // 정리
    fs::remove_file(test_file).ok();
}

#[test]
fn test_regex_search_with_limit() {
    let binary_path = get_binary_path();
    // 패턴이 3번 반복되는 데이터
    let test_data = b"Pat1\x00\x01Pat2\x00\x01Pat3\x00\x01";
    let test_file = create_test_file(test_data);

    let output = Command::new(&binary_path)
        .arg(&test_file)
        .arg("-e")
        .arg("\\x00\\x01")
        .arg("-n")
        .arg("2")
        .arg("-w")
        .arg("4")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();

    // limit=2로 설정했으므로 2개만 출력
    assert_eq!(lines.len(), 2);

    // 정리
    fs::remove_file(test_file).ok();
}

#[test]
fn test_start_position() {
    let binary_path = get_binary_path();
    let test_data = b"SkipThis\x00FindThis";
    let test_file = create_test_file(test_data);

    let output = Command::new(&binary_path)
        .arg(&test_file)
        .arg("-s")
        .arg("8") // "SkipThis"를 건너뛰기
        .arg("-w")
        .arg("10")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // 시작 위치가 8이므로 "\x00FindThis"부터 출력
    assert!(stdout.contains("00 46 69 6E 64 54 68 69 73")); // "\x00FindThis"

    // 정리
    fs::remove_file(test_file).ok();
}

#[test]
fn test_large_file_handling() {
    let binary_path = get_binary_path();

    // 5KB 크기의 파일 생성
    let mut test_data = Vec::new();
    for i in 0..5120 {
        test_data.push((i % 256) as u8);
    }
    // 중간에 패턴 삽입
    test_data[2560..2565].copy_from_slice(b"\x00\x00\x00\x01\x67");

    let test_file = create_test_file(&test_data);

    let output = Command::new(&binary_path)
        .arg(&test_file)
        .arg("-e")
        .arg("\\x00\\x00\\x00\\x01\\x67")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // 패턴이 발견되었는지 확인
    assert!(stdout.contains("00 00 00 01 67"));

    // 정리
    fs::remove_file(test_file).ok();
}

#[test]
fn test_empty_file() {
    let binary_path = get_binary_path();
    let test_file = create_test_file(b"");

    let output = Command::new(&binary_path)
        .arg(&test_file)
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // 빈 파일은 출력이 없어야 함
    assert_eq!(stdout.trim(), "");

    // 정리
    fs::remove_file(test_file).ok();
}

#[test]
fn test_binary_file_with_nulls() {
    let binary_path = get_binary_path();
    let test_data = b"\x00\x00\x00\x00\xFF\xFF\xFF\xFF\x00\x00\x00\x00";
    let test_file = create_test_file(test_data);

    let output = Command::new(&binary_path)
        .arg(&test_file)
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // NULL 바이트와 0xFF 바이트가 올바르게 표시되는지 확인
    assert!(stdout.contains("00 00 00 00 FF FF FF FF 00 00 00 00"));

    // 정리
    fs::remove_file(test_file).ok();
}

#[test]
fn test_help_output() {
    let binary_path = get_binary_path();

    let output = Command::new(&binary_path)
        .arg("--help")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // 도움말에 필요한 정보가 포함되어 있는지 확인
    assert!(stdout.contains("바이너리 파일을 정규표현식으로 검색하는 도구"));
    assert!(stdout.contains("-e, --regex"));
    assert!(stdout.contains("-w, --width"));
    assert!(stdout.contains("-n, --line"));
    assert!(stdout.contains("--no-offset"));
}

#[test]
fn test_version_output() {
    let binary_path = get_binary_path();

    let output = Command::new(&binary_path)
        .arg("--version")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // 버전 정보가 출력되는지 확인
    assert!(stdout.contains("hexsift"));
    assert!(stdout.contains("0.1.0"));
}
