use crate::buffer_manager::BufferManager;
use crate::config::Config;
use crate::error::Result;
use crate::forensic_image::{is_forensic_image_path, ForensicImageReader};
use crate::output::OutputFormatter;
use crate::progress::ProgressIndicator;
use regex::bytes::Regex;
use std::fs::File;
use std::io::{Read, Seek};
use std::path::Path;

/// File processor for handling binary file searching and hex dump operations
pub struct FileProcessor {
    config: Config,
    buffer_manager: BufferManager,
}

impl FileProcessor {
    /// Create a new FileProcessor with the given configuration
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration settings for buffer sizes and limits
    pub fn new(config: Config) -> Self {
        let buffer_size = config.buffer_size;
        let max_extra_size = config.max_line_width.max(1024); // At least 1KB for extra buffer
        let buffer_manager = BufferManager::new(buffer_size, max_extra_size);

        Self {
            config,
            buffer_manager,
        }
    }

    /// Process file without regex - simple hex dump
    ///
    /// Reads a file and outputs its contents in hexadecimal format.
    /// Automatically detects forensic image files (E01, VMDK) and processes them using appropriate libraries.
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the file to read from
    /// * `width` - Number of bytes to display per line
    /// * `limit` - Maximum number of lines to output (0 for unlimited)
    /// * `separator` - String to separate hex bytes
    /// * `show_offset` - Whether to display offset values
    /// * `progress` - Progress indicator to update during processing
    pub fn process_file_stream_from_path<P: AsRef<Path>>(
        &mut self,
        file_path: P,
        width: usize,
        limit: usize,
        separator: &str,
        show_offset: bool,
        progress: &mut ProgressIndicator,
    ) -> Result<()> {
        let file_path = file_path.as_ref();

        if is_forensic_image_path(&file_path)? {
            // Process forensic image file (E01, VMDK)
            let mut forensic_reader = ForensicImageReader::new(&file_path)?;
            let file_size = forensic_reader.size();
            self.process_reader_stream(
                &mut forensic_reader,
                width,
                limit,
                separator,
                show_offset,
                file_size,
                progress,
            )
        } else {
            // Process regular file
            let mut file = File::open(&file_path)?;
            let file_size = file.metadata()?.len();
            self.process_reader_stream(
                &mut file,
                width,
                limit,
                separator,
                show_offset,
                file_size,
                progress,
            )
        }
    }

    /// Process file without regex - simple hex dump
    ///
    /// Reads a file and outputs its contents in hexadecimal format.
    ///
    /// # Arguments
    ///
    /// * `file` - File to read from
    /// * `width` - Number of bytes to display per line
    /// * `limit` - Maximum number of lines to output (0 for unlimited)
    /// * `separator` - String to separate hex bytes
    /// * `show_offset` - Whether to display offset values
    /// * `file_size` - Total size of the file for offset formatting
    /// * `progress` - Progress indicator to update during processing
    pub fn process_file_stream(
        &mut self,
        file: &mut File,
        width: usize,
        limit: usize,
        separator: &str,
        show_offset: bool,
        file_size: u64,
        progress: &mut ProgressIndicator,
    ) -> Result<()> {
        self.process_reader_stream(
            file,
            width,
            limit,
            separator,
            show_offset,
            file_size,
            progress,
        )
    }

    /// Generic stream processing function that works with any Read + Seek reader
    fn process_reader_stream<R: Read + Seek>(
        &mut self,
        reader: &mut R,
        width: usize,
        limit: usize,
        separator: &str,
        show_offset: bool,
        file_size: u64,
        progress: &mut ProgressIndicator,
    ) -> Result<()> {
        let mut pos = reader.stream_position()?;
        let mut line = 0;
        let hex_offset_length = OutputFormatter::calculate_hex_offset_length(file_size);

        // Get a reusable buffer of the right size
        let buffer = self.buffer_manager.get_extra_buffer(width);

        loop {
            let bytes_read = reader.read(&mut buffer[..width])?;
            if bytes_read == 0 {
                break;
            }

            line += 1;

            let hex_string = OutputFormatter::format_bytes_as_hex(&buffer[..bytes_read], separator);
            OutputFormatter::print_line_with_silent(
                pos,
                &hex_string,
                show_offset,
                hex_offset_length,
                progress.is_silent(),
            );

            pos += bytes_read as u64;

            // Update progress
            progress.update(bytes_read as u64);

            // Check line limit
            if limit > 0 && line >= limit {
                break;
            }
        }

        progress.finish();
        Ok(())
    }

    /// Process file with regex pattern matching from file path
    ///
    /// Searches a file for regex pattern matches and outputs matching regions.
    /// Automatically detects forensic image files (E01, VMDK) and processes them using appropriate libraries.
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the file to search in
    /// * `regex` - Compiled regex pattern to search for
    /// * `width` - Number of bytes to display per match
    /// * `limit` - Maximum number of matches to output (0 for unlimited)
    /// * `separator` - String to separate hex bytes
    /// * `show_offset` - Whether to display offset values
    /// * `progress` - Progress indicator to update during processing
    pub fn process_stream_by_regex_from_path<P: AsRef<Path>>(
        &mut self,
        file_path: P,
        regex: &Regex,
        width: usize,
        limit: usize,
        separator: &str,
        show_offset: bool,
        progress: &mut ProgressIndicator,
    ) -> Result<()> {
        let file_path = file_path.as_ref();

        if is_forensic_image_path(&file_path)? {
            // Process forensic image file (E01, VMDK)
            let mut forensic_reader = ForensicImageReader::new(&file_path)?;
            self.process_reader_by_regex(
                &mut forensic_reader,
                regex,
                width,
                limit,
                separator,
                show_offset,
                progress,
            )
        } else {
            // Process regular file
            let mut file = File::open(&file_path)?;
            self.process_reader_by_regex(
                &mut file,
                regex,
                width,
                limit,
                separator,
                show_offset,
                progress,
            )
        }
    }

