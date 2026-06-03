use crate::error::{BingrepError, Result};
use regex::bytes::Regex;

/// Processor for handling regular expression patterns with hexadecimal escape sequences
pub struct RegexProcessor;

impl RegexProcessor {
    /// Process and compile a regex pattern that may contain hex escapes and quantifiers
    ///
    /// # Arguments
    ///
    /// * `expression` - A regex pattern string that may contain \xHH hex escape sequences
    ///
    /// # Returns
    ///
    /// * `Result<Regex>` - Compiled regex pattern or error
    ///
    /// # Examples
    ///
    /// ```
    /// use hexsift::RegexProcessor;
    /// let regex = RegexProcessor::compile_pattern("\\x00\\x01\\x02").unwrap();
    /// let regex_with_quantifier = RegexProcessor::compile_pattern("\\x58{2,3}").unwrap();
    /// ```
    pub fn compile_pattern(expression: &str) -> Result<Regex> {
        let pattern = if expression.contains("\\x") && !Self::has_regex_metacharacters(expression) {
            // Simple \xHH pattern - convert to binary then escape for regex
            let binary_pattern = Self::parse_hex_pattern(expression)?;
            if binary_pattern.is_empty() {
                return Err(BingrepError::InvalidPattern(
                    "No valid hex pattern found".to_string(),
                ));
            }
            Self::escape_bytes_for_regex(&binary_pattern)
        } else {
            // Pattern with regex metacharacters - convert only \xHH while preserving quantifiers
            Self::convert_hex_escapes_in_pattern(expression)?
        };

        Regex::new(&pattern).map_err(BingrepError::from)
    }

    /// Parse \xHH sequences into bytes
    ///
    /// Extracts hexadecimal byte values from a pattern string containing \xHH sequences.
    /// Non-hex characters are ignored.
    pub fn parse_hex_pattern(pattern: &str) -> Result<Vec<u8>> {
        let mut result = Vec::new();
        let mut chars = pattern.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '\\' {
                if let Some(&next_ch) = chars.peek() {
                    if next_ch == 'x' || next_ch == 'X' {
                        chars.next(); // consume 'x' or 'X'

                        // Parse next 2 characters as hex
                        let hex1 = chars.next();
                        let hex2 = chars.next();

                        match (hex1, hex2) {
                            (Some(h1), Some(h2)) => {
                                let hex_str = format!("{}{}", h1, h2);
                                match u8::from_str_radix(&hex_str, 16) {
                                    Ok(byte) => result.push(byte),
                                    Err(_) => {
                                        return Err(BingrepError::InvalidPattern(format!(
                                            "Invalid hex sequence: \\x{}",
                                            hex_str
                                        )));
                                    }
                                }
                            }
                            (Some(h1), None) => {
                                return Err(BingrepError::InvalidPattern(format!(
                                    "Incomplete hex sequence: \\x{}",
                                    h1
                                )));
                            }
                            (None, _) => {
                                return Err(BingrepError::InvalidPattern(
                                    "Incomplete hex sequence: \\x".to_string(),
                                ));
                            }
                        }
                    }
                }
            }
            // Ignore non-hex characters for simple patterns
        }

