use memchr::memchr_iter;
use std::collections::HashMap;
use std::io::{self, BufRead};
use std::sync::OnceLock;

/// Common trait for fast GFF record types that provide lightweight access to parsed fields
///
/// This trait focuses purely on field access without expensive conversions.
/// All implementations provide lazy attribute parsing and zero-copy string access where possible.
#[allow(dead_code)]
pub trait GffRecord {
    /// Get the sequence ID
    fn seqid(&self) -> &str;

    /// Get the source
    fn source(&self) -> &str;

    /// Get the feature type
    fn feature_type(&self) -> &str;

    /// Get the start position
    fn start(&self) -> u32;

    /// Get the end position  
    fn end(&self) -> u32;

    /// Get the score if available
    fn score(&self) -> Option<f32>;

    /// Get the strand
    fn strand(&self) -> &str;

    /// Get the phase
    fn phase(&self) -> &str;

    /// Get the raw attributes string
    fn attributes_str(&self) -> &str;

    /// Get a specific attribute value by key
    fn get_attribute(&self, key: &str) -> Option<&str>;

    /// Get all parsed attributes (lazy parsing)
    fn attributes(&self) -> &HashMap<String, String>;
}

/// Unified Record wrapper that can hold either fast record type
/// This provides a common interface without costly data conversion
#[allow(dead_code)]
#[derive(Debug)]
pub enum Record {
    Fast(FastRecordOwned),
    Simd(SIMDRecord),
}

#[allow(dead_code)]
impl Record {
    /// Create a Record from a FastRecordOwned
    pub fn from_fast(record: FastRecordOwned) -> Self {
        Self::Fast(record)
    }

    /// Create a Record from a SIMDRecord
    pub fn from_simd(record: SIMDRecord) -> Self {
        Self::Simd(record)
    }

    /// Parse a line using the fast parser
    pub fn parse_fast(line: &str) -> io::Result<Self> {
        FastRecordOwned::parse_line(line).map(Self::Fast)
    }

    /// Parse a line using the SIMD parser
    pub fn parse_simd(line: &str) -> io::Result<Self> {
        SIMDRecord::parse_line_simd(line).map(Self::Simd)
    }

    /// Get the underlying fast record if it is one
    pub fn as_fast(&self) -> Option<&FastRecordOwned> {
        match self {
            Self::Fast(record) => Some(record),
            Self::Simd(_) => None,
        }
    }

    /// Get the underlying SIMD record if it is one
    pub fn as_simd(&self) -> Option<&SIMDRecord> {
        match self {
            Self::Fast(_) => None,
            Self::Simd(record) => Some(record),
        }
    }

    /// Check if this is a fast record
    pub fn is_fast(&self) -> bool {
        matches!(self, Self::Fast(_))
    }

    /// Check if this is a SIMD record
    pub fn is_simd(&self) -> bool {
        matches!(self, Self::Simd(_))
    }
}

impl GffRecord for Record {
    fn seqid(&self) -> &str {
        match self {
            Self::Fast(record) => record.seqid(),
            Self::Simd(record) => record.seqid(),
        }
    }

    fn source(&self) -> &str {
        match self {
            Self::Fast(record) => record.source(),
            Self::Simd(record) => record.source(),
        }
    }

    fn feature_type(&self) -> &str {
        match self {
            Self::Fast(record) => record.feature_type(),
            Self::Simd(record) => record.feature_type(),
        }
    }

    fn start(&self) -> u32 {
        match self {
            Self::Fast(record) => record.start(),
            Self::Simd(record) => record.start(),
        }
    }

    fn end(&self) -> u32 {
        match self {
            Self::Fast(record) => record.end(),
            Self::Simd(record) => record.end(),
        }
    }

    fn score(&self) -> Option<f32> {
        match self {
            Self::Fast(record) => record.score(),
            Self::Simd(record) => record.score(),
        }
    }

    fn strand(&self) -> &str {
        match self {
            Self::Fast(record) => record.strand(),
            Self::Simd(record) => record.strand(),
        }
    }

