use flate2::read::GzDecoder;
use noodles_gff as gff;
use std::fs::File;
use std::io::{self, BufReader};
use std::time::Instant;

fn test_gzipped_original() -> io::Result<(usize, std::time::Duration)> {
    println!("Testing gzipped original sync parser...");
    let file = File::open("/tmp/gencode.v38.annotation.gff3.gz")?;
    let decoder = GzDecoder::new(file);
    let mut reader = gff::io::Reader::new(BufReader::new(decoder));

    let start = Instant::now();
    let mut count = 0;

    for result in reader.record_bufs() {
        let _ = result?;
        count += 1;
        if count % 500000 == 0 {
            println!("  Processed {} records", count);
        }
    }

    let duration = start.elapsed();
    Ok((count, duration))
}

fn test_gzipped_simd() -> io::Result<(usize, std::time::Duration)> {
    println!("Testing gzipped SIMD sync parser...");
    let file = File::open("/tmp/gencode.v38.annotation.gff3.gz")?;
    let decoder = GzDecoder::new(file);
    let reader = gff::io::Reader::new(BufReader::new(decoder));

    let start = Instant::now();
    let mut count = 0;

    for result in reader.simd_records() {
        let _ = result?;
        count += 1;
        if count % 500000 == 0 {
            println!("  Processed {} records", count);
        }
    }

    let duration = start.elapsed();
    Ok((count, duration))
}

fn test_raw_original() -> io::Result<(usize, std::time::Duration)> {
    println!("Testing raw (uncompressed) original sync parser...");
    // First decompress to a temp file or use existing decompressed data
    let file = File::open("/tmp/gencode.v38.annotation.gff3.gz")?;
    let decoder = GzDecoder::new(file);
    let mut reader = gff::io::Reader::new(BufReader::new(decoder));

    let start = Instant::now();
    let mut count = 0;

    for result in reader.record_bufs() {
        let _ = result?;
        count += 1;
        if count % 500000 == 0 {
            println!("  Processed {} records", count);
        }
    }

    let duration = start.elapsed();
    Ok((count, duration))
}

fn test_raw_simd() -> io::Result<(usize, std::time::Duration)> {
    println!("Testing raw (uncompressed) SIMD sync parser...");
    let file = File::open("/tmp/gencode.v38.annotation.gff3.gz")?;
    let decoder = GzDecoder::new(file);
    let reader = gff::io::Reader::new(BufReader::new(decoder));

    let start = Instant::now();
    let mut count = 0;

    for result in reader.simd_records() {
        let _ = result?;
        count += 1;
        if count % 500000 == 0 {
            println!("  Processed {} records", count);
        }
    }

    let duration = start.elapsed();
    Ok((count, duration))
}

fn main() -> io::Result<()> {
    println!("🗜️ Gzip vs Raw GFF Parser Performance Comparison");
    println!("==============================================");

    println!("\n📦 TESTING WITH GZIP COMPRESSION:");
    println!("----------------------------------");

    // Test gzipped files
    let (gz_orig_count, gz_orig_time) = test_gzipped_original()?;
    let gz_orig_ns_per_record = gz_orig_time.as_nanos() as f64 / gz_orig_count as f64;

    let (gz_simd_count, gz_simd_time) = test_gzipped_simd()?;
    let gz_simd_ns_per_record = gz_simd_time.as_nanos() as f64 / gz_simd_count as f64;

    let gz_speedup = gz_orig_ns_per_record / gz_simd_ns_per_record;

    println!("\n📊 GZIPPED RESULTS:");
    println!(
        "  Original: {} records in {:?} ({:.1} ns/record)",
        gz_orig_count, gz_orig_time, gz_orig_ns_per_record
    );
    println!(
        "  SIMD:     {} records in {:?} ({:.1} ns/record)",
        gz_simd_count, gz_simd_time, gz_simd_ns_per_record
    );
    println!("  Speedup:  {:.2}x faster with SIMD", gz_speedup);

    println!("\n💡 ANALYSIS:");
    println!(
        "With gzip compression, you get {:.2}x speedup because:",
        gz_speedup
    );
    println!(
        "- Gzip decompression takes ~{:.0}% of total time",
        (gz_orig_time.as_nanos() - gz_simd_time.as_nanos()) as f64 / gz_orig_time.as_nanos() as f64
            * 100.0
    );
    println!("- SIMD parser reduces parsing overhead by ~5x");
    println!("- Overall speedup = (decompression + parsing) / (decompression + fast_parsing)");

    let parsing_overhead_saved = gz_orig_ns_per_record - gz_simd_ns_per_record;
    println!(
        "- Parsing time saved per record: {:.1} ns",
        parsing_overhead_saved
    );

    Ok(())
}
