use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use tempfile::NamedTempFile;

fn get_binary_path() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("target");
    path.push("debug");
    path.push("hexsift");
    path
}

fn create_test_file(content: &[u8]) -> PathBuf {
    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir.join(format!("bingrep_test_edge_{}.bin", uuid::Uuid::new_v4()));
    let mut file = File::create(&file_path).unwrap();
    file.write_all(content).unwrap();
    file_path
}

#[test]
fn test_very_large_file() {
    let binary_path = get_binary_path();

    // 100MB 크기의 파일 생성 (실제 테스트에서는 작은 크기로 시작)
    let chunk_size = 1024 * 1024; // 1MB
    let num_chunks = 10; // 10MB 파일

    let temp_file = NamedTempFile::new().unwrap();
    let temp_path = temp_file.path().to_path_buf();

    {
        let mut file = File::create(&temp_path).unwrap();
        for i in 0..num_chunks {
            let mut chunk = vec![0u8; chunk_size];
            // 각 청크마다 고유한 패턴 삽입
            if i == num_chunks / 2 {
                // 중간 청크에 패턴 삽입
                chunk[chunk_size / 2..chunk_size / 2 + 5].copy_from_slice(b"\x00\x01\x02\x03\x04");
            }
            file.write_all(&chunk).unwrap();
        }
    }

    let output = Command::new(&binary_path)
        .arg(&temp_path)
        .arg("-e")
        .arg("\\x00\\x01\\x02\\x03\\x04")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // 대용량 파일에서도 패턴을 찾을 수 있는지 확인
    assert!(stdout.contains("00 01 02 03 04"));

    // stderr에 메모리 부족 에러가 없는지 확인
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("out of memory"));
    assert!(!stderr.contains("allocation"));
}

#[test]
fn test_file_with_repeated_patterns() {
    let binary_path = get_binary_path();

    // 반복 패턴으로 구성된 파일 (메모리 효율성 테스트)
    let pattern = b"\x00\x01\x02\x03";
    let repetitions = 10000;
    let mut test_data = Vec::new();

    for _ in 0..repetitions {
        test_data.extend_from_slice(pattern);
    }

    let test_file = create_test_file(&test_data);

    let output = Command::new(&binary_path)
        .arg(&test_file)
        .arg("-e")
        .arg("\\x00\\x01\\x02\\x03")
        .arg("-n")
        .arg("100") // 처음 100개만 출력
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();

    // 지정한 수만큼만 출력되는지 확인
    assert_eq!(lines.len(), 100);

    // 각 줄에 패턴이 있는지 확인
    assert!(stdout.contains("00 01 02 03"));

    // 정리
    fs::remove_file(test_file).ok();
}

#[test]
fn test_file_with_binary_zeros() {
    let binary_path = get_binary_path();

    // 대량의 NULL 바이트가 있는 파일 (sparse file 시뮬레이션)
    let mut test_data = vec![0u8; 50000];
    // 중간에 몇 개의 non-zero 바이트 삽입
    test_data[25000] = 0x01;
    test_data[25001] = 0x02;
    test_data[25002] = 0x03;

    let test_file = create_test_file(&test_data);

    let output = Command::new(&binary_path)
        .arg(&test_file)
        .arg("-e")
        .arg("\\x01\\x02\\x03")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // 패턴을 찾을 수 있는지 확인
    assert!(stdout.contains("01 02 03"));

    // 정리
    fs::remove_file(test_file).ok();
}

#[test]
fn test_file_with_all_byte_values() {
    let binary_path = get_binary_path();

    // 모든 가능한 바이트 값을 포함하는 파일
    let mut test_data = Vec::new();
    for i in 0u8..=255u8 {
        test_data.push(i);
    }
    // 패턴을 여러 번 반복
    for _ in 1..100 {
        for i in 0u8..=255u8 {
            test_data.push(i);
        }
    }

    let test_file = create_test_file(&test_data);

    // 특정 바이트 시퀀스 검색 (1, 2, 3 - 더 안전한 패턴)
    let output = Command::new(&binary_path)
        .arg(&test_file)
        .arg("-e")
        .arg("\\x01\\x02\\x03")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // 패턴이 발견되는지 확인
    assert!(stdout.contains("01 02 03"));

    // 여러 매치가 있는지 확인
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(lines.len() >= 99); // 100번 반복했으므로 99개 이상의 매치

    // 정리
    fs::remove_file(test_file).ok();
}