    fn phase(&self) -> &str {
        match self {
            Self::Fast(record) => record.phase(),
            Self::Simd(record) => record.phase(),
        }
    }

    fn attributes_str(&self) -> &str {
        match self {
            Self::Fast(record) => record.attributes_str(),
            Self::Simd(record) => record.attributes_str(),
        }
    }

    fn get_attribute(&self, key: &str) -> Option<&str> {
        match self {
            Self::Fast(record) => record.get_attribute(key),
            Self::Simd(record) => record.get_attribute(key),
        }
    }

    fn attributes(&self) -> &HashMap<String, String> {
        match self {
            Self::Fast(record) => record.attributes(),
            Self::Simd(record) => record.attributes(),
        }
    }
}

/// A fast, zero-copy GFF record that defers parsing until needed
#[allow(dead_code)]
#[derive(Debug)]
pub struct FastRecord<'a> {
    pub seqid: &'a str,
    pub source: &'a str,
    pub ty: &'a str,
    pub start: &'a str,
    pub end: &'a str,
    pub score: &'a str,
    pub strand: &'a str,
    pub phase: &'a str,
    pub attributes: &'a str,
}

#[allow(dead_code)]
impl<'a> FastRecord<'a> {
    /// Parse a line into fields without allocating
    pub fn parse(line: &'a str) -> Option<Self> {
        let mut fields = line.splitn(9, '\t');

        Some(Self {
            seqid: fields.next()?,
            source: fields.next()?,
            ty: fields.next()?,
            start: fields.next()?,
            end: fields.next()?,
            score: fields.next()?,
            strand: fields.next()?,
            phase: fields.next()?,
            attributes: fields.next()?,
        })
    }

