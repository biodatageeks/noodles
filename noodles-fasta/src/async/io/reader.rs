mod sequence;

use futures::{stream, Stream};
use tokio::io::{
    self, AsyncBufRead, AsyncBufReadExt, AsyncRead, AsyncSeek, AsyncSeekExt, SeekFrom,
};

use self::sequence::read_sequence;
use crate::Record;

/// An async FASTA reader.
pub struct Reader<R> {
    inner: R,
}

impl<R> Reader<R> {
    /// Returns a reference to the underlying reader.
    ///
    /// # Examples
    ///
    /// ```
    /// use noodles_fasta as fasta;
    /// use tokio::io;
    /// let reader = fasta::r#async::io::Reader::new(io::empty());
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
    /// use noodles_fasta as fasta;
    /// use tokio::io;
    /// let mut reader = fasta::r#async::io::Reader::new(io::empty());
    /// let _inner = reader.get_mut();
    /// ```
    pub fn get_mut(&mut self) -> &mut R {
        &mut self.inner
    }

    /// Returns the underlying reader.
    ///
    /// # Examples
    ///
    /// ```
    /// use noodles_fasta as fasta;
    /// use tokio::io;
    /// let reader = fasta::r#async::io::Reader::new(io::empty());
    /// let _inner = reader.into_inner();
    /// ```
    pub fn into_inner(self) -> R {
        self.inner
    }
}

impl<R>
    Reader<R>
where
    R: AsyncBufRead + Unpin,
{
    /// Creates an async FASTA reader.
    ///
    /// # Examples
    ///
    /// ```
    /// use noodles_fasta as fasta;
    /// let data = [];
    /// let mut reader = fasta::r#async::io::Reader::new(&data[..]);
    /// ```
    pub fn new(inner: R) -> Self {
        Self { inner }
    }

    /// Reads a raw definition line.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::io;
    /// #
    /// # #[tokio::main]
    /// # async fn main() -> io::Result<()> {
    /// use noodles_fasta as fasta;
    ///
    /// let data = b">sq0\nACGT\n>sq1\nNNNN\nNNNN\nNN\n";
    /// let mut reader = fasta::r#async::io::Reader::new(&data[..]);
    ///
    /// let mut buf = String::new();
    /// reader.read_definition(&mut buf).await?;
    ///
    /// assert_eq!(buf, ">sq0");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn read_definition(&mut self, buf: &mut String) -> io::Result<usize> {
        read_line(&mut self.inner, buf).await
    }

    /// Reads a sequence.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::io;
    /// #
    /// # #[tokio::main]
    /// # async fn main() -> io::Result<()> {
    /// use noodles_fasta as fasta;
    ///
    /// let data = b">sq0\nACGT\n>sq1\nNNNN\nNNNN\nNN\n";
    /// let mut reader = fasta::r#async::io::Reader::new(&data[..]);
    /// reader.read_definition(&mut String::new()).await?;
    ///
    /// let mut buf = Vec::new();
    /// reader.read_sequence(&mut buf).await?;
    ///
    /// assert_eq!(buf, b"ACGT");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn read_sequence(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
        read_sequence(&mut self.inner, buf).await
    }

    /// Reads a record.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::io;
    /// #
    /// # #[tokio::main]
    /// # async fn main() -> io::Result<()> {
    /// use noodles_fasta as fasta;
    ///
    /// let data = b">sq0\nACGT\n";
    /// let mut reader = fasta::r#async::io::Reader::new(&data[..]);
    ///
    /// let mut record = fasta::Record::default();
    /// reader.read_record(&mut record).await?;
    ///
    /// assert_eq!(record.name(), "sq0");
    /// assert_eq!(record.sequence(), b"ACGT");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn read_record(&mut self, record: &mut Record) -> io::Result<usize> {
        read_record(&mut self.inner, record).await
    }

    /// Returns an (async) stream over records starting from the current (input) stream position.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[tokio::main]
    /// # async fn main() -> tokio::io::Result<()> {
    /// use futures::TryStreamExt;
    /// use noodles_fasta as fasta;
    ///
    /// let data = b">sq0\nACGT\n>sq1\nNNNN\n";
    /// let mut reader = fasta::r#async::io::Reader::new(&data[..]);
    /// let mut records = reader.records();
    ///
    /// let record = records.try_next().await?.unwrap();
    /// assert_eq!(record.name(), "sq0");
    /// assert_eq!(record.sequence(), b"ACGT");
    ///
    /// let record = records.try_next().await?.unwrap();
    /// assert_eq!(record.name(), "sq1");
    /// assert_eq!(record.sequence(), b"NNNN");
    ///
    /// assert!(records.try_next().await?.is_none());
    /// # Ok(())
    /// # }
    /// ```
    pub fn records(&mut self) -> impl Stream<Item = io::Result<Record>> + '_ {
        Box::pin(stream::try_unfold(
            (self, Record::default()),
            |(reader, mut record)| async {
                match reader.read_record(&mut record).await? {
                    0 => Ok(None),
                    _ => Ok(Some((record.clone(), (reader, record)))),
                }
            },
        ))
    }
}

