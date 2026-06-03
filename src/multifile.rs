use crate::config::Config;
use crate::error::Result;
use crate::forensic_image::is_forensic_image_path;
use crate::parallel::{ParallelHexDump, ParallelProcessor};
use crate::progress::ProgressIndicator;
use crate::regex_processor::RegexProcessor;
use crate::stream::FileProcessor;
use glob::glob;
use std::fs::File;
use std::path::Path;

/// Multi-file processor for handling glob patterns and multiple files
pub struct MultiFileProcessor {
    config: Config,
}

impl MultiFileProcessor {
    /// Create a new MultiFileProcessor
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    /// Process multiple files using glob pattern
    ///
    /// # Arguments
    ///
    /// * `pattern` - Glob pattern to match files (e.g., "*.bin", "data/**/*.txt")
    /// * `expression` - Optional regex expression to search for
    /// * `line_width` - Number of bytes to display per line
    /// * `limit` - Maximum number of matches/lines per file (0 for unlimited)
    /// * `separator` - String to separate hex bytes
    /// * `show_offset` - Whether to display offset values
    /// * `parallel` - Whether to use parallel processing
    /// * `chunk_size` - Chunk size for parallel processing
    /// * `global_limit` - Global limit across all files (0 for unlimited)
    pub fn process_files_by_glob(
        &self,
        pattern: &str,
        expression: Option<&str>,
        line_width: usize,
        limit: usize,
        separator: &str,
        show_offset: bool,
        parallel: bool,
        chunk_size: usize,
        overlap_size: Option<usize>,
        global_limit: usize,
    ) -> Result<()> {
        let paths = glob(pattern)?;
        let mut total_processed = 0;

        for path_result in paths {
            let path = path_result?;

            // Skip directories
            if path.is_dir() {
                continue;
            }

            crate::output_context::write_line(&format!("=== Processing: {} ===", path.display()));

            let processed_count = self.process_single_file(
                &path,
                expression,
                line_width,
                limit,
                separator,
                show_offset,
                parallel,
                chunk_size,
                overlap_size,
            )?;

            total_processed += processed_count;

            // Check global limit
            if global_limit > 0 && total_processed >= global_limit {
                crate::output_context::write_line(&format!(
                    "=== Global limit of {} reached ===",
                    global_limit
                ));
                break;
            }
        }

        crate::output_context::write_line(&format!(
            "=== Total matches/lines processed: {} ===",
            total_processed
        ));
        Ok(())
    }

    /// Process a list of specific files
    ///
    /// # Arguments
    ///
    /// * `file_paths` - Vector of file paths to process
    /// * `expression` - Optional regex expression to search for
    /// * `line_width` - Number of bytes to display per line
    /// * `limit` - Maximum number of matches/lines per file (0 for unlimited)
    /// * `separator` - String to separate hex bytes
    /// * `show_offset` - Whether to display offset values
    /// * `parallel` - Whether to use parallel processing
    /// * `chunk_size` - Chunk size for parallel processing
    /// * `global_limit` - Global limit across all files (0 for unlimited)
    pub fn process_files_by_list(
        &self,
        file_paths: Vec<&str>,
        expression: Option<&str>,
        line_width: usize,
        limit: usize,
        separator: &str,
        show_offset: bool,
        parallel: bool,
        chunk_size: usize,
        overlap_size: Option<usize>,
        global_limit: usize,
    ) -> Result<()> {
        let mut total_processed = 0;

        for file_path in file_paths {
            let path = Path::new(file_path);

            // Skip if file doesn't exist or is a directory
            if !path.exists() {
                eprintln!("Warning: File {} does not exist, skipping", file_path);
                continue;
            }

            if path.is_dir() {
                eprintln!("Warning: {} is a directory, skipping", file_path);
                continue;
            }

            crate::output_context::write_line(&format!("=== Processing: {} ===", path.display()));

            let processed_count = self.process_single_file(
                path,
                expression,
                line_width,
                limit,
                separator,
                show_offset,
                parallel,
                chunk_size,
                overlap_size,
            )?;

            total_processed += processed_count;

            // Check global limit
            if global_limit > 0 && total_processed >= global_limit {
                crate::output_context::write_line(&format!(
                    "=== Global limit of {} reached ===",
                    global_limit
                ));
                break;
            }
        }

        crate::output_context::write_line(&format!(
            "=== Total matches/lines processed: {} ===",
            total_processed
        ));
        Ok(())
    }