    /// Parse start position only when needed
    pub fn start_position(&self) -> io::Result<u32> {
        self.start
            .parse()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    /// Parse end position only when needed
    pub fn end_position(&self) -> io::Result<u32> {
        self.end
            .parse()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    /// Parse score only when needed
    pub fn score_f32(&self) -> Option<f32> {
        if self.score == "." {
            None
        } else {
            self.score.parse().ok()
        }
    }
}

/// Fast record that owns its data with lazy attribute parsing
#[derive(Debug)]
pub struct FastRecordOwned {
    pub seqid: String,
    pub source: String,
    pub ty: String,
    pub start: u32,
    pub end: u32,
    pub score: Option<f32>,
    pub strand: String,
    pub phase: String,
    pub attributes: String,
    // Lazy-loaded attributes cache
    parsed_attributes: OnceLock<HashMap<String, String>>,
}

impl GffRecord for FastRecordOwned {
    fn seqid(&self) -> &str {
        &self.seqid
    }

    fn source(&self) -> &str {
        &self.source
    }

    fn feature_type(&self) -> &str {
        &self.ty
    }

    fn start(&self) -> u32 {
        self.start
    }

    fn end(&self) -> u32 {
        self.end
    }

    fn score(&self) -> Option<f32> {
        self.score
    }

    fn strand(&self) -> &str {
        &self.strand
    }

    fn phase(&self) -> &str {
        &self.phase
    }

    fn attributes_str(&self) -> &str {
        &self.attributes
    }

    fn get_attribute(&self, key: &str) -> Option<&str> {
        self.get_attribute(key)
    }

    fn attributes(&self) -> &HashMap<String, String> {
        self.attributes()
    }
}

impl FastRecordOwned {
    /// Parse a line into owned record with minimal allocations
    pub fn parse_line(line: &str) -> io::Result<Self> {
        let mut fields = line.splitn(9, '\t');

        let seqid = fields
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing seqid"))?
            .to_string();
        let source = fields
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing source"))?
            .to_string();
        let ty = fields
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing type"))?
            .to_string();

        let start_str = fields
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing start"))?;
        let start = start_str.parse().map_err(|e| {
            io::Error::new(io::ErrorKind::InvalidData, format!("invalid start: {}", e))
        })?;

        let end_str = fields
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing end"))?;
        let end = end_str.parse().map_err(|e| {
            io::Error::new(io::ErrorKind::InvalidData, format!("invalid end: {}", e))
        })?;

        let score_str = fields
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing score"))?;
        let score = if score_str == "." {
            None
        } else {
            Some(score_str.parse().map_err(|e| {
                io::Error::new(io::ErrorKind::InvalidData, format!("invalid score: {}", e))
            })?)
        };

        let strand = fields
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing strand"))?
            .to_string();
        let phase = fields
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing phase"))?
            .to_string();
        let attributes = fields.next().unwrap_or("").to_string();

        Ok(Self {
            seqid,
            source,
            ty,
            start,
            end,
            score,
            strand,
            phase,
            attributes,
            parsed_attributes: OnceLock::new(),
        })
    }

    /// Get an attribute value by key, parsing attributes lazily
    pub fn get_attribute(&self, key: &str) -> Option<&str> {
        let parsed = self
            .parsed_attributes
            .get_or_init(|| self.parse_attributes());
        parsed.get(key).map(|s| s.as_str())
    }

    /// Parse all attributes into a HashMap (called lazily)
    fn parse_attributes(&self) -> HashMap<String, String> {
        let mut attrs = HashMap::new();

        if self.attributes.is_empty() || self.attributes == "." {
            return attrs;
        }

        // Split by semicolon, then by equals
        for pair in self.attributes.split(';') {
            if let Some((key, value)) = pair.split_once('=') {
                attrs.insert(key.to_string(), value.to_string());
            }
        }

        attrs
    }

    /// Get all parsed attributes as a reference
    pub fn attributes(&self) -> &HashMap<String, String> {
        self.parsed_attributes
            .get_or_init(|| self.parse_attributes())
    }
}

/// Zero-copy record that borrows from a buffer
#[allow(dead_code)]
#[derive(Debug)]
pub struct ZeroCopyRecord<'a> {
    pub seqid: &'a str,
    pub source: &'a str,
    pub ty: &'a str,
    pub start_str: &'a str,
    pub end_str: &'a str,
    pub score_str: &'a str,
    pub strand: &'a str,
    pub phase: &'a str,
    pub attributes: &'a str,
}

#[allow(dead_code)]
impl<'a> ZeroCopyRecord<'a> {
    /// Parse a line into zero-copy fields
    pub fn parse(line: &'a str) -> Option<Self> {
        let mut fields = line.splitn(9, '\t');

        Some(Self {
            seqid: fields.next()?,
            source: fields.next()?,
            ty: fields.next()?,
            start_str: fields.next()?,
            end_str: fields.next()?,
            score_str: fields.next()?,
            strand: fields.next()?,
            phase: fields.next()?,
            attributes: fields.next().unwrap_or(""),
        })
    }

