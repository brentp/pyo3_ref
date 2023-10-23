use noodles::core::Region;
use noodles::vcf;
use noodles_util::variant;
use std::io;

struct QueryHolder /* <'a: 'h> */ {
    vcf: variant::indexed_reader::IndexedReader<std::fs::File>,
    query: Box<dyn Iterator<Item = io::Result<vcf::Record>>>,
    // header: &'h vcf::Header,
}

fn get_qh(vcf_path: String, region: &Region) -> Result<QueryHolder, Box<dyn std::error::Error>> {
    let mut reader = variant::indexed_reader::Builder::default().build_from_path(vcf_path)?;
    let header = reader.read_header()?;

    let q = reader.query(&header, &region)?;
    let qh = QueryHolder {
        vcf: reader,
        query: Box::new(q),
    };
    Ok(qh)
}

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let vcf_path = args.next().expect("missing vcf path");

    let region = "chr1".parse()?;

    let qh = get_qh(vcf_path, &region)?;

    Ok(())
}
