use std::io;
use std::collections::HashMap;
use std::sync::OnceLock;
use std::pin::Pin;

use tokio::io::{AsyncBufRead, AsyncBufReadExt};
use futures::Stream;
use memchr::memchr_iter;
use noodles_core::Position;

use crate::feature::{RecordBuf, record::{Strand, Phase}};
use crate::feature::record_buf::Attributes;

/// Async fast record that owns its data with lazy attribute parsing
#[derive(Debug)]
pub struct AsyncFastRecord {
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

impl AsyncFastRecord {
    /// Parse a line into owned record with minimal allocations
    pub fn parse_line(line: &str) -> io::Result<Self> {
        let mut fields = line.splitn(9, '\t');
        
        let seqid = fields.next().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, format!("missing seqid in line: '{}'", line)))?.to_string();
        let source = fields.next().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, format!("missing source in line: '{}' (fields: {})", line, line.split('\t').count())))?.to_string();
        let ty = fields.next().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing type"))?.to_string();
        
        let start_str = fields.next().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing start"))?;
        let start = start_str.parse().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("invalid start: {}", e)))?;
        
        let end_str = fields.next().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, format!("missing end in line: '{}'", line)))?;
        let end = end_str.parse().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("invalid end '{}' in line: '{}': {}", end_str, line, e)))?;
        
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
    
    /// Convert this record to a RecordBuf
    pub fn to_record_buf(&self) -> io::Result<RecordBuf> {
        // Parse strand
        let strand = match self.strand.as_str() {
            "+" => Strand::Forward,
            "-" => Strand::Reverse,
            "?" => Strand::Unknown,
            _ => Strand::None,
        };
        
        // Parse phase
        let phase = match self.phase.as_str() {
            "0" => Some(Phase::Zero),
            "1" => Some(Phase::One), 
            "2" => Some(Phase::Two),
            _ => None,
        };
        
        // Convert positions (GFF is 1-based)
        let start = Position::try_from(self.start as usize)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("invalid start position: {}", e)))?;
        let end = Position::try_from(self.end as usize)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("invalid end position: {}", e)))?;
            
        // For now, just use default attributes - full attribute parsing would require
        // more complex handling of the Attributes type
        let attributes = Attributes::default();
        
        let mut builder = RecordBuf::builder()
            .set_reference_sequence_name(self.seqid.clone())
            .set_source(self.source.clone())
            .set_type(self.ty.clone())
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
}

/// SIMD-optimized async record using memchr for field splitting
#[derive(Debug)]
pub struct AsyncSIMDRecord {
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

impl AsyncSIMDRecord {
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
    
    /// Convert this record to a RecordBuf
    pub fn to_record_buf(&self) -> io::Result<RecordBuf> {
        // Parse strand
        let strand = match self.strand.as_str() {
            "+" => Strand::Forward,
            "-" => Strand::Reverse,
            "?" => Strand::Unknown,
            _ => Strand::None,
        };
        
        // Parse phase
        let phase = match self.phase.as_str() {
            "0" => Some(Phase::Zero),
            "1" => Some(Phase::One), 
            "2" => Some(Phase::Two),
            _ => None,
        };
        
        // Convert positions (GFF is 1-based)
        let start = Position::try_from(self.start as usize)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("invalid start position: {}", e)))?;
        let end = Position::try_from(self.end as usize)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("invalid end position: {}", e)))?;
            
        // For now, just use default attributes - full attribute parsing would require
        // more complex handling of the Attributes type
        let attributes = Attributes::default();
        
        let mut builder = RecordBuf::builder()
            .set_reference_sequence_name(self.seqid.clone())
            .set_source(self.source.clone())
            .set_type(self.ty.clone())
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
}

/// Helper function to get field end position
fn get_field_end(line_bytes: &[u8], _field_start: usize, next_field_start: Option<&usize>) -> usize {
    match next_field_start {
        Some(&next_start) => next_start - 1, // -1 to exclude the tab
        None => line_bytes.len(),
    }
}

/// Async fast stream over GFF records
pub struct AsyncFastRecords<R> {
    reader: R,
    line_buf: String,
}

impl<R> AsyncFastRecords<R> 
where
    R: AsyncBufRead + Unpin,
{
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            line_buf: String::with_capacity(1024),
        }
    }
    
    /// Read the next record asynchronously
    pub async fn next_record(&mut self) -> io::Result<Option<AsyncFastRecord>> {
        loop {
            self.line_buf.clear();
            
            match self.reader.read_line(&mut self.line_buf).await? {
                0 => return Ok(None),
                _ => {
                    let line = self.line_buf.trim_end();
                    
                    // Skip comments and directives
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }
                    
                    // Skip lines that don't look like GFF records (need at least 8 fields)
                    let fields: Vec<&str> = line.split('\t').collect();
                    if fields.len() < 8 {
                        eprintln!("Warning: Skipping malformed line (not enough fields): '{}'", line);
                        continue;
                    }
                    
                    // Quick validation of numeric fields (start and end positions)
                    if fields[3].parse::<u32>().is_err() || fields[4].parse::<u32>().is_err() {
                        eprintln!("Warning: Skipping line with invalid positions: '{}'", line);
                        continue;
                    }
                    
                    return Ok(Some(AsyncFastRecord::parse_line(line)?));
                }
            }
        }
    }
}

