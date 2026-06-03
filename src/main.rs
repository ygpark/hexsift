use clap::Parser;
use hexsift::cli::Cli;
use hexsift::config::Config;
use hexsift::error::Result;
use hexsift::multifile::MultiFileProcessor;
use hexsift::output::OutputFormatter;
use hexsift::parallel::{ParallelHexDump, ParallelProcessor};
use hexsift::progress::ProgressIndicator;
use hexsift::regex_processor::RegexProcessor;
use hexsift::stream::FileProcessor;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

/// Validate and canonicalize file path to prevent path traversal attacks
fn validate_file_path(path: &str) -> Result<PathBuf> {
    let path = Path::new(path);

    // Check for potentially dangerous path components
    if path
        .components()
        .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err(hexsift::error::BingrepError::InvalidPath(
            "Path contains parent directory references (..)".to_string(),
        ));
    }

    // Canonicalize the path to resolve any symlinks and relative paths
    match path.canonicalize() {
        Ok(canonical_path) => Ok(canonical_path),
        Err(_) => {
            // If canonicalization fails, it might be because the file doesn't exist
            // In this case, we'll validate the path structure but allow it through
            if path.is_absolute() || path.components().count() == 1 {
                Ok(path.to_path_buf())
            } else {
                Err(hexsift::error::BingrepError::InvalidPath(
                    "Invalid or inaccessible file path".to_string(),
                ))
            }
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Set global color choice
    hexsift::color_context::set_color_choice(cli.color.clone());

    // Check file path or stdin
    let file_path = match &cli.file_path {
        Some(path) => {
            if path == "-" {
                // Handle stdin input
                return handle_stdin_input(&cli);
            }
            // Validate file path for security
            validate_file_path(path)?
        }
        None => {
            // Clap will automatically show help when no file path is provided
            eprintln!("Error: 파일 경로가 필요합니다.\n");
            eprintln!("사용법: hexsift <파일경로> [옵션]");
            eprintln!("사용법: hexsift - [옵션] < input_file (stdin)");
            eprintln!("도움말: hexsift --help");
            return Ok(());
        }
    };

    // Handle multi-file processing
    if cli.multi_file {
        let config = Config::default();
        config.validate_cli(&cli)?;

        let multi_processor = MultiFileProcessor::new(config);

        return multi_processor.process_files_by_glob(
            &file_path.to_string_lossy(),
            cli.expression.as_deref(),
            cli.line_width,
            cli.limit,
            &cli.separator,
            !cli.no_offset,
            cli.parallel,
            cli.chunk_size,
            cli.global_limit,
        );
    }

    // Create configuration and validate CLI parameters
    let config = Config::default();
    config.validate_cli(&cli)?;

    let mut processor = FileProcessor::new(config.clone());

    // Check if this is a forensic image file (E01, VMDK) and handle accordingly
    if hexsift::forensic_image::is_forensic_image_path(&file_path)? {
        // Process forensic image file - parallel processing not supported for forensic images yet
        let format_name =
            hexsift::forensic_image::detect_format_name(&file_path)?.unwrap_or("Unknown");
        eprintln!(
            "Detected {} forensic image: {}",
            format_name,
            file_path.display()
        );

        // Forensic images (E01) do not support progress due to exhume_body library limitations
        let mut progress = ProgressIndicator::disabled();

        if let Some(expression) = cli.expression {
            let regex = RegexProcessor::compile_pattern(&expression)?;
            processor.process_stream_by_regex_from_path(
                &file_path,
                &regex,
                cli.line_width,
                cli.limit,
                &cli.separator,
                !cli.no_offset,
                &mut progress,
            )?;
        } else {
            processor.process_file_stream_from_path(
                &file_path,
                cli.line_width,
                cli.limit,
                &cli.separator,
                !cli.no_offset,
                &mut progress,
            )?;
        }
    } else {
        // Open regular file
        let mut file = File::open(&file_path)?;
        let file_size = file.metadata()?.len();

        // Validate file size doesn't exceed limits
        config.validate_file_size(file_size)?;

        // Seek to starting position
        file.seek(SeekFrom::Start(cli.position))?;

        // Create progress indicator if requested
        let show_progress = cli.show_progress && ProgressIndicator::should_show_progress();
        let mut progress = if show_progress {
            ProgressIndicator::new(file_size - cli.position, true)
        } else {
            ProgressIndicator::disabled()
        };

        // Process file with or without regex
        if let Some(expression) = cli.expression {
            let regex = RegexProcessor::compile_pattern(&expression)?;

            if cli.parallel && file_size > cli.chunk_size as u64 {
                // Use parallel processing for large files
                ParallelProcessor::process_file_parallel(
                    &mut file,
                    &regex,
                    cli.chunk_size,
                    cli.line_width,
                    cli.limit,
                    &cli.separator,
                    !cli.no_offset,
                    file_size,
                )?;
            } else {
                // Use regular processing
                processor.process_stream_by_regex(
                    &mut file,
                    &regex,
                    cli.line_width,
                    cli.limit,
                    &cli.separator,
                    !cli.no_offset,
                    &mut progress,
                )?;
            }
        } else {
            if cli.parallel && file_size > cli.chunk_size as u64 {
                // Use parallel processing for hex dump
                ParallelHexDump::process_file_parallel(
                    &mut file,
                    cli.chunk_size,
                    cli.line_width,
                    cli.limit,
                    &cli.separator,
                    !cli.no_offset,
                    file_size,
                )?;
            } else {
                // Use regular processing
                processor.process_file_stream(
                    &mut file,
                    cli.line_width,
                    cli.limit,
                    &cli.separator,
                    !cli.no_offset,
                    file_size,
                    &mut progress,
                )?;
            }
        }
    }

    Ok(())
}

/// Handle stdin input processing
fn handle_stdin_input(cli: &Cli) -> Result<()> {
    let config = Config::default();
    config.validate_cli(cli)?;

    // Read all data from stdin into a buffer
    let mut stdin_data = Vec::new();
    io::stdin().read_to_end(&mut stdin_data)?;

    if stdin_data.is_empty() {
        eprintln!("Warning: No data received from stdin");
        return Ok(());
    }

    let data_size = stdin_data.len() as u64;

    // Process data with or without regex
    if let Some(expression) = &cli.expression {
        let regex = RegexProcessor::compile_pattern(expression)?;
        process_stdin_with_regex(&stdin_data, &regex, cli, data_size)?;
    } else {
        process_stdin_hex_dump(&stdin_data, cli, data_size)?;
    }

    Ok(())
}

/// Process stdin data with regex search
fn process_stdin_with_regex(
    data: &[u8],
    regex: &regex::bytes::Regex,
    cli: &Cli,
    data_size: u64,
) -> Result<()> {
    let hex_offset_length = OutputFormatter::calculate_hex_offset_length(data_size);
    let mut match_count = 0;

    for mat in regex.find_iter(data) {
        let match_offset = mat.start() as u64;
        let end_pos = (mat.start() + cli.line_width).min(data.len());
        let display_bytes = &data[mat.start()..end_pos];

        let hex_string = OutputFormatter::format_bytes_as_hex(display_bytes, &cli.separator);
        OutputFormatter::print_line(match_offset, &hex_string, !cli.no_offset, hex_offset_length);

        match_count += 1;
        if cli.limit > 0 && match_count >= cli.limit {
            break;
        }
    }

    Ok(())
}

/// Process stdin data as hex dump
fn process_stdin_hex_dump(data: &[u8], cli: &Cli, data_size: u64) -> Result<()> {
    let hex_offset_length = OutputFormatter::calculate_hex_offset_length(data_size);
    let mut pos = 0;
    let mut line = 0;

    while pos < data.len() {
        let end_pos = (pos + cli.line_width).min(data.len());
        let line_bytes = &data[pos..end_pos];

        let hex_string = OutputFormatter::format_bytes_as_hex(line_bytes, &cli.separator);
        OutputFormatter::print_line(pos as u64, &hex_string, !cli.no_offset, hex_offset_length);

        pos += cli.line_width;
        line += 1;

        if cli.limit > 0 && line >= cli.limit {
            break;
        }
    }

    Ok(())
}
