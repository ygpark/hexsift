use clap::Parser;
use hexsift::cli::Cli;
use hexsift::config::Config;
use hexsift::error::Result;
use hexsift::multifile::MultiFileProcessor;
use hexsift::output::OutputFormatter;
use hexsift::parallel::{ParallelHexDump, ParallelProcessor};
use hexsift::physical_device::{
    format_drive_details, format_drive_size, format_sector_size, get_physical_drive_size,
    is_physical_drive_path, list_physical_drives, PHYSICAL_DEVICE_SIZE_FALLBACK,
};
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

    if cli.list_disks {
        return handle_list_disks();
    }

    // Check file path or stdin
    let (file_path, is_physical_drive) = match &cli.file_path {
        Some(path) => {
            if path == "-" {
                // Handle stdin input
                return handle_stdin_input(&cli);
            }

            if is_physical_drive_path(path) {
                (PathBuf::from(path), true)
            } else {
                // Validate file path for security
                (validate_file_path(path)?, false)
            }
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
            cli.overlap_size,
            cli.global_limit,
        );
    }

    // Create configuration and validate CLI parameters
    let config = Config::default();
    config.validate_cli(&cli)?;

    let mut processor = FileProcessor::new(config.clone());

    if is_physical_drive {
        eprintln!("Detected Windows physical drive: {}", file_path.display());
        let mut file = match File::open(&file_path) {
            Ok(file) => file,
            Err(err) if err.kind() == io::ErrorKind::PermissionDenied => {
                return Err(hexsift::error::BingrepError::Io(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    format!(
                        "Access denied opening {}. Run your terminal as Administrator.",
                        file_path.display()
                    ),
                )));
            }
            Err(err) => return Err(err.into()),
        };
        let file_size = get_physical_drive_size(&mut file).unwrap_or(PHYSICAL_DEVICE_SIZE_FALLBACK);

        file.seek(SeekFrom::Start(cli.position))?;

        let mut progress = ProgressIndicator::disabled();

        if let Some(expression) = cli.expression {
            let regex = RegexProcessor::compile_pattern(&expression)?;
            let overlap_size = overlap_for_stream(&expression, &config, cli.overlap_size);
            processor.process_stream_by_regex(
                &mut file,
                &regex,
                cli.line_width,
                cli.limit,
                &cli.separator,
                !cli.no_offset,
                overlap_size,
                &mut progress,
            )?;
        } else {
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

        return Ok(());
    }

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
            let overlap_size = overlap_for_stream(&expression, &config, cli.overlap_size);
            processor.process_stream_by_regex_from_path(
                &file_path,
                &regex,
                cli.line_width,
                cli.limit,
                &cli.separator,
                !cli.no_offset,
                overlap_size,
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
                let parallel_overlap =
                    overlap_for_parallel(&expression, &config, cli.chunk_size, cli.overlap_size);

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
                    parallel_overlap,
                )?;
            } else {
                let stream_overlap = overlap_for_stream(&expression, &config, cli.overlap_size);

                // Use regular processing
                processor.process_stream_by_regex(
                    &mut file,
                    &regex,
                    cli.line_width,
                    cli.limit,
                    &cli.separator,
                    !cli.no_offset,
                    stream_overlap,
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

fn overlap_for_stream(expression: &str, config: &Config, explicit_overlap: Option<usize>) -> usize {
    let default_overlap = explicit_overlap.unwrap_or(config.buffer_padding);
    let overlap = RegexProcessor::overlap_for_expression(expression, default_overlap);
    warn_for_large_overlap(overlap, default_overlap, explicit_overlap.is_some());
    overlap
}

fn overlap_for_parallel(
    expression: &str,
    config: &Config,
    chunk_size: usize,
    explicit_overlap: Option<usize>,
) -> usize {
    let default_overlap = explicit_overlap.unwrap_or(config.buffer_padding.min(chunk_size / 10));
    let overlap = RegexProcessor::overlap_for_expression(expression, default_overlap);
    warn_for_large_overlap(overlap, default_overlap, explicit_overlap.is_some());
    overlap
}

fn warn_for_large_overlap(overlap: usize, default_overlap: usize, explicit_overlap: bool) {
    const LARGE_OVERLAP_WARNING_THRESHOLD: usize = 64 * 1024;

    if overlap > LARGE_OVERLAP_WARNING_THRESHOLD && overlap > default_overlap {
        eprintln!(
            "Warning: pattern requires {} bytes of overlap; consider increasing --chunk-size for parallel searches.",
            overlap
        );
    } else if explicit_overlap && overlap > LARGE_OVERLAP_WARNING_THRESHOLD {
        eprintln!(
            "Warning: using large overlap size {} bytes; this may increase memory and duplicate scanning work.",
            overlap
        );
    }
}

fn handle_list_disks() -> Result<()> {
    let drives = list_physical_drives();

    if drives.is_empty() {
        println!("No Windows physical drives found.");
        return Ok(());
    }

    println!(
        "{:<22} {:>12}  {:>8}  {:<14}  {}",
        "Path", "Size", "Sector", "Status", "Details"
    );
    for drive in drives {
        let status = if drive.accessible {
            "accessible".to_string()
        } else {
            drive.note.unwrap_or_else(|| "not accessible".to_string())
        };

        println!(
            "{:<22} {:>12}  {:>8}  {:<14}  {}",
            drive.path,
            format_drive_size(drive.size),
            format_sector_size(drive.metadata.bytes_per_sector),
            status,
            format_drive_details(&drive.metadata)
        );
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
