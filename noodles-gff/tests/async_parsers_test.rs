use futures::StreamExt;
use noodles_gff as gff;
use noodles_gff::feature::{
    RecordBuf,
    record::{Phase, Strand},
    record_buf::attributes::field::Value,
};
use std::io;
use tokio::io::BufReader;

#[tokio::test]
async fn test_fast_parser_with_real_records() -> io::Result<()> {
    let test_data = get_test_data();
    let reader = gff::r#async::io::Reader::new(BufReader::new(test_data.as_bytes()));
    let mut stream = reader.fast_record_bufs();

    let mut records: Vec<RecordBuf> = Vec::new();
    while let Some(result) = stream.next().await {
        let record = result?;
        records.push(record);
    }

    assert_eq!(records.len(), 10);

    // Test record 1: gene record
    let gene_record = &records[0];
    assert_eq!(gene_record.reference_sequence_name(), b"chr1");
    assert_eq!(gene_record.source(), b"HAVANA");
    assert_eq!(gene_record.ty(), b"gene");
    assert_eq!(gene_record.start().get(), 11869);
    assert_eq!(gene_record.end().get(), 14409);
    assert!(gene_record.score().is_none());
    assert_eq!(gene_record.strand(), Strand::Forward);
    assert!(gene_record.phase().is_none());

    // Test attributes parsing for gene
    let attributes = gene_record.attributes();
    assert!(attributes.get(b"ID").is_some());
    if let Some(Value::String(id)) = attributes.get(b"ID") {
        assert_eq!(id.as_ref() as &[u8], b"ENSG00000223972.5");
    } else {
        panic!("Expected ID to be a string");
    }

    assert!(attributes.get(b"gene_id").is_some());
    if let Some(Value::String(gene_id)) = attributes.get(b"gene_id") {
        assert_eq!(gene_id.as_ref() as &[u8], b"ENSG00000223972.5");
    } else {
        panic!("Expected gene_id to be a string");
    }

    assert!(attributes.get(b"gene_type").is_some());
    if let Some(Value::String(gene_type)) = attributes.get(b"gene_type") {
        assert_eq!(
            gene_type.as_ref() as &[u8],
            b"transcribed_unprocessed_pseudogene"
        );
    } else {
        panic!("Expected gene_type to be a string");
    }

    // Test record 3: exon with score
    let exon_record = &records[2];
    assert_eq!(exon_record.reference_sequence_name(), b"chr1");
    assert_eq!(exon_record.source(), b"HAVANA");
    assert_eq!(exon_record.ty(), b"exon");
    assert_eq!(exon_record.start().get(), 12613);
    assert_eq!(exon_record.end().get(), 12721);
    assert!(exon_record.score().is_some());
    assert_eq!(exon_record.strand(), Strand::Reverse);
    assert_eq!(exon_record.phase(), Some(Phase::Zero));

    // Test record 4: CDS with array attributes
    let cds_record = &records[3];
    let cds_attributes = cds_record.attributes();

    // Test Alias array parsing
    if let Some(Value::Array(alias_values)) = cds_attributes.get(b"Alias") {
        assert_eq!(alias_values.len(), 3);
        assert_eq!(alias_values[0].as_ref() as &[u8], b"alt1");
        assert_eq!(alias_values[1].as_ref() as &[u8], b"alt2");
        assert_eq!(alias_values[2].as_ref() as &[u8], b"alt3");
    } else {
        panic!("Expected Alias to be an array");
    }

    // Test ontology_term array parsing
    if let Some(Value::Array(ontology_values)) = cds_attributes.get(b"ontology_term") {
        assert_eq!(ontology_values.len(), 2);
        assert_eq!(ontology_values[0].as_ref() as &[u8], b"GO:0005515");
        assert_eq!(ontology_values[1].as_ref() as &[u8], b"GO:0003674");
    } else {
        panic!("Expected ontology_term to be an array");
    }

    Ok(())
}

