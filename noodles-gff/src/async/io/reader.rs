mod line;
pub mod fast;

use futures::{Stream, TryStreamExt, stream};
use tokio::io::{self, AsyncBufRead, AsyncBufReadExt};

use crate::{Line, LineBuf, directive_buf::key, feature::RecordBuf};
use self::fast::{AsyncFastRecords, AsyncSIMDRecords};

/// An async GFF reader.
pub struct Reader<R> {
    inner: R,
}

impl<R> Reader<R> {
    /// Returns a reference to the underlying reader.
    ///
    /// # Examples
    ///
    /// ```
    /// use noodles_gff as gff;
    /// use tokio::io;
    /// let reader = gff::r#async::io::Reader::new(io::empty());
    /// let _inner = reader.get_ref();
    /// ```
    pub fn get_ref(&self) -> &R {
        &self.inner
    }

    /// Returns a mutable reference to the underlying reader.
    ///
    /// # Examples
    ///
    /// ```
    /// use noodles_gff as gff;
    /// use tokio::io;
    /// let mut reader = gff::r#async::io::Reader::new(io::empty());
    /// let _inner = reader.get_mut();
    /// ```
    pub fn get_mut(&mut self) -> &mut R {
        &mut self.inner
    }

    /// Unwraps and returns the underlying reader.
    ///
    /// # Examples
    ///
    /// ```
    /// use noodles_gff as gff;
    /// use tokio::io;
    /// let reader = gff::r#async::io::Reader::new(io::empty());
    /// let _inner = reader.into_inner();
    /// ```
    pub fn into_inner(self) -> R {
        self.inner
    }
}

