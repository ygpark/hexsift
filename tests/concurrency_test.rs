use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Duration;

fn get_binary_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_hexsift"))
}

fn create_test_file(content: &[u8], suffix: &str) -> PathBuf {
    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir.join(format!(
        "bingrep_concurrent_test_{}_{}.bin",
        uuid::Uuid::new_v4(),
        suffix
    ));
    let mut file = File::create(&file_path).unwrap();
    file.write_all(content).unwrap();
    file_path
}

#[test]
fn test_concurrent_file_access() {
    let binary_path = get_binary_path();
    let test_data = b"Concurrent access test data with pattern \x00\x01\x02\x03 in the middle";
    let test_file = create_test_file(test_data, "concurrent");

    let num_threads = 10;
    let barrier = Arc::new(Barrier::new(num_threads));
    let mut handles = Vec::new();

    for i in 0..num_threads {
        let binary_path = binary_path.clone();
        let test_file = test_file.clone();
        let barrier = Arc::clone(&barrier);

        let handle = thread::spawn(move || {
            // 모든 스레드가 동시에 시작하도록 대기
            barrier.wait();

            let output = Command::new(&binary_path)
                .arg(&test_file)
                .arg("-e")
                .arg("\\x00\\x01\\x02\\x03")
                .output()
                .expect(&format!("Failed to execute command in thread {}", i));

            (
                output.status.success(),
                String::from_utf8_lossy(&output.stdout).to_string(),
                String::from_utf8_lossy(&output.stderr).to_string(),
            )
        });

        handles.push(handle);
    }

    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

    // 모든 스레드가 성공적으로 실행되었는지 확인
    for (i, (success, stdout, stderr)) in results.iter().enumerate() {
        assert!(*success, "Thread {} failed. stderr: {}", i, stderr);
        assert!(
            stdout.contains("00 01 02 03"),
            "Thread {} didn't find pattern",
            i
        );
    }

    // 모든 결과가 동일한지 확인
    let first_stdout = &results[0].1;
    for (i, (_, stdout, _)) in results.iter().enumerate().skip(1) {
        assert_eq!(
            stdout, first_stdout,
            "Thread {} produced different output",
            i
        );
    }

    // 정리
    fs::remove_file(test_file).ok();
}

#[test]
fn test_concurrent_different_files() {
    let binary_path = get_binary_path();
    let num_threads = 5;
    let barrier = Arc::new(Barrier::new(num_threads));
    let mut handles = Vec::new();

    for i in 0..num_threads {
        let binary_path = binary_path.clone();
        let barrier = Arc::clone(&barrier);

        let handle = thread::spawn(move || {
            // 각 스레드마다 다른 파일 생성
            let test_data = format!(
                "Thread {} data with pattern \\x0{}\\x0{}\\x0{}\\x0{}",
                i, i, i, i, i
            )
            .into_bytes();
            let mut actual_data = test_data.clone();
            // 실제 바이너리 패턴 삽입
            let pattern = vec![i as u8, i as u8, i as u8, i as u8];
            actual_data.extend_from_slice(&pattern);

            let test_file = create_test_file(&actual_data, &format!("thread_{}", i));

            // 모든 스레드가 동시에 시작하도록 대기
            barrier.wait();

            let pattern_str = format!("\\x{:02x}\\x{:02x}\\x{:02x}\\x{:02x}", i, i, i, i);
            let output = Command::new(&binary_path)
                .arg(&test_file)
                .arg("-e")
                .arg(&pattern_str)
                .output()
                .expect(&format!("Failed to execute command in thread {}", i));

            let result = (
                output.status.success(),
                String::from_utf8_lossy(&output.stdout).to_string(),
                String::from_utf8_lossy(&output.stderr).to_string(),
                test_file,
            );

            result
        });

        handles.push(handle);
    }

    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

    // 모든 스레드가 성공적으로 실행되었는지 확인
    for (i, (success, stdout, stderr, test_file)) in results.iter().enumerate() {
        assert!(*success, "Thread {} failed. stderr: {}", i, stderr);

        // 각 스레드의 패턴이 발견되었는지 확인
        let expected_hex = format!("{:02X} {:02X} {:02X} {:02X}", i, i, i, i);
        assert!(
            stdout.contains(&expected_hex),
            "Thread {} didn't find its pattern. stdout: {}",
            i,
            stdout
        );

        // 파일 정리
        fs::remove_file(test_file).ok();
    }
}