#[tokio::test]
async fn test_simd_parser_with_real_records() -> io::Result<()> {
    let test_data = get_test_data();
    let reader = gff::r#async::io::Reader::new(BufReader::new(test_data.as_bytes()));
    let mut stream = reader.simd_record_bufs();

    let mut records: Vec<RecordBuf> = Vec::new();
    while let Some(result) = stream.next().await {
        let record = result?;
        records.push(record);
    }

    assert_eq!(records.len(), 10);

    // Test record 1: gene record
    let gene_record = &records[0];
    assert_eq!(gene_record.reference_sequence_name(), b"chr1");
    assert_eq!(gene_record.source(), b"HAVANA");
    assert_eq!(gene_record.ty(), b"gene");
    assert_eq!(gene_record.start().get(), 11869);
    assert_eq!(gene_record.end().get(), 14409);
    assert!(gene_record.score().is_none());
    assert_eq!(gene_record.strand(), Strand::Forward);
    assert!(gene_record.phase().is_none());

    // Test attributes parsing for gene
    let attributes = gene_record.attributes();
    assert!(attributes.get(b"ID").is_some());
    if let Some(Value::String(id)) = attributes.get(b"ID") {
        assert_eq!(id.as_ref() as &[u8], b"ENSG00000223972.5");
    } else {
        panic!("Expected ID to be a string");
    }

    assert!(attributes.get(b"gene_id").is_some());
    if let Some(Value::String(gene_id)) = attributes.get(b"gene_id") {
        assert_eq!(gene_id.as_ref() as &[u8], b"ENSG00000223972.5");
    } else {
        panic!("Expected gene_id to be a string");
    }

    // Test record 4: CDS with array attributes
    let cds_record = &records[3];
    let cds_attributes = cds_record.attributes();

    // Test Alias array parsing
    if let Some(Value::Array(alias_values)) = cds_attributes.get(b"Alias") {
        assert_eq!(alias_values.len(), 3);
        assert_eq!(alias_values[0].as_ref() as &[u8], b"alt1");
        assert_eq!(alias_values[1].as_ref() as &[u8], b"alt2");
        assert_eq!(alias_values[2].as_ref() as &[u8], b"alt3");
    } else {
        panic!("Expected Alias to be an array");
    }

    // Test ontology_term array parsing
    if let Some(Value::Array(ontology_values)) = cds_attributes.get(b"ontology_term") {
        assert_eq!(ontology_values.len(), 2);
        assert_eq!(ontology_values[0].as_ref() as &[u8], b"GO:0005515");
        assert_eq!(ontology_values[1].as_ref() as &[u8], b"GO:0003674");
    } else {
        panic!("Expected ontology_term to be an array");
    }

    Ok(())
}

#[tokio::test]
async fn test_parser_consistency() -> io::Result<()> {
    let test_data = get_test_data();

    // Parse with fast parser
    let reader = gff::r#async::io::Reader::new(BufReader::new(test_data.as_bytes()));
    let mut fast_stream = reader.fast_record_bufs();
    let mut fast_records: Vec<RecordBuf> = Vec::new();
    while let Some(result) = fast_stream.next().await {
        fast_records.push(result?);
    }

    // Parse with SIMD parser
    let reader = gff::r#async::io::Reader::new(BufReader::new(test_data.as_bytes()));
    let mut simd_stream = reader.simd_record_bufs();
    let mut simd_records: Vec<RecordBuf> = Vec::new();
    while let Some(result) = simd_stream.next().await {
        simd_records.push(result?);
    }

    // Verify same number of records
    assert_eq!(fast_records.len(), simd_records.len());

    // Compare each record field by field
    for (i, (fast_record, simd_record)) in fast_records.iter().zip(simd_records.iter()).enumerate()
    {
        // Basic fields
        assert_eq!(
            fast_record.reference_sequence_name(),
            simd_record.reference_sequence_name(),
            "Record {} reference sequence name mismatch",
            i
        );
        assert_eq!(
            fast_record.source(),
            simd_record.source(),
            "Record {} source mismatch",
            i
        );
        assert_eq!(
            fast_record.ty(),
            simd_record.ty(),
            "Record {} type mismatch",
            i
        );
        assert_eq!(
            fast_record.start(),
            simd_record.start(),
            "Record {} start mismatch",
            i
        );
        assert_eq!(
            fast_record.end(),
            simd_record.end(),
            "Record {} end mismatch",
            i
        );
        assert_eq!(
            fast_record.score(),
            simd_record.score(),
            "Record {} score mismatch",
            i
        );
        assert_eq!(
            fast_record.strand(),
            simd_record.strand(),
            "Record {} strand mismatch",
            i
        );
        assert_eq!(
            fast_record.phase(),
            simd_record.phase(),
            "Record {} phase mismatch",
            i
        );

        // Attributes
        let fast_attrs = fast_record.attributes();
        let simd_attrs = simd_record.attributes();

        assert_eq!(
            fast_attrs.len(),
            simd_attrs.len(),
            "Record {} attributes length mismatch",
            i
        );

        // Compare attributes using the as_ref() method to access the IndexMap
        use indexmap::IndexMap;
        let fast_map: &IndexMap<_, _> = fast_attrs.as_ref();
        let simd_map: &IndexMap<_, _> = simd_attrs.as_ref();

        for (key, fast_value) in fast_map {
            let simd_value = simd_attrs.get(key).expect(&format!(
                "Record {} missing attribute {}",
                i,
                std::str::from_utf8(key).unwrap()
            ));

            match (fast_value, simd_value) {
                (Value::String(fast_str), Value::String(simd_str)) => {
                    assert_eq!(
                        fast_str.as_ref() as &[u8],
                        simd_str.as_ref() as &[u8],
                        "Record {} attribute {} string value mismatch",
                        i,
                        std::str::from_utf8(key).unwrap()
                    );
                }
                (Value::Array(fast_arr), Value::Array(simd_arr)) => {
                    assert_eq!(
                        fast_arr.len(),
                        simd_arr.len(),
                        "Record {} attribute {} array length mismatch",
                        i,
                        std::str::from_utf8(key).unwrap()
                    );
                    for (j, (fast_item, simd_item)) in
                        fast_arr.iter().zip(simd_arr.iter()).enumerate()
                    {
                        assert_eq!(
                            fast_item.as_ref() as &[u8],
                            simd_item.as_ref() as &[u8],
                            "Record {} attribute {} array item {} mismatch",
                            i,
                            std::str::from_utf8(key).unwrap(),
                            j
                        );
                    }
                }
                _ => panic!(
                    "Record {} attribute {} type mismatch between parsers",
                    i,
                    std::str::from_utf8(key).unwrap()
                ),
            }
        }
    }

    Ok(())
}

