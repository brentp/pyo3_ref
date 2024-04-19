use rust_htslib::bcf::{self, header::HeaderView, Header, Read};

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut reader = bcf::Reader::from_path("test.vcf.gz")?;
    let h = reader.header(); // this is a header, but I need a HeaderView

    // can't do this because we get double free core dump.
    //let hv = HeaderView::new(h.inner);

    // try this? also fails.
    //let mut inner = unsafe { *h.inner };
    //let hv = HeaderView::new(&mut inner);

    // so we have to do this?
    let hv = HeaderView::new(unsafe { rust_htslib::htslib::bcf_hdr_dup(h.inner) });

    let samples = vec!["32049-32049"]
        .into_iter()
        .map(|x| x.as_bytes())
        .collect::<Vec<_>>();
    let header = bcf::Header::from_template_subset(&hv, &samples)?;

    let mut writer = bcf::Writer::from_path("output.vcf.gz", &header, false, bcf::Format::Vcf)?;
    eprintln!(
        "writer header n samples: {}",
        writer.header().sample_count()
    );

    // this sets n-samples to 0. not sure how to call it correctly.
    /*
    let name_pointers: Vec<_> = samples
        .iter()
        .map(|s| s.as_ptr() as *const std::ffi::c_char)
        .collect();
    unsafe {
        rust_htslib::htslib::bcf_hdr_set_samples(
            h.inner,
            name_pointers.as_ptr() as *const std::ffi::c_char,
            0,
        )
    };
    */

    for record in reader.records() {
        let mut record = record?;
        eprintln!("record n samples: {}", record.sample_count());
        unsafe {
            rust_htslib::htslib::bcf_subset_format(writer.header().inner, record.inner);
        }
        writer.translate(&mut record);
        eprintln!("record n samples: {}", record.sample_count());
        writer.write(&record)?;
    }
    Ok(())
}