impl<R> Reader<R>
where
    R: AsyncBufRead + Unpin,
{
    /// Creates an async GFF reader.
    ///
    /// # Examples
    ///
    /// ```
    /// use noodles_gff as gff;
    /// use tokio::io;
    /// let reader = gff::r#async::io::Reader::new(io::empty());
    /// ```
    pub fn new(inner: R) -> Self {
        Self { inner }
    }

    /// Reads a lazy line.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[tokio::main]
    /// # async fn main() -> tokio::io::Result<()> {
    /// use noodles_gff as gff;
    ///
    /// let data = b"##gff-version 3\n";
    /// let mut reader = gff::r#async::io::Reader::new(&data[..]);
    ///
    /// let mut line = gff::Line::default();
    ///
    /// reader.read_line(&mut line).await?;
    /// assert_eq!(line.kind(), gff::line::Kind::Directive);
    ///
    /// assert_eq!(reader.read_line(&mut line).await?, 0);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn read_line(&mut self, line: &mut Line) -> io::Result<usize> {
        line::read_line(&mut self.inner, line).await
    }

    /// Returns a stream over lines.
    ///
    /// When using this, the caller is responsible to stop reading at either EOF or when the
    /// `FASTA` directive is read, whichever comes first.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[tokio::main]
    /// # async fn main() -> tokio::io::Result<()> {
    /// use futures::TryStreamExt;
    /// use noodles_gff::{self as gff, directive_buf::key};
    /// use tokio::io;
    ///
    /// let mut reader = gff::r#async::io::Reader::new(io::empty());
    /// let mut lines = reader.lines();
    ///
    /// while let Some(line) = lines.try_next().await? {
    ///     if let Some(key::FASTA) = line.as_directive().map(|directive| directive.key().as_ref()) {
    ///         break;
    ///     }
    ///
    ///     // ...
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn lines(&mut self) -> impl Stream<Item = io::Result<Line>> + '_ {
        Box::pin(stream::try_unfold(
            (self, Line::default()),
            |(reader, mut line)| async {
                reader.read_line(&mut line).await.map(|n| match n {
                    0 => None,
                    _ => Some((line.clone(), (reader, line))),
                })
            },
        ))
    }

    /// Returns a stream over line buffers.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[tokio::main]
    /// # async fn main() -> tokio::io::Result<()> {
    /// use futures::TryStreamExt;
    /// use noodles_gff::{self as gff, LineBuf};
    ///
    /// let data = b"##gff-version 3\n";
    /// let mut reader = gff::r#async::io::Reader::new(&data[..]);
    /// let mut lines = reader.line_bufs();
    ///
    /// let line = lines.try_next().await?;
    /// assert!(matches!(line, Some(LineBuf::Directive(_))));
    ///
    /// assert!(lines.try_next().await?.is_none());
    /// # Ok(())
    /// # }
    /// ```
    pub fn line_bufs(&mut self) -> impl Stream<Item = io::Result<LineBuf>> + '_ {
        use crate::line::Kind;

        Box::pin(stream::try_unfold(
            (self, Line::default()),
            |(reader, mut line)| async {
                reader.read_line(&mut line).await.and_then(|n| match n {
                    0 => Ok(None),
                    _ => match line.kind() {
                        Kind::Directive => {
                            let directive = line
                                .as_directive()
                                .map(|d| LineBuf::Directive(d.into()))
                                .unwrap(); // SAFETY: `line` is a directive.

                            Ok(Some((directive, (reader, line))))
                        }
                        Kind::Comment => Ok(Some((
                            LineBuf::Comment(line.as_ref().into()),
                            (reader, line),
                        ))),
                        Kind::Record => {
                            let record = line
                                .as_record()
                                .unwrap() // SAFETY: `line` is a record.
                                .and_then(|record| {
                                    RecordBuf::try_from_feature_record(&record).map(LineBuf::Record)
                                })?;

                            Ok(Some((record, (reader, line))))
                        }
                    },
                })
            },
        ))
    }

    /// Returns a stream over records.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[tokio::main]
    /// # async fn main() -> tokio::io::Result<()> {
    /// use futures::TryStreamExt;
    /// use noodles_gff as gff;
    ///
    /// let data = b"##gff-version 3\n";
    /// let mut reader = gff::r#async::io::Reader::new(&data[..]);
    /// let mut records = reader.record_bufs();
    ///
    /// assert!(records.try_next().await?.is_none());
    /// # Ok(())
    /// # }
    /// ```
    pub fn record_bufs(&mut self) -> impl Stream<Item = io::Result<RecordBuf>> + '_ {
        Box::pin(stream::try_unfold(self.line_bufs(), |mut lines| async {
            loop {
                match lines.try_next().await? {
                    None => return Ok(None),
                    Some(LineBuf::Directive(directive)) if directive.key() == key::FASTA => {
                        return Ok(None);
                    }
                    Some(LineBuf::Record(record)) => return Ok(Some((record, lines))),
                    _ => {}
                }
            }
        }))
    }
    
    /// Returns a fast async stream over records using simple string parsing.
    ///
    /// This is a performance-optimized async parser that skips complex abstractions.
    /// It consumes the reader and returns owned records.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[tokio::main]
    /// # async fn main() -> tokio::io::Result<()> {
    /// use futures::StreamExt;
    /// use noodles_gff as gff;
    /// use tokio::io::BufReader;
    ///
    /// let data = b"##gff-version 3\nsq0\tNOODLES\tgene\t8\t13\t.\t+\t.\tgene_id=ndls0;gene_name=gene0\n";
    /// let reader = gff::r#async::io::Reader::new(BufReader::new(&data[..]));
    /// let mut stream = reader.fast_records();
    ///
    /// while let Some(result) = stream.next().await {
    ///     let record = result?;
    ///     // Process record...
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn fast_records(self) -> AsyncFastRecords<R> {
        AsyncFastRecords::new(self.inner)
    }
    
    /// Returns a SIMD-optimized async stream over records.
    ///
    /// This uses vectorized operations for field splitting in an async context.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[tokio::main]
    /// # async fn main() -> tokio::io::Result<()> {
    /// use futures::StreamExt;
    /// use noodles_gff as gff;
    /// use tokio::io::BufReader;
    ///
    /// let data = b"##gff-version 3\nsq0\tNOODLES\tgene\t8\t13\t.\t+\t.\tgene_id=ndls0;gene_name=gene0\n";
    /// let reader = gff::r#async::io::Reader::new(BufReader::new(&data[..]));
    /// let mut stream = reader.simd_records();
    ///
    /// while let Some(result) = stream.next().await {
    ///     let record = result?;
    ///     // Process record...
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn simd_records(self) -> AsyncSIMDRecords<R> {
        AsyncSIMDRecords::new(self.inner)
    }
}

async fn read_line<R>(reader: &mut R, buf: &mut Vec<u8>) -> io::Result<usize>
where
    R: AsyncBufRead + Unpin,
{
    const LINE_FEED: u8 = b'\n';
    const CARRIAGE_RETURN: u8 = b'\r';

    match reader.read_until(LINE_FEED, buf).await? {
        0 => Ok(0),
        n => {
            if buf.ends_with(&[LINE_FEED]) {
                buf.pop();

                if buf.ends_with(&[CARRIAGE_RETURN]) {
                    buf.pop();
                }
            }

            Ok(n)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_read_line() -> io::Result<()> {
        async fn t(buf: &mut Vec<u8>, mut data: &[u8], expected: &[u8]) -> io::Result<()> {
            buf.clear();
            read_line(&mut data, buf).await?;
            assert_eq!(buf, expected);
            Ok(())
        }

        let mut buf = Vec::new();

        t(&mut buf, b"noodles\n", b"noodles").await?;
        t(&mut buf, b"noodles\r\n", b"noodles").await?;
        t(&mut buf, b"noodles", b"noodles").await?;

        Ok(())
    }
}