fn get_test_data() -> &'static str {
    r#"##gff-version 3
chr1	HAVANA	gene	11869	14409	.	+	.	ID=ENSG00000223972.5;gene_id=ENSG00000223972.5;gene_type=transcribed_unprocessed_pseudogene;gene_name=DDX11L1;level=2
chr1	HAVANA	exon	11869	12227	.	+	.	ID=exon:ENST00000456328.2:1;Parent=ENST00000456328.2;gene_id=ENSG00000223972.5;transcript_id=ENST00000456328.2;exon_number=1
chr1	HAVANA	exon	12613	12721	42.5	-	0	ID=exon:ENST00000456328.2:2;Parent=ENST00000456328.2;gene_id=ENSG00000223972.5;transcript_id=ENST00000456328.2;exon_number=2
chr1	HAVANA	CDS	1000	2000	.	+	.	ID=cds1;Parent=transcript1;Alias=alt1,alt2,alt3;ontology_term=GO:0005515,GO:0003674
chr1	HAVANA	transcript	11869	14409	.	+	.	ID=ENST00000456328.2;Parent=ENSG00000223972.5;gene_id=ENSG00000223972.5;transcript_id=ENST00000456328.2;gene_name=DDX11L1;gene_type=processed_transcript;transcript_name=DDX11L1-202;level=2;transcript_support_level=1;hgnc_id=HGNC:37102;tag=basic,Ensembl_canonical;havana_gene=OTTHUMG00000000961.2;havana_transcript=OTTHUMT00000362751.1
chr1	HAVANA	start_codon	69091	69093	.	+	0	ID=start_codon:ENST00000335137.3:1;Parent=ENST00000335137.3;exon_id=ENSE00002319515.1;gene_id=ENSG00000186092.4;transcript_id=ENST00000335137.3;exon_number=1
chr1	HAVANA	stop_codon	70008	70010	.	+	0	ID=stop_codon:ENST00000335137.3:1;Parent=ENST00000335137.3;exon_id=ENSE00001948541.1;gene_id=ENSG00000186092.4;transcript_id=ENST00000335137.3;exon_number=6
chr1	HAVANA	five_prime_UTR	65419	65433	.	+	.	ID=five_prime_utr:ENST00000335137.3:1;Parent=ENST00000335137.3;gene_id=ENSG00000186092.4;transcript_id=ENST00000335137.3
chr1	HAVANA	three_prime_UTR	70011	70108	.	+	.	ID=three_prime_utr:ENST00000335137.3:1;Parent=ENST00000335137.3;gene_id=ENSG00000186092.4;transcript_id=ENST00000335137.3
chr1	ensembl_havana	gene	1020000	1056118	.	-	.	ID=ENSG00000228794.8;gene_id=ENSG00000228794.8;gene_type=lncRNA;gene_name=RP11-465B22.8;level=2;hgnc_id=HGNC:42746;havana_gene=OTTHUMG00000001047.4
"#
}