    /// Process file with regex pattern matching
    ///
    /// Searches a file for regex pattern matches and outputs matching regions.
    ///
    /// # Arguments
    ///
    /// * `file` - File to search in
    /// * `regex` - Compiled regex pattern to search for
    /// * `width` - Number of bytes to display per match
    /// * `limit` - Maximum number of matches to output (0 for unlimited)
    /// * `separator` - String to separate hex bytes
    /// * `show_offset` - Whether to display offset values
    pub fn process_stream_by_regex(
        &mut self,
        file: &mut File,
        regex: &Regex,
        width: usize,
        limit: usize,
        separator: &str,
        show_offset: bool,
        progress: &mut ProgressIndicator,
    ) -> Result<()> {
        self.process_reader_by_regex(file, regex, width, limit, separator, show_offset, progress)
    }

    /// Generic regex processing function that works with any Read + Seek reader
    fn process_reader_by_regex<R: Read + Seek>(
        &mut self,
        reader: &mut R,
        regex: &Regex,
        width: usize,
        limit: usize,
        separator: &str,
        show_offset: bool,
        progress: &mut ProgressIndicator,
    ) -> Result<()> {
        let buffer_padding = self.config.buffer_padding;

        let mut line = 0;
        let mut last_hit_pos: i64 = -1;
        let mut absolute_pos = reader.stream_position()?;
        let mut carry = Vec::new();

        // For EWF files, we need to get size differently
        // For now, we'll use a large default for generic readers
        const FORENSIC_IMAGE_DEFAULT_SIZE: u64 = 1024 * 1024 * 1024 * 1024; // 1TB default
        let file_size = FORENSIC_IMAGE_DEFAULT_SIZE;
        let hex_offset_length = OutputFormatter::calculate_hex_offset_length(file_size);

        loop {
            let bytes_read = self.buffer_manager.read_into_main(reader)?;

            if bytes_read == 0 {
                break;
            }

            // Update progress
            progress.update(bytes_read as u64);

            let current_data = self.buffer_manager.get_main_slice(0, bytes_read);
            let base_offset = absolute_pos.saturating_sub(carry.len() as u64);
            let mut search_buffer = Vec::with_capacity(carry.len() + bytes_read);
            search_buffer.extend_from_slice(&carry);
            search_buffer.extend_from_slice(current_data);

            for mat in regex.find_iter(&search_buffer) {
                let match_start = mat.start();
                let new_hit_pos = base_offset + match_start as u64;

                if new_hit_pos as i64 <= last_hit_pos {
                    continue;
                }

                line += 1;

                let display_end = match_start.saturating_add(width).min(search_buffer.len());
                let display_data = &search_buffer[match_start..display_end];
                let hex_string = OutputFormatter::format_bytes_as_hex(display_data, separator);
                let match_info = regex.find(display_data).map(|matched| matched.len());

                // Calculate match position within the displayed hex string
                let match_byte_pos = Some(0);
                let match_byte_len = match_info.map(|len| std::cmp::min(len, width));

                OutputFormatter::print_line_with_match_highlight_silent(
                    new_hit_pos,
                    &hex_string,
                    show_offset,
                    hex_offset_length,
                    crate::color_context::get_color_choice(),
                    match_byte_pos,
                    match_byte_len,
                    progress.is_silent(),
                );
                last_hit_pos = new_hit_pos as i64;

                // Check line limit
                if limit > 0 && line >= limit {
                    return Ok(());
                }
            }

            let carry_len = buffer_padding.min(search_buffer.len());
            carry.clear();
            carry.extend_from_slice(&search_buffer[search_buffer.len() - carry_len..]);
            absolute_pos += bytes_read as u64;
        }

        progress.finish();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{SeekFrom, Write};
    use tempfile::NamedTempFile;

    #[test]
    fn test_file_processor_creation() {
        let config = Config::default();
        let processor = FileProcessor::new(config);
        assert_eq!(processor.config.buffer_size, 4 * 1024 * 1024);
    }

    #[test]
    fn test_process_file_stream() -> Result<()> {
        let config = Config::default();
        let mut processor = FileProcessor::new(config);

        // Create a temporary file with test data
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"Hello World!").unwrap();
        temp_file.seek(SeekFrom::Start(0)).unwrap();

        let mut file = temp_file.reopen().unwrap();
        let file_size = file.metadata()?.len();

        // This would normally print, but in tests we just verify it doesn't error
        let mut progress = ProgressIndicator::disabled();
        let result =
            processor.process_file_stream(&mut file, 16, 1, " ", false, file_size, &mut progress);
        assert!(result.is_ok());

        Ok(())
    }
}
