use async_compression::tokio::bufread::GzipDecoder;
use futures::StreamExt;
use noodles_gff as gff;
use std::io;
use tokio::fs::File;
use tokio::io::BufReader;

#[tokio::main]
async fn main() -> io::Result<()> {
    let file = File::open("/tmp/gencode.v38.annotation.gff3.gz").await?;
    let decoder = GzipDecoder::new(BufReader::new(file));
    let reader = gff::r#async::io::Reader::new(BufReader::new(decoder));

    let mut stream = reader.simd_record_bufs();
    let mut count = 0;
    let mut valid_records = 0;

    println!("Testing line splitting fix with real gzipped GFF file...");

    while let Some(result) = stream.next().await {
        count += 1;
        match result {
            Ok(_record) => {
                valid_records += 1;
            }
            Err(e) => {
                println!("Error parsing record {}: {}", count, e);
            }
        }

        if count >= 100 {
            // Test first 100 records
            break;
        }
    }

    println!("Processed {} records, {} valid", count, valid_records);
    println!(
        "Success rate: {:.1}%",
        (valid_records as f64 / count as f64) * 100.0
    );

    Ok(())
}