impl<R> Stream for AsyncFastRecords<R>
where
    R: AsyncBufRead + Unpin,
{
    type Item = io::Result<AsyncFastRecord>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        use std::task::Poll;
        
        let future = self.next_record();
        tokio::pin!(future);
        
        match future.poll(cx) {
            Poll::Ready(Ok(Some(record))) => Poll::Ready(Some(Ok(record))),
            Poll::Ready(Ok(None)) => Poll::Ready(None),
            Poll::Ready(Err(e)) => Poll::Ready(Some(Err(e))),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Async SIMD-optimized stream over GFF records
pub struct AsyncSIMDRecords<R> {
    reader: R,
    line_buf: String,
}

impl<R> AsyncSIMDRecords<R>
where
    R: AsyncBufRead + Unpin,
{
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            line_buf: String::with_capacity(1024),
        }
    }
    
    /// Read the next record asynchronously using SIMD optimization
    pub async fn next_record(&mut self) -> io::Result<Option<AsyncSIMDRecord>> {
        loop {
            self.line_buf.clear();
            
            match self.reader.read_line(&mut self.line_buf).await? {
                0 => return Ok(None),
                _ => {
                    let line = self.line_buf.trim_end();
                    
                    // Skip comments and directives
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }
                    
                    // Skip lines that don't look like GFF records (need at least 8 fields)
                    let fields: Vec<&str> = line.split('\t').collect();
                    if fields.len() < 8 {
                        eprintln!("Warning: Skipping malformed line (not enough fields): '{}'", line);
                        continue;
                    }
                    
                    // Quick validation of numeric fields (start and end positions)
                    if fields[3].parse::<u32>().is_err() || fields[4].parse::<u32>().is_err() {
                        eprintln!("Warning: Skipping line with invalid positions: '{}'", line);
                        continue;
                    }
                    
                    return Ok(Some(AsyncSIMDRecord::parse_line_simd(line)?));
                }
            }
        }
    }
}

impl<R> Stream for AsyncSIMDRecords<R>
where
    R: AsyncBufRead + Unpin,
{
    type Item = io::Result<AsyncSIMDRecord>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        use std::task::Poll;
        
        let future = self.next_record();
        tokio::pin!(future);
        
        match future.poll(cx) {
            Poll::Ready(Ok(Some(record))) => Poll::Ready(Some(Ok(record))),
            Poll::Ready(Ok(None)) => Poll::Ready(None),
            Poll::Ready(Err(e)) => Poll::Ready(Some(Err(e))),
            Poll::Pending => Poll::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::BufReader;

    #[tokio::test]
    async fn test_async_fast_record_parse() {
        let line = "seq1\ttest\tgene\t1000\t2000\t.\t+\t.\tID=gene1;Name=test";
        let record = AsyncFastRecord::parse_line(line).unwrap();
        
        assert_eq!(record.seqid, "seq1");
        assert_eq!(record.ty, "gene");
        assert_eq!(record.start, 1000);
        assert_eq!(record.end, 2000);
        assert!(record.score.is_none());
        assert_eq!(record.attributes, "ID=gene1;Name=test");
        assert_eq!(record.get_attribute("ID"), Some("gene1"));
    }

    #[tokio::test]
    async fn test_async_simd_record_parse() {
        let line = "seq1\ttest\tgene\t1000\t2000\t.\t+\t.\tID=gene1;Name=test";
        let record = AsyncSIMDRecord::parse_line_simd(line).unwrap();
        
        assert_eq!(record.seqid, "seq1");
        assert_eq!(record.ty, "gene");
        assert_eq!(record.start, 1000);
        assert_eq!(record.end, 2000);
        assert!(record.score.is_none());
        assert_eq!(record.attributes, "ID=gene1;Name=test");
        assert_eq!(record.get_attribute("ID"), Some("gene1"));
    }
    
    #[tokio::test]
    async fn test_async_stream() {
        use futures::StreamExt;
        
        let data = b"##gff-version 3\nseq1\ttest\tgene\t1000\t2000\t.\t+\t.\tID=gene1;Name=test\n";
        let reader = BufReader::new(&data[..]);
        let mut stream = AsyncFastRecords::new(reader);
        
        let record = stream.next().await.unwrap().unwrap();
        assert_eq!(record.seqid, "seq1");
        assert_eq!(record.ty, "gene");
    }
}