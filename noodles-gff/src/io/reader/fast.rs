use std::io::{self, BufRead};

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

/// Fast record that owns its data to avoid lifetime issues
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
        })
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