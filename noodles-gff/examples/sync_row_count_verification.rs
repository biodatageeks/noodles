use flate2::read::GzDecoder;
use noodles_gff as gff;
use std::fs::File;
use std::io::{self, BufReader};

fn test_original_sync() -> io::Result<usize> {
    println!("Counting records with original sync parser...");
    let file = File::open("/tmp/gencode.v38.annotation.gff3.gz")?;
    let decoder = GzDecoder::new(file);
    let mut reader = gff::io::Reader::new(BufReader::new(decoder));

    let mut count = 0;

    for result in reader.record_bufs() {
        let _ = result?;
        count += 1;
        if count % 500000 == 0 {
            println!("  Original: {} records so far", count);
        }
    }

    println!("  Original: {} total records", count);
    Ok(count)
}

fn test_fast_sync() -> io::Result<usize> {
    println!("Counting records with fast sync parser...");
    let file = File::open("/tmp/gencode.v38.annotation.gff3.gz")?;
    let decoder = GzDecoder::new(file);
    let reader = gff::io::Reader::new(BufReader::new(decoder));

    let mut count = 0;

    for result in reader.fast_records() {
        let _ = result?;
        count += 1;
        if count % 500000 == 0 {
            println!("  Fast: {} records so far", count);
        }
    }

    println!("  Fast: {} total records", count);
    Ok(count)
}

fn test_simd_sync() -> io::Result<usize> {
    println!("Counting records with SIMD sync parser...");
    let file = File::open("/tmp/gencode.v38.annotation.gff3.gz")?;
    let decoder = GzDecoder::new(file);
    let reader = gff::io::Reader::new(BufReader::new(decoder));

    let mut count = 0;

    for result in reader.simd_records() {
        let _ = result?;
        count += 1;
        if count % 500000 == 0 {
            println!("  SIMD: {} records so far", count);
        }
    }

    println!("  SIMD: {} total records", count);
    Ok(count)
}

fn main() -> io::Result<()> {
    println!("✅ Sync Row Count Verification for Gzipped GFF");
    println!("============================================");
    println!("Testing all three sync parsers with the same gzipped file\n");

    // Test all three parsers
    let original_count = test_original_sync()?;
    println!();
    let fast_count = test_fast_sync()?;
    println!();
    let simd_count = test_simd_sync()?;

    // Compare results
    println!("\n🎯 RECORD COUNT COMPARISON:");
    println!("===========================");
    println!("  Original: {:>10} records", original_count);
    println!("  Fast:     {:>10} records", fast_count);
    println!("  SIMD:     {:>10} records", simd_count);

    // Check for matches
    println!("\n🔍 VERIFICATION:");
    if fast_count == original_count {
        println!("  ✅ Fast parser count matches original");
    } else {
        println!(
            "  ❌ Fast parser count MISMATCH! Difference: {}",
            (fast_count as i32) - (original_count as i32)
        );
    }

    if simd_count == original_count {
        println!("  ✅ SIMD parser count matches original");
    } else {
        println!(
            "  ❌ SIMD parser count MISMATCH! Difference: {}",
            (simd_count as i32) - (original_count as i32)
        );
    }

    if fast_count == simd_count {
        println!("  ✅ Fast and SIMD counts match each other");
    } else {
        println!(
            "  ❌ Fast and SIMD counts don't match! Difference: {}",
            (fast_count as i32) - (simd_count as i32)
        );
    }

    // Final summary
    if original_count == fast_count && fast_count == simd_count {
        println!("\n🎉 SUCCESS: All parsers return exactly the same record count!");
        println!(
            "   Perfect accuracy: {} records processed by all parsers",
            original_count
        );
    } else {
        println!("\n⚠️  WARNING: Record count mismatches detected!");
        println!("   This indicates potential data loss or parsing errors.");
    }

    Ok(())
}
