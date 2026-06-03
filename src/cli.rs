use clap::{Parser, ValueEnum};

#[derive(Parser)]
#[command(name = "hexsift")]
#[command(about = "바이너리 파일 정규표현식 검색 도구")]
#[command(version = "0.1.0")]
#[command(long_about = "바이너리 파일을 정규표현식으로 검색하는 도구입니다.

이 프로그램의 정규표현식은 Rust regex 라이브러리의 문법을 따릅니다.

Example 01 파일 내용을 HEX값으로 출력:
    hexsift \"path_to_file.txt\"
    hexsift \"path_to_file.txt\" -n 10    (10줄만 출력)

Example 02 파일 내용을 정규표현식으로 검색:
    hexsift \"path_to_file.txt\" -e \"\\x00\\x00\\x00\\x01\\x67\" -w 100

Example 03 Windows 물리 디스크 검색 (관리자 권한 필요):
    hexsift --list-disks
    hexsift \"\\\\.\\PHYSICALDRIVE0\" -e \"\\x00\\x00\\x01\\x67\"

Example 04 임의의 32바이트 뒤에 오는 시그니처 검색:
    hexsift \"\\\\.\\PHYSICALDRIVE0\" -e \"(?s-u).{32}\\x00\\x00\\x00\\x01\\x67\" -w 37")]
pub struct Cli {
    /// 입력 파일 경로 또는 glob 패턴 (예: "*.bin", "data/**/*.txt")
    pub file_path: Option<String>,

    /// 정규표현식 패턴 (예: -e "\x00\x00\x00\x01\x67")
    #[arg(short = 'e', long = "regex")]
    pub expression: Option<String>,

    /// 한 줄에 표시할 바이트 개수 (기본값: 16)
    #[arg(short = 'w', long = "width", default_value = "16")]
    pub line_width: usize,

    /// 출력할 라인 수 (0: 무제한)
    #[arg(short = 'n', long = "line", default_value = "0")]
    pub limit: usize,

    /// 시작 위치 (바이트 단위)
    #[arg(short = 's', long = "position", default_value = "0")]
    pub position: u64,

    /// 바이트 문자열 분리 기호
    #[arg(short = 't', long = "separator", default_value = " ")]
    pub separator: String,

    /// 오프셋 출력 안함
    #[arg(long = "no-offset")]
    pub no_offset: bool,

    /// 병렬 처리 사용 (큰 파일에서 성능 향상)
    #[arg(short = 'p', long = "parallel")]
    pub parallel: bool,

    /// 청크 크기 (병렬 처리 시, 바이트 단위, 기본값: 16MB)
    #[arg(long = "chunk-size", default_value = "16777216")]
    pub chunk_size: usize,

    /// 복잡한 정규식의 경계 검색 overlap 크기 (바이트 단위, 기본값: 8KB)
    #[arg(long = "overlap-size")]
    pub overlap_size: Option<usize>,

    /// 다중 파일 모드 (glob 패턴 또는 여러 파일 처리)
    #[arg(short = 'm', long = "multi")]
    pub multi_file: bool,

    /// Windows 물리 디스크 목록 출력
    #[arg(short = 'l', long = "list-disks")]
    pub list_disks: bool,

    /// 전체 파일에 대한 전역 제한 (0: 무제한)
    #[arg(long = "global-limit", default_value = "0")]
    pub global_limit: usize,

    /// 출력 형식 (hex, json, csv, plain)
    #[arg(short = 'f', long = "format", default_value = "hex")]
    pub output_format: String,

    /// 진행률 표시 (대용량 파일 처리 시)
    #[arg(long = "progress")]
    pub show_progress: bool,

    /// 색상 출력 설정 (always, never, auto)
    #[arg(long = "color", default_value = "auto")]
    pub color: ColorChoice,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum ColorChoice {
    /// 항상 색상 출력
    Always,
    /// 색상 출력 안함
    Never,
    /// 터미널일 때만 색상 출력
    Auto,
}
