use async_compression::tokio::bufread::GzipDecoder;
use futures::StreamExt;
use noodles_gff as gff;
use std::io;
use std::time::Instant;
use tokio::fs::File;
use tokio::io::BufReader;

async fn test_original() -> io::Result<(usize, std::time::Duration)> {
    println!("Testing original async parser...");
    let file = File::open("/tmp/gencode.v38.annotation.gff3.gz").await?;
    let decoder = GzipDecoder::new(BufReader::new(file));
    let mut reader = gff::r#async::io::Reader::new(BufReader::new(decoder));

    let start = Instant::now();
    let mut count = 0;

    let mut stream = reader.record_bufs();
    while let Some(result) = stream.next().await {
        let _ = result?;
        count += 1;
        if count % 10000 == 0 {
            println!("  Processed {} records", count);
        }
    }

    let duration = start.elapsed();
    Ok((count, duration))
}

async fn test_fast() -> io::Result<(usize, std::time::Duration)> {
    println!("Testing fast async parser...");
    let file = File::open("/tmp/gencode.v38.annotation.gff3.gz").await?;
    let decoder = GzipDecoder::new(BufReader::new(file));
    let reader = gff::r#async::io::Reader::new(BufReader::new(decoder));

    let start = Instant::now();
    let mut count = 0;

    let mut stream = reader.fast_record_bufs();
    while let Some(result) = stream.next().await {
        let _ = result?;
        count += 1;
        if count % 10000 == 0 {
            println!("  Processed {} records", count);
        }
    }

    let duration = start.elapsed();
    Ok((count, duration))
}

async fn test_simd() -> io::Result<(usize, std::time::Duration)> {
    println!("Testing SIMD async parser...");
    let file = File::open("/tmp/gencode.v38.annotation.gff3.gz").await?;
    let decoder = GzipDecoder::new(BufReader::new(file));
    let reader = gff::r#async::io::Reader::new(BufReader::new(decoder));

    let start = Instant::now();
    let mut count = 0;

    let mut stream = reader.simd_record_bufs();
    while let Some(result) = stream.next().await {
        let _ = result?;
        count += 1;
        if count % 10000 == 0 {
            println!("  Processed {} records", count);
        }
    }

    let duration = start.elapsed();
    Ok((count, duration))
}

#[tokio::main]
async fn main() -> io::Result<()> {
    println!("🔍 Debugging Performance & Record Count Issues\n");

    // Test original parser
    let (orig_count, orig_time) = test_original().await?;
    let orig_ns_per_record = orig_time.as_nanos() as f64 / orig_count as f64;

    println!(
        "📊 ORIGINAL: {} records in {:?} ({:.1} ns/record)\n",
        orig_count, orig_time, orig_ns_per_record
    );

    // Test fast parser
    let (fast_count, fast_time) = test_fast().await?;
    let fast_ns_per_record = fast_time.as_nanos() as f64 / fast_count as f64;

    println!(
        "⚡ FAST: {} records in {:?} ({:.1} ns/record)",
        fast_count, fast_time, fast_ns_per_record
    );

    if fast_count != orig_count {
        println!(
            "❌ RECORD COUNT MISMATCH! Fast: {}, Original: {}",
            fast_count, orig_count
        );
    } else {
        println!("✅ Record counts match");
    }

    let fast_speedup = orig_ns_per_record / fast_ns_per_record;
    println!(
        "   Speedup: {:.2}x {}\n",
        fast_speedup,
        if fast_speedup > 1.0 {
            "faster"
        } else {
            "slower"
        }
    );

    // Test SIMD parser
    let (simd_count, simd_time) = test_simd().await?;
    let simd_ns_per_record = simd_time.as_nanos() as f64 / simd_count as f64;

    println!(
        "🔥 SIMD: {} records in {:?} ({:.1} ns/record)",
        simd_count, simd_time, simd_ns_per_record
    );

    if simd_count != orig_count {
        println!(
            "❌ RECORD COUNT MISMATCH! SIMD: {}, Original: {}",
            simd_count, orig_count
        );
    } else {
        println!("✅ Record counts match");
    }

    let simd_speedup = orig_ns_per_record / simd_ns_per_record;
    println!(
        "   Speedup: {:.2}x {}\n",
        simd_speedup,
        if simd_speedup > 1.0 {
            "faster"
        } else {
            "slower"
        }
    );

    // Summary
    println!("🎯 SUMMARY:");
    println!(
        "  Original: {} records, {:.1} ns/record",
        orig_count, orig_ns_per_record
    );
    println!(
        "  Fast:     {} records, {:.1} ns/record ({:.2}x)",
        fast_count, fast_ns_per_record, fast_speedup
    );
    println!(
        "  SIMD:     {} records, {:.1} ns/record ({:.2}x)",
        simd_count, simd_ns_per_record, simd_speedup
    );

    Ok(())
}
