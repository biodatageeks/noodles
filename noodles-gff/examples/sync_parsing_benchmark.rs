use flate2::read::GzDecoder;
use noodles_gff as gff;
use std::fs::File;
use std::io::{self, BufReader};
use std::time::Instant;

fn test_original_sync() -> io::Result<(usize, std::time::Duration)> {
    println!("Testing original sync parser...");
    let file = File::open("/tmp/gencode.v38.annotation.gff3.gz")?;
    let decoder = GzDecoder::new(file);
    let mut reader = gff::io::Reader::new(BufReader::new(decoder));

    let start = Instant::now();
    let mut count = 0;

    for result in reader.record_bufs() {
        let _ = result?;
        count += 1;
        if count % 100000 == 0 {
            println!("  Processed {} records", count);
        }
    }

    let duration = start.elapsed();
    Ok((count, duration))
}

fn test_fast_sync() -> io::Result<(usize, std::time::Duration)> {
    println!("Testing fast sync parser...");
    let file = File::open("/tmp/gencode.v38.annotation.gff3.gz")?;
    let decoder = GzDecoder::new(file);
    let reader = gff::io::Reader::new(BufReader::new(decoder));

    let start = Instant::now();
    let mut count = 0;

    for result in reader.fast_records() {
        let _ = result?;
        count += 1;
        if count % 100000 == 0 {
            println!("  Processed {} records", count);
        }
    }

    let duration = start.elapsed();
    Ok((count, duration))
}

fn test_simd_sync() -> io::Result<(usize, std::time::Duration)> {
    println!("Testing SIMD sync parser...");
    let file = File::open("/tmp/gencode.v38.annotation.gff3.gz")?;
    let decoder = GzDecoder::new(file);
    let reader = gff::io::Reader::new(BufReader::new(decoder));

    let start = Instant::now();
    let mut count = 0;

    for result in reader.simd_records() {
        let _ = result?;
        count += 1;
        if count % 100000 == 0 {
            println!("  Processed {} records", count);
        }
    }

    let duration = start.elapsed();
    Ok((count, duration))
}

fn main() -> io::Result<()> {
    println!("🏁 Sync GFF Parser Raw Performance Benchmark");
    println!("=============================================");
    println!("This tests the raw parsing performance without async overhead\n");

    // Test original parser
    let (orig_count, orig_time) = test_original_sync()?;
    let orig_ns_per_record = orig_time.as_nanos() as f64 / orig_count as f64;

    println!(
        "📊 ORIGINAL SYNC: {} records in {:?} ({:.1} ns/record)\n",
        orig_count, orig_time, orig_ns_per_record
    );

    // Test fast parser
    let (fast_count, fast_time) = test_fast_sync()?;
    let fast_ns_per_record = fast_time.as_nanos() as f64 / fast_count as f64;

    println!(
        "⚡ FAST SYNC: {} records in {:?} ({:.1} ns/record)",
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
    let (simd_count, simd_time) = test_simd_sync()?;
    let simd_ns_per_record = simd_time.as_nanos() as f64 / simd_count as f64;

    println!(
        "🔥 SIMD SYNC: {} records in {:?} ({:.1} ns/record)",
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
    println!("🎯 SYNC PERFORMANCE SUMMARY:");
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

    println!("\n💡 Note: This tests raw parsing performance using sync I/O,");
    println!("   which eliminates async overhead and focuses on parser optimization.");

    Ok(())
}
