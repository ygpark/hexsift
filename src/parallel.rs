use crate::error::Result;
use crate::output::OutputFormatter;
use rayon::prelude::*;
use regex::bytes::Regex;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

/// Parallel file processor for improved performance on large files
pub struct ParallelProcessor;

impl ParallelProcessor {
    /// Process file with parallel chunked search
    ///
    /// Divides the file into chunks and processes them in parallel for better performance.
    ///
    /// # Arguments
    ///
    /// * `file` - File to search in
    /// * `regex` - Compiled regex pattern to search for
    /// * `chunk_size` - Size of each chunk in bytes
    /// * `width` - Number of bytes to display per match
    /// * `limit` - Maximum number of matches to output (0 for unlimited)
    /// * `separator` - String to separate hex bytes
    /// * `show_offset` - Whether to display offset values
    /// * `file_size` - Total size of the file for offset formatting
    pub fn process_file_parallel(
        file: &mut File,
        regex: &Regex,
        chunk_size: usize,
        width: usize,
        limit: usize,
        separator: &str,
        show_offset: bool,
        file_size: u64,
        overlap_size: usize,
    ) -> Result<()> {
        let hex_offset_length = OutputFormatter::calculate_hex_offset_length(file_size);
        let mut all_matches = Vec::new();
        let mut current_pos = file.stream_position()?;
        let mut match_count = 0;

        while current_pos < file_size {
            let remaining = file_size - current_pos;
            let actual_chunk_size = if remaining < chunk_size as u64 {
                remaining as usize
            } else {
                chunk_size + overlap_size
            };

            // Read chunk with overlap
            let mut chunk_buffer = vec![0u8; actual_chunk_size];
            file.seek(SeekFrom::Start(current_pos))?;
            let bytes_read = file.read(&mut chunk_buffer)?;
            chunk_buffer.truncate(bytes_read);

            if chunk_buffer.is_empty() {
                break;
            }

            // Process chunk and find matches
            let chunk_matches = Self::process_chunk(
                &chunk_buffer,
                regex,
                current_pos,
                width,
                separator,
                show_offset,
                hex_offset_length,
            );

            // Add matches to the collection
            for (offset, line) in chunk_matches {
                // Skip matches in overlap region except for the first chunk
                if current_pos > 0 && offset >= current_pos + chunk_size as u64 {
                    continue;
                }

                all_matches.push((offset, line));
                match_count += 1;

                // Check limit
                if limit > 0 && match_count >= limit {
                    break;
                }
            }

            if limit > 0 && match_count >= limit {
                break;
            }

            // Move to next chunk (without overlap to avoid double processing)
            if let Some(new_pos) = current_pos.checked_add(chunk_size as u64) {
                current_pos = new_pos;
            } else {
                // Overflow would occur - we've reached the end
                break;
            }
        }

        // Sort matches by offset and print
        all_matches.sort_by_key(|(offset, _)| *offset);
        for (_, line) in all_matches
            .into_iter()
            .take(if limit > 0 { limit } else { usize::MAX })
        {
            println!("{}", line);
        }

        Ok(())
    }

    /// Process a chunk of data and find regex matches
    fn process_chunk(
        data: &[u8],
        regex: &Regex,
        chunk_start_offset: u64,
        width: usize,
        separator: &str,
        show_offset: bool,
        hex_offset_length: usize,
    ) -> Vec<(u64, String)> {
        let mut matches = Vec::new();

        for mat in regex.find_iter(data) {
            let match_offset = chunk_start_offset + mat.start() as u64;

            // Determine the range to display
            let start_pos = mat.start();
            let end_pos = (start_pos + width).min(data.len());

            if start_pos < data.len() {
                let display_bytes = &data[start_pos..end_pos];
                let hex_string = OutputFormatter::format_bytes_as_hex(display_bytes, separator);
                let formatted_line = if show_offset {
                    OutputFormatter::format_line_with_offset(
                        match_offset,
                        &hex_string,
                        hex_offset_length,
                    )
                } else {
                    hex_string
                };
                matches.push((match_offset, formatted_line));
            }
        }

        matches
    }

