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

// Skip zero-copy for now due to lifetime complexity

fn benchmark_simd() -> io::Result<(usize, std::time::Duration)> {
    let file = File::open("/tmp/gencode.v38.annotation.gff3.gz")?;
    let decoder = GzDecoder::new(file);
    let reader = gff::io::Reader::new(BufReader::new(decoder));

    let start = std::time::Instant::now();
    let mut count = 0;

    for result in reader.simd_records() {
        let _ = result?;
        count += 1;
    }

    let duration = start.elapsed();
    Ok((count, duration))
}

fn main() -> io::Result<()> {
    println!("🚀 Ultimate GFF Parser Benchmark 🚀");
    println!("====================================");
    
    // Warmup
    let _ = benchmark_original()?;
    let _ = benchmark_fast()?;
    let _ = benchmark_simd()?;

    println!("Running benchmarks...\n");

    // Original parser
    let (original_count, original_duration) = benchmark_original()?;
    let original_ns_per_record = original_duration.as_nanos() as f64 / original_count as f64;

    println!("📊 ORIGINAL PARSER (baseline)");
    println!("Records: {}", original_count);
    println!("Time: {:?}", original_duration);
    println!("ns/record: {:.1}", original_ns_per_record);
    println!("records/sec: {:.0}\n", original_count as f64 / original_duration.as_secs_f64());

    // Fast parser
    let (fast_count, fast_duration) = benchmark_fast()?;
    let fast_ns_per_record = fast_duration.as_nanos() as f64 / fast_count as f64;
    let fast_speedup = original_ns_per_record / fast_ns_per_record;

    println!("⚡ FAST PARSER");
    println!("Records: {}", fast_count);
    println!("Time: {:?}", fast_duration);
    println!("ns/record: {:.1}", fast_ns_per_record);
    println!("records/sec: {:.0}", fast_count as f64 / fast_duration.as_secs_f64());
    println!("Speedup: {:.2}x\n", fast_speedup);

    // Skip zero-copy parser due to lifetime complexity

    // SIMD parser
    let (simd_count, simd_duration) = benchmark_simd()?;
    let simd_ns_per_record = simd_duration.as_nanos() as f64 / simd_count as f64;
    let simd_speedup = original_ns_per_record / simd_ns_per_record;

    println!("🔥 SIMD PARSER");
    println!("Records: {}", simd_count);
    println!("Time: {:?}", simd_duration);
    println!("ns/record: {:.1}", simd_ns_per_record);
    println!("records/sec: {:.0}", simd_count as f64 / simd_duration.as_secs_f64());
    println!("Speedup: {:.2}x\n", simd_speedup);

    // Summary
    println!("🏆 PERFORMANCE SUMMARY");
    println!("====================");
    
    let mut results = vec![
        ("Original", original_ns_per_record, 1.0),
        ("Fast", fast_ns_per_record, fast_speedup),
        ("SIMD", simd_ns_per_record, simd_speedup),
    ];
    
    // Sort by performance (lowest ns/record first)
    results.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    
    println!("Ranking (fastest to slowest):");
    for (i, (name, ns_per_record, speedup)) in results.iter().enumerate() {
        println!("{}. {} - {:.1} ns/record ({:.2}x)", i + 1, name, ns_per_record, speedup);
    }

    Ok(())
}