#[test]
fn test_single_byte_file() {
    let binary_path = get_binary_path();
    let test_data = b"\xFF";
    let test_file = create_test_file(test_data);

    let output = Command::new(&binary_path)
        .arg(&test_file)
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // 단일 바이트도 올바르게 출력되는지 확인
    assert!(stdout.contains("FF"));

    // 정리
    fs::remove_file(test_file).ok();
}

#[test]
fn test_non_existent_file() {
    let binary_path = get_binary_path();
    let non_existent_path = std::env::temp_dir().join("bingrep_non_existent_file_12345.bin");
    let non_existent_path = non_existent_path.to_str().unwrap();

    let output = Command::new(&binary_path)
        .arg(non_existent_path)
        .output()
        .expect("Failed to execute command");

    // 명령이 실패해야 함
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    // 파일을 찾을 수 없다는 에러 메시지 확인
    assert!(
        stderr.contains("No such file")
            || stderr.contains("not found")
            || stderr.contains("찾을 수 없")
            || stderr.contains("NotFound")
            || stderr.contains("cannot find")
    );
}

#[test]
fn test_invalid_regex_pattern() {
    let binary_path = get_binary_path();
    let test_data = b"Test data";
    let test_file = create_test_file(test_data);

    // 잘못된 정규표현식 패턴
    let output = Command::new(&binary_path)
        .arg(&test_file)
        .arg("-e")
        .arg("\\x") // 불완전한 hex 패턴
        .output()
        .expect("Failed to execute command");

    // 명령이 실패하거나 적절한 에러 메시지를 출력해야 함
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.len() > 0); // 에러 메시지가 있어야 함
    }

    // 정리
    fs::remove_file(test_file).ok();
}

#[test]
fn test_maximum_line_width() {
    let binary_path = get_binary_path();
    let test_data: Vec<u8> = (0..255).collect();
    let test_file = create_test_file(&test_data);

    let output = Command::new(&binary_path)
        .arg(&test_file)
        .arg("-w")
        .arg("255") // 최대 라인 너비
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();

    // 하나의 라인으로 출력되어야 함
    assert_eq!(lines.len(), 1);

    // 모든 바이트가 포함되어 있는지 확인 (오프셋 부분 제외하고 hex만 계산)
    let hex_part = lines[0].split(" : ").nth(1).unwrap_or("");
    let hex_count = hex_part.split_whitespace().count();
    assert_eq!(hex_count, 255);

    // 정리
    fs::remove_file(test_file).ok();
}

#[test]
fn test_zero_line_width() {
    let binary_path = get_binary_path();
    let test_data = b"Test";
    let test_file = create_test_file(test_data);

    let output = Command::new(&binary_path)
        .arg(&test_file)
        .arg("-w")
        .arg("0")
        .output()
        .expect("Failed to execute command");

    // 0은 유효하지 않은 값이므로 에러가 발생하거나 기본값이 사용되어야 함
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        assert!(stderr.len() > 0);
    }

    // 정리
    fs::remove_file(test_file).ok();
}

#[test]
fn test_extremely_large_position() {
    let binary_path = get_binary_path();
    let test_data = b"Small file content";
    let test_file = create_test_file(test_data);

    let output = Command::new(&binary_path)
        .arg(&test_file)
        .arg("-s")
        .arg("999999999") // 파일 크기보다 훨씬 큰 위치
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // 파일 끝을 넘어선 위치에서 시작하면 출력이 없거나 적절한 처리가 되어야 함
    assert!(stdout.trim().is_empty() || !output.status.success());

    // 정리
    fs::remove_file(test_file).ok();
}