    /// Parse start position on demand
    pub fn start(&self) -> io::Result<u32> {
        self.start_str
            .parse()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    /// Parse end position on demand
    pub fn end(&self) -> io::Result<u32> {
        self.end_str
            .parse()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    /// Parse score on demand
    pub fn score(&self) -> Option<f32> {
        if self.score_str == "." {
            None
        } else {
            self.score_str.parse().ok()
        }
    }

    /// Get attribute value by key (zero-allocation)
    pub fn get_attribute(&self, key: &str) -> Option<&str> {
        if self.attributes.is_empty() || self.attributes == "." {
            return None;
        }

        for pair in self.attributes.split(';') {
            if let Some((k, v)) = pair.split_once('=') {
                if k == key {
                    return Some(v);
                }
            }
        }
        None
    }
}

/// SIMD-optimized record using memchr for field splitting
#[derive(Debug)]
pub struct SIMDRecord {
    pub seqid: String,
    pub source: String,
    pub ty: String,
    pub start: u32,
    pub end: u32,
    pub score: Option<f32>,
    pub strand: String,
    pub phase: String,
    pub attributes: String,
    parsed_attributes: OnceLock<HashMap<String, String>>,
}

impl GffRecord for SIMDRecord {
    fn seqid(&self) -> &str {
        &self.seqid
    }

    fn source(&self) -> &str {
        &self.source
    }

    fn feature_type(&self) -> &str {
        &self.ty
    }

    fn start(&self) -> u32 {
        self.start
    }

    fn end(&self) -> u32 {
        self.end
    }

    fn score(&self) -> Option<f32> {
        self.score
    }

    fn strand(&self) -> &str {
        &self.strand
    }

    fn phase(&self) -> &str {
        &self.phase
    }

    fn attributes_str(&self) -> &str {
        &self.attributes
    }

    fn get_attribute(&self, key: &str) -> Option<&str> {
        self.get_attribute(key)
    }

    fn attributes(&self) -> &HashMap<String, String> {
        self.attributes()
    }
}

impl SIMDRecord {
    /// Parse line using SIMD-optimized field splitting
    pub fn parse_line_simd(line: &str) -> io::Result<Self> {
        let line_bytes = line.as_bytes();
        let mut field_starts = Vec::with_capacity(9);
        field_starts.push(0);

        // Use SIMD to find all tab positions at once
        for tab_pos in memchr_iter(b'\t', line_bytes) {
            field_starts.push(tab_pos + 1);
            if field_starts.len() >= 9 {
                break;
            }
        }

        // Ensure we have enough fields
        if field_starts.len() < 8 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "insufficient fields",
            ));
        }

        // Extract fields using the positions
        let seqid = line
            [field_starts[0]..get_field_end(line_bytes, field_starts[0], field_starts.get(1))]
            .to_string();
        let source = line
            [field_starts[1]..get_field_end(line_bytes, field_starts[1], field_starts.get(2))]
            .to_string();
        let ty = line
            [field_starts[2]..get_field_end(line_bytes, field_starts[2], field_starts.get(3))]
            .to_string();

        let start_str =
            &line[field_starts[3]..get_field_end(line_bytes, field_starts[3], field_starts.get(4))];
        let start = start_str.parse().map_err(|e| {
            io::Error::new(io::ErrorKind::InvalidData, format!("invalid start: {}", e))
        })?;

        let end_str =
            &line[field_starts[4]..get_field_end(line_bytes, field_starts[4], field_starts.get(5))];
        let end = end_str.parse().map_err(|e| {
            io::Error::new(io::ErrorKind::InvalidData, format!("invalid end: {}", e))
        })?;

        let score_str =
            &line[field_starts[5]..get_field_end(line_bytes, field_starts[5], field_starts.get(6))];
        let score = if score_str == "." {
            None
        } else {
            Some(score_str.parse().map_err(|e| {
                io::Error::new(io::ErrorKind::InvalidData, format!("invalid score: {}", e))
            })?)
        };

        let strand = line
            [field_starts[6]..get_field_end(line_bytes, field_starts[6], field_starts.get(7))]
            .to_string();
        let phase = line
            [field_starts[7]..get_field_end(line_bytes, field_starts[7], field_starts.get(8))]
            .to_string();
        let attributes = if let Some(&attr_start) = field_starts.get(8) {
            line[attr_start..].to_string()
        } else {
            String::new()
        };

        Ok(Self {
            seqid,
            source,
            ty,
            start,
            end,
            score,
            strand,
            phase,
            attributes,
            parsed_attributes: OnceLock::new(),
        })
    }

    /// Get an attribute value by key, parsing attributes lazily
    pub fn get_attribute(&self, key: &str) -> Option<&str> {
        let parsed = self
            .parsed_attributes
            .get_or_init(|| self.parse_attributes());
        parsed.get(key).map(|s| s.as_str())
    }

