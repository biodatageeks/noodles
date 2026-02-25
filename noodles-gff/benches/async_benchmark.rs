use std::io;

use futures::StreamExt;
use tokio::fs::File;
use tokio::io::BufReader;
use tokio_util::codec::{FramedRead, LinesCodec};

use noodles_gff as gff;

/// Async benchmark using the original async parser
async fn benchmark_async_original() -> io::Result<(usize, std::time::Duration)> {
    let file = File::open("/tmp/gencode.v38.annotation.gff3.gz").await?;
    let decoder = async_compression::tokio::bufread::GzipDecoder::new(BufReader::new(file));
    let mut reader = gff::r#async::io::Reader::new(BufReader::new(decoder));

    let start = std::time::Instant::now();
    let mut count = 0;

    let mut stream = reader.record_bufs();
    while let Some(result) = stream.next().await {
        let _ = result?;
        count += 1;
    }

    let duration = start.elapsed();
    Ok((count, duration))
}

/// Async benchmark using the fast parser
async fn benchmark_async_fast() -> io::Result<(usize, std::time::Duration)> {
    let file = File::open("/tmp/gencode.v38.annotation.gff3.gz").await?;
    let decoder = async_compression::tokio::bufread::GzipDecoder::new(BufReader::new(file));
    let reader = gff::r#async::io::Reader::new(BufReader::new(decoder));

    let start = std::time::Instant::now();
    let mut count = 0;

    let mut stream = reader.fast_records();
    while let Some(result) = stream.next().await {
        let _ = result?;
        count += 1;
    }

    let duration = start.elapsed();
    Ok((count, duration))
}

/// Async benchmark using the SIMD parser
async fn benchmark_async_simd() -> io::Result<(usize, std::time::Duration)> {
    let file = File::open("/tmp/gencode.v38.annotation.gff3.gz").await?;
    let decoder = async_compression::tokio::bufread::GzipDecoder::new(BufReader::new(file));
    let reader = gff::r#async::io::Reader::new(BufReader::new(decoder));

    let start = std::time::Instant::now();
    let mut count = 0;

    let mut stream = reader.simd_records();
    while let Some(result) = stream.next().await {
        let _ = result?;
        count += 1;
    }

    let duration = start.elapsed();
    Ok((count, duration))
}

/// Raw async line reading benchmark for comparison
async fn benchmark_raw_lines() -> io::Result<(usize, std::time::Duration)> {
    let file = File::open("/tmp/gencode.v38.annotation.gff3.gz").await?;
    let decoder = async_compression::tokio::bufread::GzipDecoder::new(BufReader::new(file));
    let mut lines = FramedRead::new(decoder, LinesCodec::new());

    let start = std::time::Instant::now();
    let mut count = 0;

    while let Some(result) = lines.next().await {
        let line = result.map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        // Skip comments and directives (same logic as parsers)
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        count += 1;
    }

    let duration = start.elapsed();
    Ok((count, duration))
}

#[tokio::main]
async fn main() -> io::Result<()> {
    println!("🚀 Async GFF Parser Ultimate Benchmark 🚀");
    println!("==========================================");

    // Warmup runs
    println!("Warming up...");
    let _ = benchmark_raw_lines().await?;
    let _ = benchmark_async_original().await?;
    let _ = benchmark_async_fast().await?;
    let _ = benchmark_async_simd().await?;

    println!("Running async benchmarks...\n");

    // Raw line reading (theoretical maximum)
    let (raw_count, raw_duration) = benchmark_raw_lines().await?;
    let raw_ns_per_record = raw_duration.as_nanos() as f64 / raw_count as f64;

    println!("🔥 RAW LINE READING (theoretical max)");
    println!("Records: {}", raw_count);
    println!("Time: {:?}", raw_duration);
    println!("ns/record: {:.1}", raw_ns_per_record);
    println!(
        "records/sec: {:.0}\n",
        raw_count as f64 / raw_duration.as_secs_f64()
    );

    // Original async parser
    let (original_count, original_duration) = benchmark_async_original().await?;
    let original_ns_per_record = original_duration.as_nanos() as f64 / original_count as f64;

    println!("📊 ORIGINAL ASYNC PARSER");
    println!("Records: {}", original_count);
    println!("Time: {:?}", original_duration);
    println!("ns/record: {:.1}", original_ns_per_record);
    println!(
        "records/sec: {:.0}",
        original_count as f64 / original_duration.as_secs_f64()
    );
    println!(
        "vs Raw: {:.2}x slower\n",
        original_ns_per_record / raw_ns_per_record
    );

    // Fast async parser
    let (fast_count, fast_duration) = benchmark_async_fast().await?;
    let fast_ns_per_record = fast_duration.as_nanos() as f64 / fast_count as f64;
    let fast_speedup = original_ns_per_record / fast_ns_per_record;

    println!("⚡ ASYNC FAST PARSER");
    println!("Records: {}", fast_count);
    println!("Time: {:?}", fast_duration);
    println!("ns/record: {:.1}", fast_ns_per_record);
    println!(
        "records/sec: {:.0}",
        fast_count as f64 / fast_duration.as_secs_f64()
    );
    println!("vs Original: {:.2}x faster", fast_speedup);
    println!(
        "vs Raw: {:.2}x slower\n",
        fast_ns_per_record / raw_ns_per_record
    );

    // SIMD async parser
    let (simd_count, simd_duration) = benchmark_async_simd().await?;
    let simd_ns_per_record = simd_duration.as_nanos() as f64 / simd_count as f64;
    let simd_speedup = original_ns_per_record / simd_ns_per_record;

    println!("🔥 ASYNC SIMD PARSER");
    println!("Records: {}", simd_count);
    println!("Time: {:?}", simd_duration);
    println!("ns/record: {:.1}", simd_ns_per_record);
    println!(
        "records/sec: {:.0}",
        simd_count as f64 / simd_duration.as_secs_f64()
    );
    println!("vs Original: {:.2}x faster", simd_speedup);
    println!(
        "vs Raw: {:.2}x slower\n",
        simd_ns_per_record / raw_ns_per_record
    );

    // Summary
    println!("🏆 ASYNC PERFORMANCE SUMMARY");
    println!("=============================");

    let mut results = vec![
        (
            "Raw Lines",
            raw_ns_per_record,
            raw_ns_per_record / raw_ns_per_record,
        ),
        (
            "Original Async",
            original_ns_per_record,
            original_ns_per_record / raw_ns_per_record,
        ),
        (
            "Fast Async",
            fast_ns_per_record,
            fast_ns_per_record / raw_ns_per_record,
        ),
        (
            "SIMD Async",
            simd_ns_per_record,
            simd_ns_per_record / raw_ns_per_record,
        ),
    ];

    // Sort by performance (lowest ns/record first)
    results.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    println!("Ranking (fastest to slowest):");
    for (i, (name, ns_per_record, slowdown)) in results.iter().enumerate() {
        if name.contains("Raw") {
            println!(
                "{}. {} - {:.1} ns/record (baseline)",
                i + 1,
                name,
                ns_per_record
            );
        } else {
            println!(
                "{}. {} - {:.1} ns/record ({:.2}x slower than raw)",
                i + 1,
                name,
                ns_per_record,
                slowdown
            );
        }
    }

    println!("\n💡 Key Insights:");
    println!("- Raw line reading sets the theoretical maximum performance");
    println!("- Async adds overhead but enables concurrent processing");
    println!("- SIMD optimizations help even in async contexts");
    println!("- Fast parsers can process millions of records/second asynchronously");

    Ok(())
}