        Ok(result)
    }

    /// Escape bytes for regex use
    ///
    /// Converts a byte array into a regex-compatible string that disables Unicode mode
    /// to ensure literal byte matching without UTF-8 interpretation.
    pub fn escape_bytes_for_regex(bytes: &[u8]) -> String {
        let escaped = bytes
            .iter()
            .map(|&b| {
                // Use \xHH format for literal bytes
                format!("\\x{:02x}", b)
            })
            .collect::<String>();

        // Disable Unicode mode to force literal byte matching
        format!("(?-u){}", escaped)
    }

    /// Check if pattern contains regex metacharacters
    ///
    /// Returns true if the pattern contains any regex quantifiers or special characters
    fn has_regex_metacharacters(pattern: &str) -> bool {
        pattern.chars().any(|c| {
            matches!(
                c,
                '+' | '*' | '?' | '(' | ')' | '[' | ']' | '{' | '}' | '|' | '^' | '$'
            )
        })
    }

    /// Convert hex escapes in pattern while preserving other regex syntax
    ///
    /// Processes a regex pattern to convert \xHH sequences while maintaining
    /// other regex metacharacters and syntax intact.
    fn convert_hex_escapes_in_pattern(pattern: &str) -> Result<String> {
        let mut result = String::new();
        let mut chars = pattern.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '\\' {
                if let Some(&next_ch) = chars.peek() {
                    if next_ch == 'x' || next_ch == 'X' {
                        chars.next(); // consume 'x'

                        // Parse next 2 characters as hex
                        let hex1 = chars.next();
                        let hex2 = chars.next();

                        match (hex1, hex2) {
                            (Some(h1), Some(h2)) => {
                                let hex_str = format!("{}{}", h1, h2);
                                match u8::from_str_radix(&hex_str, 16) {
                                    Ok(byte) => {
                                        // Convert byte to regex form
                                        result.push_str(&format!("\\x{:02x}", byte));
                                    }
                                    Err(_) => {
                                        return Err(BingrepError::InvalidPattern(format!(
                                            "Invalid hex sequence in regex pattern: \\x{}",
                                            hex_str
                                        )));
                                    }
                                }
                            }
                            (Some(h1), None) => {
                                return Err(BingrepError::InvalidPattern(format!(
                                    "Incomplete hex sequence in regex pattern: \\x{}",
                                    h1
                                )));
                            }
                            (None, _) => {
                                return Err(BingrepError::InvalidPattern(
                                    "Incomplete hex sequence in regex pattern: \\x".to_string(),
                                ));
                            }
                        }
                    } else {
                        result.push('\\');
                    }
                } else {
                    result.push('\\');
                }
            } else {
                result.push(ch);
            }
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hex_pattern_basic() {
        let pattern = "\\x00\\x01\\x02\\xFF";
        let result = RegexProcessor::parse_hex_pattern(pattern).unwrap();
        assert_eq!(result, vec![0x00, 0x01, 0x02, 0xFF]);
    }

    #[test]
    fn test_parse_hex_pattern_mixed_case() {
        let pattern = "\\x0a\\x0B\\xfF\\xAA";
        let result = RegexProcessor::parse_hex_pattern(pattern).unwrap();
        assert_eq!(result, vec![0x0a, 0x0B, 0xFF, 0xAA]);
    }

    #[test]
    fn test_parse_hex_pattern_with_text() {
        let pattern = "prefix\\x41\\x42\\x43suffix";
        let result = RegexProcessor::parse_hex_pattern(pattern).unwrap();
        assert_eq!(result, vec![0x41, 0x42, 0x43]);
    }

    #[test]
    fn test_parse_hex_pattern_invalid() {
        let pattern = "\\xZZ";
        let result = RegexProcessor::parse_hex_pattern(pattern);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_hex_pattern_incomplete() {
        let pattern = "\\x4";
        let result = RegexProcessor::parse_hex_pattern(pattern);
        assert!(result.is_err());
    }

    #[test]
    fn test_escape_bytes_for_regex_basic() {
        let bytes = vec![0x00, 0x01, 0x41, 0xFF];
        let result = RegexProcessor::escape_bytes_for_regex(&bytes);
        assert_eq!(result, "(?-u)\\x00\\x01\\x41\\xff");
    }

    #[test]
    fn test_compile_pattern_simple() {
        let result = RegexProcessor::compile_pattern("\\x00\\x01\\x02");
        assert!(result.is_ok());
    }

    #[test]
    fn test_compile_pattern_with_quantifier() {
        let result = RegexProcessor::compile_pattern("\\x58{2,3}");
        assert!(result.is_ok());
    }

    #[test]
    fn test_has_regex_metacharacters() {
        assert!(RegexProcessor::has_regex_metacharacters("\\x58{2}"));
        assert!(RegexProcessor::has_regex_metacharacters("\\x58+"));
        assert!(!RegexProcessor::has_regex_metacharacters("\\x58\\x59"));
    }

    #[test]
    fn test_utf8_pattern_fix() {
        // Test case for UTF-8 interpretation issue fix
        let pattern = "\\x00\\x00\\xba";
        let regex = RegexProcessor::compile_pattern(pattern).unwrap();

        // Test data: exact pattern should match
        let test_data1 = vec![0x00, 0x00, 0xba, 0xAA];
        // Test data: UTF-8 encoded version should NOT match
        let test_data2 = vec![0x00, 0x00, 0xc2, 0xba, 0xAA];

        assert_eq!(
            RegexProcessor::parse_hex_pattern(pattern).unwrap(),
            vec![0x00, 0x00, 0xba]
        );
        assert!(regex.is_match(&test_data1), "Exact pattern should match");
        assert!(
            !regex.is_match(&test_data2),
            "UTF-8 encoded pattern should not match"
        );
    }

    #[cfg(test)]
    #[test]
    fn test_utf8_pattern_with_file() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        // Create temporary files with UTF-8 test data
        let mut temp_file1 = NamedTempFile::new().unwrap();
        let mut temp_file2 = NamedTempFile::new().unwrap();

        // Write exact pattern and UTF-8 pattern to separate files
        temp_file1.write_all(&[0x00, 0x00, 0xba, 0xAA]).unwrap();
        temp_file2
            .write_all(&[0x00, 0x00, 0xc2, 0xba, 0xAA])
            .unwrap();

        let pattern = "\\x00\\x00\\xba";
        let regex = RegexProcessor::compile_pattern(pattern).unwrap();

        // Read files and test pattern matching
        let data1 = std::fs::read(temp_file1.path()).unwrap();
        let data2 = std::fs::read(temp_file2.path()).unwrap();

        assert!(
            regex.is_match(&data1),
            "File with exact pattern should match"
        );
        assert!(
            !regex.is_match(&data2),
            "File with UTF-8 pattern should not match"
        );

        // Files are automatically deleted when NamedTempFile goes out of scope
    }
}