impl<R>
    Reader<R>
where
    R: AsyncRead + AsyncSeek + Unpin,
{
    /// Seeks the underlying stream to the given position.
    pub async fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.inner.seek(pos).await
    }
}

async fn read_record<R>(reader: &mut R, record: &mut Record) -> io::Result<usize>
where
    R: AsyncBufRead + Unpin,
{
    record.clear();

    let mut definition_buf = String::new();

    let n = match read_line(reader, &mut definition_buf).await? {
        0 => return Ok(0),
        n => n,
    };

    let definition = definition_buf
        .parse()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    *record.definition_mut() = definition;

    let mut sequence_buf = Vec::new();
    let m = read_sequence(reader, &mut sequence_buf).await?;
    *record.sequence_mut() = sequence_buf.into();

    Ok(n + m)
}

pub(crate) async fn read_line<R>(reader: &mut R, buf: &mut String) -> io::Result<usize>
where
    R: AsyncBufRead + Unpin,
{
    const LINE_FEED: char = '\n';
    const CARRIAGE_RETURN: char = '\r';

    match reader.read_line(buf).await? {
        0 => Ok(0),
        n => {
            if buf.ends_with(LINE_FEED) {
                buf.pop();

                if buf.ends_with(CARRIAGE_RETURN) {
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
    async fn test_read_definition() -> io::Result<()> {
        let data = b">sq0\nACGT\n";
        let mut reader = Reader::new(&data[..]);

        let mut buf = String::new();
        reader.read_definition(&mut buf).await?;

        assert_eq!(buf, ">sq0");

        Ok(())
    }

    #[tokio::test]
    async fn test_read_record() -> io::Result<()> {
        let data = b">sq0\nACGT\n>sq1\nNNNN\nNNNN\nNN\n";
        let mut reader = Reader::new(&data[..]);

        let mut record = Record::default();

        reader.read_record(&mut record).await?;
        assert_eq!(record.name(), "sq0");
        assert_eq!(record.sequence(), b"ACGT");

        reader.read_record(&mut record).await?;
        assert_eq!(record.name(), "sq1");
        assert_eq!(record.sequence(), b"NNNNNNNNNN");

        let n = reader.read_record(&mut record).await?;
        assert_eq!(n, 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_read_line() -> io::Result<()> {
        async fn t(buf: &mut String, mut data: &[u8], expected: &str) -> io::Result<()> {
            buf.clear();
            read_line(&mut data, buf).await?;
            assert_eq!(buf, expected);
            Ok(())
        }

        let mut buf = String::new();

        t(&mut buf, b"noodles\n", "noodles").await?;
        t(&mut buf, b"noodles\r\n", "noodles").await?;
        t(&mut buf, b"noodles", "noodles").await?;

        Ok(())
    }
}
