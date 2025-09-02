use std::io::{self, BufRead};
use std::collections::HashMap;
use std::sync::OnceLock;
use memchr::memchr_iter;
use crate::feature::{RecordBuf, record_buf::{attributes::field::{Tag, Value}, Attributes}};
use crate::feature::record::{Strand, Phase};
use noodles_core::Position;
use bstr::BString;

/// A fast, zero-copy GFF record that defers parsing until needed
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
        self.start.parse().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    /// Parse end position only when needed
    pub fn end_position(&self) -> io::Result<u32> {
        self.end.parse().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
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

impl FastRecordOwned {
    /// Parse a line into owned record with minimal allocations
    pub fn parse_line(line: &str) -> io::Result<Self> {
        let mut fields = line.splitn(9, '\t');
        
        let seqid = fields.next().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing seqid"))?.to_string();
        let source = fields.next().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing source"))?.to_string();
        let ty = fields.next().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing type"))?.to_string();
        
        let start_str = fields.next().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing start"))?;
        let start = start_str.parse().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("invalid start: {}", e)))?;
        
        let end_str = fields.next().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing end"))?;
        let end = end_str.parse().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("invalid end: {}", e)))?;
        
        let score_str = fields.next().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing score"))?;
        let score = if score_str == "." {
            None
        } else {
            Some(score_str.parse().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("invalid score: {}", e)))?)
        };
        
        let strand = fields.next().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing strand"))?.to_string();
        let phase = fields.next().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing phase"))?.to_string();
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

    /// Convert to RecordBuf with proper attribute parsing
    pub fn to_record_buf(&self) -> io::Result<RecordBuf> {
        let start = Position::new(self.start as usize)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid start position"))?;
        let end = Position::new(self.end as usize)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid end position"))?;

        let strand = match self.strand.as_str() {
            "+" => Strand::Forward,
            "-" => Strand::Reverse,
            "?" => Strand::Unknown,
            _ => Strand::None,
        };

        let phase = match self.phase.as_str() {
            "0" => Some(Phase::Zero),
            "1" => Some(Phase::One),
            "2" => Some(Phase::Two),
            _ => None,
        };

        // Convert attributes HashMap to proper Attributes format
        let mut attributes = Attributes::default();
        for (key, value) in self.attributes() {
            let tag = Tag::from(key.clone());
            let attr_value = if value.contains(',') {
                // Split comma-separated values into array
                let values: Vec<BString> = value.split(',').map(|v| BString::from(v.trim())).collect();
                Value::Array(values)
            } else {
                Value::String(BString::from(value.clone()))
            };
            attributes.as_mut().insert(tag, attr_value);
        }

        let mut builder = RecordBuf::builder()
            .set_reference_sequence_name(self.seqid.as_str())
            .set_source(self.source.as_str())
            .set_type(self.ty.as_str())
            .set_start(start)
            .set_end(end)
            .set_strand(strand)
            .set_attributes(attributes);

        if let Some(score) = self.score {
            builder = builder.set_score(score);
        }

        if let Some(phase) = phase {
            builder = builder.set_phase(phase);
        }

        Ok(builder.build())
    }
    
    /// Get an attribute value by key, parsing attributes lazily
    pub fn get_attribute(&self, key: &str) -> Option<&str> {
        let parsed = self.parsed_attributes.get_or_init(|| {
            self.parse_attributes()
        });
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
        self.parsed_attributes.get_or_init(|| {
            self.parse_attributes()
        })
    }
}

/// Zero-copy record that borrows from a buffer
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
        self.start_str.parse().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }
    
    /// Parse end position on demand
    pub fn end(&self) -> io::Result<u32> {
        self.end_str.parse().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
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

/// Simplified zero-copy approach - let's skip this for now due to lifetime complexity
/// Instead, let's focus on the fast parser with minimal allocations

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
            return Err(io::Error::new(io::ErrorKind::InvalidData, "insufficient fields"));
        }
        
        // Extract fields using the positions
        let seqid = line[field_starts[0]..get_field_end(line_bytes, field_starts[0], field_starts.get(1))].to_string();
        let source = line[field_starts[1]..get_field_end(line_bytes, field_starts[1], field_starts.get(2))].to_string();
        let ty = line[field_starts[2]..get_field_end(line_bytes, field_starts[2], field_starts.get(3))].to_string();
        
        let start_str = &line[field_starts[3]..get_field_end(line_bytes, field_starts[3], field_starts.get(4))];
        let start = start_str.parse().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("invalid start: {}", e)))?;
        
        let end_str = &line[field_starts[4]..get_field_end(line_bytes, field_starts[4], field_starts.get(5))];
        let end = end_str.parse().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("invalid end: {}", e)))?;
        
        let score_str = &line[field_starts[5]..get_field_end(line_bytes, field_starts[5], field_starts.get(6))];
        let score = if score_str == "." {
            None
        } else {
            Some(score_str.parse().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("invalid score: {}", e)))?)
        };
        
        let strand = line[field_starts[6]..get_field_end(line_bytes, field_starts[6], field_starts.get(7))].to_string();
        let phase = line[field_starts[7]..get_field_end(line_bytes, field_starts[7], field_starts.get(8))].to_string();
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
        let parsed = self.parsed_attributes.get_or_init(|| {
            self.parse_attributes()
        });
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
        self.parsed_attributes.get_or_init(|| {
            self.parse_attributes()
        })
    }
}

/// Helper function to get field end position
fn get_field_end(line_bytes: &[u8], _field_start: usize, next_field_start: Option<&usize>) -> usize {
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
}