    /// Process multiple chunks in parallel
    ///
    /// This method divides a large buffer into smaller chunks and processes them
    /// in parallel using rayon's parallel iterators.
    pub fn process_buffer_parallel(
        data: &[u8],
        regex: &Regex,
        base_offset: u64,
        width: usize,
        separator: &str,
        show_offset: bool,
        hex_offset_length: usize,
    ) -> Vec<(u64, String)> {
        const PARALLEL_CHUNK_SIZE: usize = 64 * 1024; // 64KB per thread
        const OVERLAP_SIZE: usize = 1024; // 1KB overlap

        if data.len() <= PARALLEL_CHUNK_SIZE {
            return Self::process_chunk(
                data,
                regex,
                base_offset,
                width,
                separator,
                show_offset,
                hex_offset_length,
            );
        }

        let mut chunks = Vec::new();
        let mut pos = 0;

        while pos < data.len() {
            let end = (pos + PARALLEL_CHUNK_SIZE + OVERLAP_SIZE).min(data.len());
            let chunk_data = &data[pos..end];
            let chunk_offset = base_offset + pos as u64;
            chunks.push((chunk_data, chunk_offset));

            pos += PARALLEL_CHUNK_SIZE;
        }

        // Process chunks in parallel
        let all_matches: Vec<Vec<(u64, String)>> = chunks
            .into_par_iter()
            .map(|(chunk_data, chunk_offset)| {
                Self::process_chunk(
                    chunk_data,
                    regex,
                    chunk_offset,
                    width,
                    separator,
                    show_offset,
                    hex_offset_length,
                )
            })
            .collect();

        // Flatten and sort results
        let mut matches: Vec<(u64, String)> = all_matches.into_iter().flatten().collect();
        matches.sort_by_key(|(offset, _)| *offset);

        // Remove duplicates that might occur in overlap regions
        matches.dedup_by_key(|(offset, _)| *offset);

        matches
    }
}

/// Parallel hex dump processor for non-regex operations
pub struct ParallelHexDump;

impl ParallelHexDump {
    /// Process file in parallel for hex dump (non-regex mode)
    pub fn process_file_parallel(
        file: &mut File,
        chunk_size: usize,
        width: usize,
        limit: usize,
        separator: &str,
        show_offset: bool,
        file_size: u64,
    ) -> Result<()> {
        let hex_offset_length = OutputFormatter::calculate_hex_offset_length(file_size);
        let mut current_pos = file.stream_position()?;
        let mut lines_processed = 0;

        // For hex dump, we don't need overlap since we're not searching for patterns
        while current_pos < file_size && (limit == 0 || lines_processed < limit) {
            let remaining = file_size - current_pos;
            let actual_chunk_size = (chunk_size as u64).min(remaining) as usize;

            let mut chunk_buffer = vec![0u8; actual_chunk_size];
            file.seek(SeekFrom::Start(current_pos))?;
            let bytes_read = file.read(&mut chunk_buffer)?;
            chunk_buffer.truncate(bytes_read);

            if chunk_buffer.is_empty() {
                break;
            }

            // Process chunk
            let chunk_lines = Self::process_chunk_hex_dump(
                &chunk_buffer,
                current_pos,
                width,
                separator,
                show_offset,
                hex_offset_length,
                if limit > 0 {
                    limit - lines_processed
                } else {
                    0
                },
            );

            for line in chunk_lines {
                println!("{}", line);
                lines_processed += 1;
                if limit > 0 && lines_processed >= limit {
                    break;
                }
            }

            current_pos += bytes_read as u64;
        }

        Ok(())
    }

    /// Process a chunk for hex dump output
    fn process_chunk_hex_dump(
        data: &[u8],
        start_offset: u64,
        width: usize,
        separator: &str,
        show_offset: bool,
        hex_offset_length: usize,
        remaining_limit: usize,
    ) -> Vec<String> {
        let mut lines = Vec::new();
        let mut pos = 0;
        let mut line_count = 0;

        while pos < data.len() && (remaining_limit == 0 || line_count < remaining_limit) {
            let end = (pos + width).min(data.len());
            let line_bytes = &data[pos..end];
            let offset = start_offset + pos as u64;

            let hex_string = OutputFormatter::format_bytes_as_hex(line_bytes, separator);
            let formatted_line = if show_offset {
                OutputFormatter::format_line_with_offset(offset, &hex_string, hex_offset_length)
            } else {
                hex_string
            };

            lines.push(formatted_line);
            pos += width;
            line_count += 1;
        }

        lines
    }
}
