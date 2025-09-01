use std::fs::File;
use std::io::{self, BufReader};

use flate2::read::GzDecoder;
use noodles_gff as gff;

fn benchmark_original() -> io::Result<(usize, std::time::Duration)> {
    let file = File::open("/tmp/gencode.v38.annotation.gff3.gz")?;
    let decoder = GzDecoder::new(file);
    let mut reader = gff::io::Reader::new(BufReader::new(decoder));

    let start = std::time::Instant::now();
    let mut count = 0;

    for result in reader.record_bufs() {
        let _ = result?;
        count += 1;
    }

    let duration = start.elapsed();
    Ok((count, duration))
}

fn benchmark_fast() -> io::Result<(usize, std::time::Duration)> {
    let file = File::open("/tmp/gencode.v38.annotation.gff3.gz")?;
    let decoder = GzDecoder::new(file);
    let reader = gff::io::Reader::new(BufReader::new(decoder));

    let start = std::time::Instant::now();
    let mut count = 0;

    for result in reader.fast_records() {
        let _ = result?;
        count += 1;
    }

    let duration = start.elapsed();
    Ok((count, duration))
}

fn main() -> io::Result<()> {
    println!("Benchmarking GFF parsers...");
    
    // Warmup
    let _ = benchmark_original()?;
    let _ = benchmark_fast()?;

    // Original parser
    let (original_count, original_duration) = benchmark_original()?;
    let original_records_per_sec = original_count as f64 / original_duration.as_secs_f64();
    let original_ns_per_record = original_duration.as_nanos() as f64 / original_count as f64;

    println!("=== ORIGINAL PARSER ===");
    println!("Records parsed: {}", original_count);
    println!("Time taken: {:?}", original_duration);
    println!("Records/sec: {:.0}", original_records_per_sec);
    println!("Nanoseconds/record: {:.1}", original_ns_per_record);

    // Fast parser
    let (fast_count, fast_duration) = benchmark_fast()?;
    let fast_records_per_sec = fast_count as f64 / fast_duration.as_secs_f64();
    let fast_ns_per_record = fast_duration.as_nanos() as f64 / fast_count as f64;

    println!("\n=== FAST PARSER ===");
    println!("Records parsed: {}", fast_count);
    println!("Time taken: {:?}", fast_duration);
    println!("Records/sec: {:.0}", fast_records_per_sec);
    println!("Nanoseconds/record: {:.1}", fast_ns_per_record);

    // Comparison
    let speedup = original_ns_per_record / fast_ns_per_record;
    println!("\n=== COMPARISON ===");
    println!("Speedup: {:.2}x", speedup);
    
    if fast_ns_per_record < original_ns_per_record {
        let improvement = ((original_ns_per_record - fast_ns_per_record) / original_ns_per_record) * 100.0;
        println!("Performance improvement: {:.1}%", improvement);
    } else {
        let regression = ((fast_ns_per_record - original_ns_per_record) / original_ns_per_record) * 100.0;
        println!("Performance regression: {:.1}%", regression);
    }

    Ok(())
}