    /// Process a single file and return the number of matches/lines processed
    fn process_single_file(
        &self,
        path: &Path,
        expression: Option<&str>,
        line_width: usize,
        limit: usize,
        separator: &str,
        show_offset: bool,
        parallel: bool,
        chunk_size: usize,
        explicit_overlap_size: Option<usize>,
    ) -> Result<usize> {
        if is_forensic_image_path(path)? {
            let mut processor = FileProcessor::new(self.config.clone());
            let mut progress = ProgressIndicator::disabled();

            if let Some(expr) = expression {
                let regex = RegexProcessor::compile_pattern(expr)?;
                let overlap_size = self.overlap_for_stream(expr, explicit_overlap_size);
                processor.process_stream_by_regex_from_path(
                    path,
                    &regex,
                    line_width,
                    limit,
                    separator,
                    show_offset,
                    overlap_size,
                    &mut progress,
                )?;
            } else {
                processor.process_file_stream_from_path(
                    path,
                    line_width,
                    limit,
                    separator,
                    show_offset,
                    &mut progress,
                )?;
            }

            return Ok(0);
        }

        let mut file = File::open(path)?;
        let file_size = file.metadata()?.len();

        if let Some(expr) = expression {
            // Regex search mode
            let regex = RegexProcessor::compile_pattern(expr)?;
            let matches_before = Self::count_matches_in_output();

            if parallel && file_size > chunk_size as u64 {
                let parallel_overlap =
                    self.overlap_for_parallel(expr, chunk_size, explicit_overlap_size);
                let mut progress = ProgressIndicator::disabled();

                ParallelProcessor::process_file_parallel(
                    &mut file,
                    &regex,
                    chunk_size,
                    line_width,
                    limit,
                    separator,
                    show_offset,
                    file_size,
                    parallel_overlap,
                    &mut progress,
                )?;
            } else {
                let stream_overlap = self.overlap_for_stream(expr, explicit_overlap_size);

                let mut processor = FileProcessor::new(self.config.clone());
                let mut progress = ProgressIndicator::disabled();
                processor.process_stream_by_regex(
                    &mut file,
                    &regex,
                    line_width,
                    limit,
                    separator,
                    show_offset,
                    stream_overlap,
                    &mut progress,
                )?;
            }

            let matches_after = Self::count_matches_in_output();
            Ok(matches_after - matches_before)
        } else {
            // Hex dump mode
            let lines_before = Self::count_lines_in_output();

            if parallel && file_size > chunk_size as u64 {
                let mut progress = ProgressIndicator::disabled();
                ParallelHexDump::process_file_parallel(
                    &mut file,
                    chunk_size,
                    line_width,
                    limit,
                    separator,
                    show_offset,
                    file_size,
                    &mut progress,
                )?;
            } else {
                let mut processor = FileProcessor::new(self.config.clone());
                let mut progress = ProgressIndicator::disabled();
                processor.process_file_stream(
                    &mut file,
                    line_width,
                    limit,
                    separator,
                    show_offset,
                    file_size,
                    &mut progress,
                )?;
            }

            let lines_after = Self::count_lines_in_output();
            Ok(lines_after - lines_before)
        }
    }

    /// Dummy function to count matches - in a real implementation,
    /// this would capture output and count actual matches
    fn count_matches_in_output() -> usize {
        // This is a simplified implementation
        // In practice, you'd want to capture stdout and count lines
        0
    }

    /// Dummy function to count lines - in a real implementation,
    /// this would capture output and count actual lines
    fn count_lines_in_output() -> usize {
        // This is a simplified implementation
        // In practice, you'd want to capture stdout and count lines
        0
    }

    /// Process multiple files in parallel
    ///
    /// This method processes multiple files concurrently using rayon
    pub fn process_files_parallel(
        &self,
        file_paths: Vec<&str>,
        expression: Option<&str>,
        line_width: usize,
        limit: usize,
        separator: &str,
        show_offset: bool,
        parallel_processing: bool,
        chunk_size: usize,
        overlap_size: Option<usize>,
    ) -> Result<()> {
        use rayon::prelude::*;

        let results: Vec<Result<()>> = file_paths
            .par_iter()
            .map(|file_path| {
                let path = Path::new(file_path);

                if !path.exists() || path.is_dir() {
                    return Ok(());
                }

                crate::output_context::write_line(&format!(
                    "=== Processing: {} ===",
                    path.display()
                ));

                self.process_single_file(
                    path,
                    expression,
                    line_width,
                    limit,
                    separator,
                    show_offset,
                    parallel_processing,
                    chunk_size,
                    overlap_size,
                )
                .map(|_| ())
            })
            .collect();

        // Check for any errors
        for result in results {
            result?;
        }

        Ok(())
    }

    fn overlap_for_stream(&self, expression: &str, explicit_overlap: Option<usize>) -> usize {
        let default_overlap = explicit_overlap.unwrap_or(self.config.buffer_padding);
        let overlap = RegexProcessor::overlap_for_expression(expression, default_overlap);
        Self::warn_for_large_overlap(overlap, default_overlap, explicit_overlap.is_some());
        overlap
    }

    fn overlap_for_parallel(
        &self,
        expression: &str,
        chunk_size: usize,
        explicit_overlap: Option<usize>,
    ) -> usize {
        let default_overlap =
            explicit_overlap.unwrap_or(self.config.buffer_padding.min(chunk_size / 10));
        let overlap = RegexProcessor::overlap_for_expression(expression, default_overlap);
        Self::warn_for_large_overlap(overlap, default_overlap, explicit_overlap.is_some());
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
}
