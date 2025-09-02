use std::io;
use futures::StreamExt;
use tokio::io::BufReader;
use noodles_gff as gff;

#[tokio::main]
async fn main() -> io::Result<()> {
    println!("Testing Async RecordBuf Support");
    println!("===============================");

    // Sample GFF data with various attribute types
    let data = b"##gff-version 3
chr1\tHAVANA\tgene\t11869\t14409\t.\t+\t.\tID=ENSG00000223972.5;gene_id=ENSG00000223972.5;gene_type=transcribed_unprocessed_pseudogene;gene_name=DDX11L1;level=2
chr1\tHAVANA\texon\t11869\t12227\t.\t+\t.\tID=exon:ENST00000456328.2:1;Parent=ENST00000456328.2;gene_id=ENSG00000223972.5;transcript_id=ENST00000456328.2;exon_number=1
chr1\tHAVANA\texon\t12613\t12721\t42.5\t-\t0\tID=exon:ENST00000456328.2:2;Parent=ENST00000456328.2;gene_id=ENSG00000223972.5;transcript_id=ENST00000456328.2;exon_number=2
chr1\tHAVANA\tCDS\t1000\t2000\t.\t+\t.\tID=cds1;Parent=transcript1;Alias=alt1,alt2,alt3;ontology_term=GO:0005515,GO:0003674
";

    println!("Testing fast_record_bufs()...");
    let reader = gff::r#async::io::Reader::new(BufReader::new(&data[..]));
    let mut stream = reader.fast_record_bufs();
    let mut fast_count = 0;

    while let Some(result) = stream.next().await {
        match result {
            Ok(record) => {
                fast_count += 1;
                println!("Fast Record {}: {} {}:{}-{} ({:?})", 
                    fast_count, 
                    record.reference_sequence_name(),
                    record.ty(),
                    record.start(),
                    record.end(),
                    record.strand()
                );
                
                // Test score parsing
                if let Some(score) = record.score() {
                    println!("  Score: {}", score);
                }
                
                println!("  Attributes: {:?}", record.attributes());
            }
            Err(e) => {
                println!("Error parsing fast record: {}", e);
            }
        }
    }

    println!("\nTesting simd_record_bufs()...");
    let reader = gff::r#async::io::Reader::new(BufReader::new(&data[..]));
    let mut stream = reader.simd_record_bufs();
    let mut simd_count = 0;

    while let Some(result) = stream.next().await {
        match result {
            Ok(record) => {
                simd_count += 1;
                println!("SIMD Record {}: {} {}:{}-{} ({:?})", 
                    simd_count, 
                    record.reference_sequence_name(),
                    record.ty(),
                    record.start(),
                    record.end(),
                    record.strand()
                );
                
                // Test score parsing
                if let Some(score) = record.score() {
                    println!("  Score: {}", score);
                }
                
                println!("  Attributes: {:?}", record.attributes());
            }
            Err(e) => {
                println!("Error parsing SIMD record: {}", e);
            }
        }
    }

    println!("\nSummary:");
    println!("Fast RecordBuf count: {}", fast_count);
    println!("SIMD RecordBuf count: {}", simd_count);
    
    if fast_count == simd_count && fast_count > 0 {
        println!("✅ Both parsers produced the same number of records!");
    } else {
        println!("❌ Mismatch in record counts");
    }

    Ok(())
}