    /// Parse all attributes into a HashMap (called lazily)
    fn parse_attributes(&self) -> HashMap<String, String> {
        let mut attrs = HashMap::new();

        if self.attributes.is_empty() || self.attributes == "." {
            return attrs;
        }

        // Use SIMD to find semicolons
        let attr_bytes = self.attributes.as_bytes();
        let mut start = 0;

        for semi_pos in memchr_iter(b';', attr_bytes) {
            if let Some((key, value)) = self.attributes[start..semi_pos].split_once('=') {
                attrs.insert(key.to_string(), value.to_string());
            }
            start = semi_pos + 1;
        }

        // Handle the last field (no trailing semicolon)
        if start < self.attributes.len() {
            if let Some((key, value)) = self.attributes[start..].split_once('=') {
                attrs.insert(key.to_string(), value.to_string());
            }
        }

        attrs
    }

    /// Get all parsed attributes as a reference
    pub fn attributes(&self) -> &HashMap<String, String> {
        self.parsed_attributes
            .get_or_init(|| self.parse_attributes())
    }
}

/// Helper function to get field end position
fn get_field_end(
    line_bytes: &[u8],
    _field_start: usize,
    next_field_start: Option<&usize>,
) -> usize {
    match next_field_start {
        Some(&next_start) => next_start - 1, // -1 to exclude the tab
        None => line_bytes.len(),
    }
}

/// SIMD-optimized iterator
pub struct SIMDRecords<R> {
    reader: R,
    line_buf: String,
}

impl<R: BufRead> SIMDRecords<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            line_buf: String::with_capacity(1024),
        }
    }
}

impl<R: BufRead> Iterator for SIMDRecords<R> {
    type Item = io::Result<SIMDRecord>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            self.line_buf.clear();

            match self.reader.read_line(&mut self.line_buf) {
                Ok(0) => return None,
                Ok(_) => {
                    let line = self.line_buf.trim_end();

                    // Skip comments and directives
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }

                    return Some(SIMDRecord::parse_line_simd(line));
                }
                Err(e) => return Some(Err(e)),
            }
        }
    }
}

/// Fast iterator over GFF records using simple string parsing
pub struct FastRecords<R> {
    reader: R,
    line_buf: String,
}

impl<R: BufRead> FastRecords<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            line_buf: String::with_capacity(1024),
        }
    }
}

impl<R: BufRead> Iterator for FastRecords<R> {
    type Item = io::Result<FastRecordOwned>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            self.line_buf.clear();

            match self.reader.read_line(&mut self.line_buf) {
                Ok(0) => return None,
                Ok(_) => {
                    let line = self.line_buf.trim_end();

                    // Skip comments and directives
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }

                    return Some(FastRecordOwned::parse_line(line));
                }
                Err(e) => return Some(Err(e)),
            }
        }
    }
}

/// Helper function to work with any GffRecord implementation
#[allow(dead_code)]
pub fn process_record<T: GffRecord>(record: &T) {
    println!(
        "Processing {} feature {} at {}:{}-{}",
        record.seqid(),
        record.feature_type(),
        record.start(),
        record.end(),
        record
            .score()
            .map_or("no score".to_string(), |s| s.to_string())
    );
}

/// Unified iterator that yields Record enum instances
#[allow(dead_code)]
pub struct Records<R> {
    reader: R,
    line_buf: String,
    use_simd: bool,
}

#[allow(dead_code)]
impl<R: BufRead> Records<R> {
    /// Create a new Records iterator using fast parsing
    pub fn new_fast(reader: R) -> Self {
        Self {
            reader,
            line_buf: String::with_capacity(1024),
            use_simd: false,
        }
    }

    /// Create a new Records iterator using SIMD parsing
    pub fn new_simd(reader: R) -> Self {
        Self {
            reader,
            line_buf: String::with_capacity(1024),
            use_simd: true,
        }
    }
}

impl<R: BufRead> Iterator for Records<R> {
    type Item = io::Result<Record>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            self.line_buf.clear();

            match self.reader.read_line(&mut self.line_buf) {
                Ok(0) => return None,
                Ok(_) => {
                    let line = self.line_buf.trim_end();

                    // Skip comments and directives
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }

                    return Some(if self.use_simd {
                        Record::parse_simd(line)
                    } else {
                        Record::parse_fast(line)
                    });
                }
                Err(e) => return Some(Err(e)),
            }
        }
    }
}

