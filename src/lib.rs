use noodles::core::Region;
use noodles::vcf;
use noodles_util::variant;
use std::io;

struct QueryHolder<'r, 'h: 'r> {
    query: Box<dyn Iterator<Item = io::Result<vcf::Record>> + 'r>,
    header: &'h vcf::Header,
}

fn get_qh<'r, 'h: 'r>(
    reader: &'r mut variant::IndexedReader<std::fs::File>,
    header: &'h vcf::Header,
    region: &Region,
) -> Result<QueryHolder<'r, 'h>, Box<dyn std::error::Error>> {
    let q = reader.query(&header, &region)?;
    let qh = QueryHolder {
        query: Box::new(q),
        header: header,
    };
    Ok(qh)
}

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let vcf_path = args.next().expect("missing vcf path");
    let mut reader = variant::indexed_reader::Builder::default().build_from_path(vcf_path)?;
    let header = reader.read_header()?;

    let region = "1".parse()?;

    let mut qh = get_qh(&mut reader, &header, &region)?;
    let variant = qh.query.next().unwrap()?;
    eprintln!("{:?}", variant);

    Ok(())
}
