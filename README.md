# hexsift

고성능 바이너리 파일 검색 및 16진수 표시 도구

## 개요

hexsift은 바이너리 파일을 16진수로 표시하고 정규표현식 패턴을 검색하는 Rust 기반 도구입니다. 대용량 파일의 효율적인 처리와 포렌식 이미지 분석을 위해 설계되었습니다.

## 주요 기능

- **바이너리 패턴 검색**: `\xHH` 형식의 16진수 패턴 지원
- **고성능 처리**: 스트리밍 및 병렬 처리로 대용량 파일 처리
- **다양한 입력 형식**: 일반 파일, stdin 입력, Windows 물리 디스크, 포렌식 이미지 (E01/EWF) 지원
- **유연한 출력**: 사용자 정의 가능한 16진수 표시 형식
- **멀티파일 처리**: glob 패턴을 사용한 배치 처리

## 설치

### 소스에서 빌드

```bash
git clone <repository-url>
cd hexsift
cargo build --release
```

빌드된 바이너리는 `./target/release/hexsift`에 생성됩니다.

## 사용법

### 기본 사용법

```bash
# 파일을 16진수로 표시
hexsift file.bin

# 정규표현식으로 패턴 검색
hexsift file.bin -e "\x00\x00\x00\x01\x67"

# stdin에서 입력 받기
cat file.bin | hexsift -

# 한 줄에 8바이트씩 표시
hexsift file.bin -w 8
```

### 명령줄 옵션

| 옵션                    | 설명                                  |
| ----------------------- | ------------------------------------- |
| `-e, --regex <PATTERN>` | 검색할 정규표현식 패턴                |
| `-w, --width <N>`       | 한 줄에 표시할 바이트 수 (기본값: 16) |
| `-n, --line <N>`        | 출력할 줄 수 제한 (0: 무제한)         |
| `-s, --position <N>`    | 시작 위치 (바이트 단위)               |
| `-t, --separator <STR>` | 바이트 구분자 (기본값: 공백)          |
| `--no-offset`           | 오프셋 숨기기                         |
| `--parallel`            | 병렬 처리 활성화                      |
| `--multi-file`          | 멀티파일 모드                         |

## 사용 예제

### H.264 비디오 분석

```bash
# SPS (Sequence Parameter Set) 검색
hexsift video.mp4 -e "\x00\x00\x00\x01\x67"

# NAL unit 시작 코드 (유연한 패턴)
hexsift video.mp4 -e "\x00{2,3}\x01"
```

### 실행 파일 분석

```bash
# PE 헤더 검색
hexsift program.exe -e "\x4D\x5A"

# ELF 헤더 검색
hexsift program -e "\x7F\x45\x4C\x46"
```

### 포렌식 분석

```bash
# E01 포렌식 이미지 분석
hexsift evidence.E01 -e "\x53\x51\x4C\x69\x74\x65"

# Windows 물리 디스크 분석 (관리자 권한 터미널 필요)
hexsift "\\.\PHYSICALDRIVE0" -e "\x00\x00\x01\x67"

# 멀티파일 검색
hexsift "*.bin" --multi-file -e "\xFF\xD8\xFF"
```

### 정규표현식 수량자

```bash
# 정확히 4개의 NULL 바이트
hexsift file.bin -e "\x00{4}"

# 2-4개의 NULL 바이트
hexsift file.bin -e "\x00{2,4}"

# 1개 이상의 0xFF 바이트
hexsift file.bin -e "\xFF+"
```

## 개발

### 빌드 및 테스트

```bash
# 개발 빌드
cargo build

# 릴리즈 빌드
cargo build --release

# 전체 테스트 실행
./run_tests.sh

# 개발자 워크플로우
make pre-commit  # 포맷팅 + 린팅 + 테스트
```

### 프로젝트 구조

- `src/main.rs` - 메인 엔트리 포인트
- `src/stream.rs` - 파일 스트림 처리
- `src/parallel.rs` - 병렬 처리 로직
- `src/regex_processor.rs` - 정규표현식 처리
- `src/output.rs` - 출력 포맷팅
- `src/forensic_image.rs` - 포렌식 이미지 지원

## 성능 특성

- **메모리 효율적**: 스트리밍 방식으로 대용량 파일 처리
- **병렬 처리**: 멀티코어를 활용한 빠른 검색
- **최적화된 I/O**: 청크 기반 버퍼링
- **안전성**: Rust의 메모리 안전성 보장

## 기여하기

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run `make pre-commit` to ensure code quality
5. Submit a pull request

## 라이선스

이 프로젝트는 원본 프로젝트와 동일한 라이선스를 따릅니다.