/// Generic iterator that can work with any GffRecord implementation (deprecated, use Records instead)
#[allow(dead_code)]
pub enum GffRecordIterator<R: BufRead> {
    Fast(FastRecords<R>),
    Simd(SIMDRecords<R>),
}

#[allow(dead_code)]
impl<R: BufRead> GffRecordIterator<R> {
    pub fn new_fast(reader: R) -> Self {
        Self::Fast(FastRecords::new(reader))
    }

    pub fn new_simd(reader: R) -> Self {
        Self::Simd(SIMDRecords::new(reader))
    }
}

impl<R: BufRead> Iterator for GffRecordIterator<R> {
    type Item = io::Result<Box<dyn GffRecord>>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Fast(iter) => iter
                .next()
                .map(|result| result.map(|record| Box::new(record) as Box<dyn GffRecord>)),
            Self::Simd(iter) => iter
                .next()
                .map(|result| result.map(|record| Box::new(record) as Box<dyn GffRecord>)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fast_record_parse() {
        let line = "seq1\ttest\tgene\t1000\t2000\t.\t+\t.\tID=gene1;Name=test";
        let record = FastRecordOwned::parse_line(line).unwrap();

        assert_eq!(record.seqid, "seq1");
        assert_eq!(record.ty, "gene");
        assert_eq!(record.start, 1000);
        assert_eq!(record.end, 2000);
        assert!(record.score.is_none());
        assert_eq!(record.attributes, "ID=gene1;Name=test");
    }

    #[test]
    fn test_gff_record_trait() {
        let line = "seq1\ttest\tgene\t1000\t2000\t5.5\t+\t0\tID=gene1;Name=test";

        // Test with FastRecordOwned
        let fast_record = FastRecordOwned::parse_line(line).unwrap();
        assert_eq!(fast_record.seqid(), "seq1");
        assert_eq!(fast_record.feature_type(), "gene");
        assert_eq!(fast_record.start(), 1000);
        assert_eq!(fast_record.end(), 2000);
        assert_eq!(fast_record.score(), Some(5.5));
        assert_eq!(fast_record.get_attribute("ID"), Some("gene1"));

        // Test with SIMDRecord
        let simd_record = SIMDRecord::parse_line_simd(line).unwrap();
        assert_eq!(simd_record.seqid(), "seq1");
        assert_eq!(simd_record.feature_type(), "gene");
        assert_eq!(simd_record.start(), 1000);
        assert_eq!(simd_record.end(), 2000);
        assert_eq!(simd_record.score(), Some(5.5));
        assert_eq!(simd_record.get_attribute("ID"), Some("gene1"));

        // Test generic function works with both
        fn test_with_trait<T: GffRecord>(record: &T) {
            assert_eq!(record.seqid(), "seq1");
            assert_eq!(record.feature_type(), "gene");
        }

        test_with_trait(&fast_record);
        test_with_trait(&simd_record);
    }

    #[test]
    fn test_unified_record() {
        let line = "seq1\ttest\tgene\t1000\t2000\t5.5\t+\t0\tID=gene1;Name=test";

        // Test creating Records from both types
        let fast_record = Record::parse_fast(line).unwrap();
        let simd_record = Record::parse_simd(line).unwrap();

        // Both should provide the same interface
        assert_eq!(fast_record.seqid(), "seq1");
        assert_eq!(fast_record.feature_type(), "gene");
        assert_eq!(fast_record.start(), 1000);
        assert_eq!(fast_record.end(), 2000);
        assert_eq!(fast_record.get_attribute("ID"), Some("gene1"));

        assert_eq!(simd_record.seqid(), "seq1");
        assert_eq!(simd_record.feature_type(), "gene");
        assert_eq!(simd_record.start(), 1000);
        assert_eq!(simd_record.end(), 2000);
        assert_eq!(simd_record.get_attribute("ID"), Some("gene1"));

        // Test type checking
        assert!(fast_record.is_fast());
        assert!(!fast_record.is_simd());
        assert!(simd_record.is_simd());
        assert!(!simd_record.is_fast());

        // Test generic usage
        fn process_any_record(record: &Record) -> String {
            format!("{}:{}-{}", record.seqid(), record.start(), record.end())
        }

        assert_eq!(process_any_record(&fast_record), "seq1:1000-2000");
        assert_eq!(process_any_record(&simd_record), "seq1:1000-2000");
    }

    #[test]
    fn test_unified_iterator() {
        use std::io::Cursor;

        let data = "seq1\ttest\tgene\t1000\t2000\t5.5\t+\t0\tID=gene1\nseq1\ttest\texon\t1200\t1800\t.\t+\t.\tID=exon1";

        // Test fast iterator
        let cursor = Cursor::new(data);
        let fast_records: Vec<_> = Records::new_fast(cursor)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(fast_records.len(), 2);
        assert_eq!(fast_records[0].feature_type(), "gene");
        assert_eq!(fast_records[1].feature_type(), "exon");
        assert!(fast_records[0].is_fast());
        assert!(fast_records[1].is_fast());

        // Test SIMD iterator
        let cursor = Cursor::new(data);
        let simd_records: Vec<_> = Records::new_simd(cursor)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(simd_records.len(), 2);
        assert_eq!(simd_records[0].feature_type(), "gene");
        assert_eq!(simd_records[1].feature_type(), "exon");
        assert!(simd_records[0].is_simd());
        assert!(simd_records[1].is_simd());
    }
    
    #[test]
    fn test_coordinate_system_consistency() {
        // Test that our fast/SIMD parsers preserve the same coordinate system as original noodles
        let line = "chr1\tensembl\tgene\t1000\t2000\t.\t+\t.\tID=gene1";
        
        // Original noodles GFF parsing
        let original_record = crate::record::Record::try_new(line.as_bytes()).unwrap();
        let original_start = original_record.start().unwrap().get();
        let original_end = original_record.end().unwrap().get();
        
        // Fast parsing
        let fast_record = FastRecordOwned::parse_line(line).unwrap();
        let fast_start = fast_record.start();
        let fast_end = fast_record.end();
        
        // SIMD parsing  
        let simd_record = SIMDRecord::parse_line_simd(line).unwrap();
        let simd_start = simd_record.start();
        let simd_end = simd_record.end();
        
        // All should return the same coordinate values
        assert_eq!(original_start, fast_start as usize, "FastRecord start coordinate should match original noodles");
        assert_eq!(original_end, fast_end as usize, "FastRecord end coordinate should match original noodles");
        assert_eq!(original_start, simd_start as usize, "SIMDRecord start coordinate should match original noodles");
        assert_eq!(original_end, simd_end as usize, "SIMDRecord end coordinate should match original noodles");
        
        // Verify the coordinate system: GFF uses 1-based coordinates
        assert_eq!(fast_start, 1000, "Fast parser should preserve 1-based coordinates from GFF");
        assert_eq!(fast_end, 2000, "Fast parser should preserve 1-based coordinates from GFF");
        assert_eq!(simd_start, 1000, "SIMD parser should preserve 1-based coordinates from GFF");
        assert_eq!(simd_end, 2000, "SIMD parser should preserve 1-based coordinates from GFF");
        
        // Test edge case: coordinate 1 (minimum valid 1-based coordinate)
        let edge_line = "chr1\tensembl\tgene\t1\t1\t.\t+\t.\tID=edge";
        let original_edge = crate::record::Record::try_new(edge_line.as_bytes()).unwrap();
        let fast_edge = FastRecordOwned::parse_line(edge_line).unwrap();
        let simd_edge = SIMDRecord::parse_line_simd(edge_line).unwrap();
        
        assert_eq!(original_edge.start().unwrap().get(), 1);
        assert_eq!(fast_edge.start(), 1);
        assert_eq!(simd_edge.start(), 1);
        assert_eq!(original_edge.end().unwrap().get(), 1); 
        assert_eq!(fast_edge.end(), 1);
        assert_eq!(simd_edge.end(), 1);
    }
}
