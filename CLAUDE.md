# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

hexsift is a Rust-based binary file search tool that displays files in hexadecimal format and searches for regex patterns in binary data. It supports hexadecimal escape sequences (\xHH format) and is optimized for large file processing with streaming and parallel capabilities.

## Development Commands

### Build Commands
```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Alternative using make/just
make build                # or: just build
make build-release        # or: just build-release
```

### Testing Commands
```bash
# Run all tests
./run_tests.sh

# Individual test commands
cargo test                            # Unit tests
cargo test --test integration_test    # Integration tests
cargo test --test multifile_test      # Multi-file processing tests
cargo test --test parallel_test       # Parallel processing tests
cargo test --test regex_quantifiers_test  # Regex quantifier tests

# Run ignored tests
cargo test -- --ignored

# Alternative using make/just
make test                 # Basic tests
make test-all            # All tests including ignored ones
```

### Development Tools
```bash
# Code formatting
cargo fmt                # or: make fmt / just fmt

# Linting
cargo clippy            # or: make clippy / just clippy

# Syntax checking
cargo check             # or: make check / just check

# Clean build artifacts
cargo clean             # or: make clean / just clean
```

### Combined Commands
```bash
# Development workflow
make dev-test           # Run tests + clippy
make pre-commit         # Format + lint + test (recommended before commits)
make ci                 # Full CI pipeline
```

## Architecture Overview

### Core Components

**Main Entry Point**: `src/main.rs`
- CLI argument parsing using clap
- File type detection (regular files vs forensic images)
- Processing orchestration (regular vs parallel)
- Stdin input handling

**Stream Processing**: `src/stream.rs`
- `FileProcessor`: Core file processing logic
- Streaming approach for memory efficiency with large files
- Regex pattern matching on binary data

**Parallel Processing**: `src/parallel.rs`
- `ParallelProcessor`: Multi-threaded regex search
- `ParallelHexDump`: Multi-threaded hex dump
- Chunk-based processing for large files

**Multi-file Support**: `src/multifile.rs`
- `MultiFileProcessor`: Glob pattern file processing
- Batch processing capabilities

**Regex Processing**: `src/regex_processor.rs`
- Pattern compilation and validation
- Hex escape sequence handling (\xHH format)
- Binary regex search using rust regex crate

**Output Formatting**: `src/output.rs` & `src/structured_output.rs`
- `OutputFormatter`: Hex display formatting
- Configurable separators, line widths, offset display
- Structured output formats (JSON, CSV)

**Buffer Management**: `src/buffer_manager.rs`
- Efficient memory management for streaming
- Optimized for large file processing

**Forensic Image Support**: `src/forensic_image.rs`
- E01/EWF forensic image format detection
- Integration with exhume_body library (optional feature)

### Key Features

1. **Binary Pattern Search**: Uses regex on raw binary data with hex escape sequences
2. **Streaming Architecture**: Memory-efficient processing of large files
3. **Parallel Processing**: Multi-threaded processing for performance on large files
4. **Forensic Image Support**: Can process E01/EWF forensic disk images
5. **Flexible Output**: Configurable hex display formats and structured output
6. **Multi-file Processing**: Glob pattern support for batch operations

### Configuration

**Features**:
- `exhume` (default): Enables forensic image support via exhume_body crate
- Can be disabled with `--no-default-features`

**CLI Interface**: Defined in `src/cli.rs` using clap derive macros
- Supports both single files and glob patterns
- Configurable output formatting options
- Parallel processing controls

### Test Structure

Tests are organized by functionality:
- `integration_test.rs`: End-to-end functionality tests
- `parallel_test.rs`: Multi-threading and performance tests
- `multifile_test.rs`: Glob pattern and batch processing tests
- `regex_quantifiers_test.rs`: Regex pattern matching tests
- `structured_output_test.rs`: Output format tests
- `edge_case_test.rs`: Error handling and edge cases
- `concurrency_test.rs`: Thread safety tests

## Important Notes

- The project includes both Makefile and justfile for build automation
- All documentation is in Korean, reflecting the target user base
- Forensic image support requires the exhume_body external dependency
- Parallel processing is automatically enabled for files larger than chunk_size
- The tool can process stdin input when file path is "-"