#[test]
fn test_concurrent_different_patterns() {
    let binary_path = get_binary_path();

    // 여러 패턴이 포함된 큰 테스트 데이터 생성
    let mut test_data = Vec::new();
    test_data.extend_from_slice(b"Start of file ");
    for i in 0u8..10u8 {
        test_data.extend_from_slice(&format!("Pattern {} before ", i).into_bytes());
        test_data.extend_from_slice(&[i, i, i, i]); // 패턴
        test_data.extend_from_slice(&format!(" after pattern {}\n", i).into_bytes());
    }
    test_data.extend_from_slice(b"End of file");

    let test_file = create_test_file(&test_data, "multipattern");

    let num_patterns = 5;
    let barrier = Arc::new(Barrier::new(num_patterns));
    let mut handles = Vec::new();

    for i in 0..num_patterns {
        let binary_path = binary_path.clone();
        let test_file = test_file.clone();
        let barrier = Arc::clone(&barrier);

        let handle = thread::spawn(move || {
            barrier.wait();

            let pattern_str = format!("\\x{:02x}\\x{:02x}\\x{:02x}\\x{:02x}", i, i, i, i);
            let output = Command::new(&binary_path)
                .arg(&test_file)
                .arg("-e")
                .arg(&pattern_str)
                .output()
                .expect(&format!("Failed to execute command for pattern {}", i));

            (
                i,
                output.status.success(),
                String::from_utf8_lossy(&output.stdout).to_string(),
                String::from_utf8_lossy(&output.stderr).to_string(),
            )
        });

        handles.push(handle);
    }

    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

    // 결과를 패턴 순서대로 정렬
    let mut sorted_results = results;
    sorted_results.sort_by_key(|(i, _, _, _)| *i);

    // 각 패턴이 올바르게 찾아졌는지 확인
    for (i, success, stdout, stderr) in sorted_results {
        assert!(success, "Pattern {} search failed. stderr: {}", i, stderr);

        let expected_hex = format!("{:02X} {:02X} {:02X} {:02X}", i, i, i, i);
        assert!(
            stdout.contains(&expected_hex),
            "Pattern {} not found. stdout: {}",
            i,
            stdout
        );
    }

    // 정리
    fs::remove_file(test_file).ok();
}

#[test]
fn test_stress_concurrent_execution() {
    let binary_path = get_binary_path();
    let test_data =
        b"Stress test data \x42\x42\x42\x42 with repeating pattern \x42\x42\x42\x42 multiple times";
    let test_file = create_test_file(test_data, "stress");

    let num_threads = 20;
    let iterations_per_thread = 5;
    let barrier = Arc::new(Barrier::new(num_threads));
    let mut handles = Vec::new();

    for thread_id in 0..num_threads {
        let binary_path = binary_path.clone();
        let test_file = test_file.clone();
        let barrier = Arc::clone(&barrier);

        let handle = thread::spawn(move || {
            barrier.wait();

            let mut all_success = true;
            let mut match_counts = Vec::new();

            for iteration in 0..iterations_per_thread {
                let output = Command::new(&binary_path)
                    .arg(&test_file)
                    .arg("-e")
                    .arg("\\x42\\x42\\x42\\x42")
                    .output()
                    .expect(&format!(
                        "Failed to execute command in thread {} iteration {}",
                        thread_id, iteration
                    ));

                if !output.status.success() {
                    all_success = false;
                    break;
                }

                let stdout = String::from_utf8_lossy(&output.stdout);
                let match_count = stdout.lines().count();
                match_counts.push(match_count);

                // 작은 지연을 추가하여 스케줄링 테스트
                thread::sleep(Duration::from_millis(1));
            }

            (thread_id, all_success, match_counts)
        });

        handles.push(handle);
    }

    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

    // 모든 스레드가 성공했는지 확인
    for (thread_id, success, match_counts) in &results {
        assert!(*success, "Thread {} failed during stress test", thread_id);

        // 모든 반복에서 동일한 매치 수를 얻었는지 확인
        assert!(
            !match_counts.is_empty(),
            "Thread {} got no results",
            thread_id
        );
        let first_count = match_counts[0];
        for (i, &count) in match_counts.iter().enumerate() {
            assert_eq!(
                count, first_count,
                "Thread {} iteration {} got different match count: {} vs {}",
                thread_id, i, count, first_count
            );
        }
    }

    // 모든 스레드가 동일한 결과를 얻었는지 확인
    let expected_count = results[0].2[0];
    for (thread_id, _, match_counts) in &results {
        assert_eq!(
            match_counts[0], expected_count,
            "Thread {} got different match count: {} vs {}",
            thread_id, match_counts[0], expected_count
        );
    }

    // 정리
    fs::remove_file(test_file).ok();
}

#[test]
fn test_concurrent_with_different_options() {
    let binary_path = get_binary_path();
    let test_data = b"Options test \x11\x22\x33\x44 with pattern \x11\x22\x33\x44 repeated";
    let test_file = create_test_file(test_data, "options");

    let barrier = Arc::new(Barrier::new(4));
    let mut handles = Vec::new();

    // 다른 옵션으로 동시 실행
    let test_configs = vec![
        ("basic", vec!["-e", "\\x11\\x22\\x33\\x44"]),
        ("with_width", vec!["-e", "\\x11\\x22\\x33\\x44", "-w", "8"]),
        ("with_limit", vec!["-e", "\\x11\\x22\\x33\\x44", "-n", "1"]),
        (
            "no_offset",
            vec!["-e", "\\x11\\x22\\x33\\x44", "--no-offset"],
        ),
    ];

    for (test_name, args) in test_configs {
        let binary_path = binary_path.clone();
        let test_file = test_file.clone();
        let barrier = Arc::clone(&barrier);
        let args = args.clone();

        let handle = thread::spawn(move || {
            barrier.wait();

            let mut cmd = Command::new(&binary_path);
            cmd.arg(&test_file);
            for arg in args {
                cmd.arg(arg);
            }

            let output = cmd
                .output()
                .expect(&format!("Failed to execute command for {}", test_name));

            (
                test_name,
                output.status.success(),
                String::from_utf8_lossy(&output.stdout).to_string(),
            )
        });

        handles.push(handle);
    }

    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

    // 모든 구성이 성공했는지 확인
    for (test_name, success, stdout) in &results {
        assert!(*success, "Test configuration '{}' failed", test_name);
        assert!(
            stdout.contains("11 22 33 44"),
            "Test configuration '{}' didn't find pattern",
            test_name
        );
    }

    // 정리
    fs::remove_file(test_file).ok();
}
