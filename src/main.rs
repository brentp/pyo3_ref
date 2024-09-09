use std::sync::Arc;

use rust_htslib::bcf::{self, Record};
use rust_htslib::bcf::record::GenotypeAllele;
use rusty_v8 as v8;

struct Variant {
    record: rust_htslib::bcf::Record,
}

impl Variant {
    fn new(record: rust_htslib::bcf::Record) -> Self {
        Self { record }
    }

    fn start(&self) -> i64 {
        self.record.pos() as i64
    }
}

fn create_vcf_record() -> Result<Record, Box<dyn std::error::Error>> {
    let mut header = bcf::Header::new();
    header.push_record(r#"##contig=<ID=chr1,length=10000>"#.as_bytes());
    header.push_record(
        r#"##FORMAT=<ID=GT,Number=1,Type=String,Description="Genotype">"#.as_bytes(),
    );
    header.push_record(r#"##FILTER=<ID=PASS,Description="All filters passed">"#.as_bytes());
    header.push_record(
        r#"##INFO=<ID=DP,Number=1,Type=Integer,Description="Total Depth">"#.as_bytes(),
    );
    header.push_sample("NA12878".as_bytes());
    header.push_sample("NA12879".as_bytes());
    let vcf = bcf::Writer::from_path("_test.vcf", &header, true, bcf::Format::Vcf).unwrap();
    let mut record = vcf.empty_record();
    let _ = record.set_rid(Some(vcf.header().name2rid(b"chr1").unwrap()));
    record.set_pos(6);
    record.set_alleles(&[b"A", b"T"]).unwrap();
    record.set_id(b"rs1234").unwrap();
    record.set_filters(&["PASS".as_bytes()]).unwrap();
    record.push_info_integer(b"DP", &[10]).unwrap();
    let alleles = &[
        GenotypeAllele::Unphased(0),
        GenotypeAllele::Phased(1),
        GenotypeAllele::Unphased(1),
        GenotypeAllele::Unphased(1),
    ];
    record.push_genotypes(alleles).unwrap();
    Ok(record)
}

fn create_variant_object<'a>(
    scope: &mut v8::HandleScope<'a>,
    variant: Arc<Variant>,
) -> v8::Local<'a, v8::Object> {
    let object_template = v8::ObjectTemplate::new(scope);
    object_template.set_internal_field_count(1);

    // Fix the start_getter function
    let start_name = v8::String::new(scope, "start").unwrap();
    let start_getter = |scope: &mut v8::HandleScope,
                        _property: v8::Local<v8::Name>,
                        args: v8::PropertyCallbackArguments,
                        mut retval: v8::ReturnValue| {
        let this = args.this();
        let external = unsafe { v8::Local::<v8::External>::cast(this.get_internal_field(scope, 0).unwrap()) };
        let variant_ptr = external.value() as *const Variant;
        let variant = unsafe { &*variant_ptr };

        let start = variant.start();
        let start_value = v8::Integer::new(scope, start as i32);
        retval.set(start_value.into());
    };
    object_template.set_accessor(start_name.into(), start_getter);

    let object = object_template.new_instance(scope).unwrap();
    let external_variant = v8::External::new(scope, Arc::into_raw(variant) as *mut _);
    object.set_internal_field(0, external_variant.into());

    object
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize V8
    let platform = v8::new_default_platform(0, false).make_shared();
    v8::V8::initialize_platform(platform);
    v8::V8::initialize();

    let isolate = &mut v8::Isolate::new(v8::CreateParams::default());
    let handle_scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(handle_scope);
    let scope = &mut v8::ContextScope::new(handle_scope, context);

    let record = create_vcf_record()?;
    let variant = Arc::new(Variant::new(record));
    let variant_object = create_variant_object(scope, variant);

    // Add the variant object to the global object
    let global = context.global(scope);
    let variant_name = v8::String::new(scope, "variant").unwrap();
    global.set(scope, variant_name.into(), variant_object.into());

    // Execute JavaScript code
    let source = v8::String::new(scope, "variant.start").unwrap();
    let script = v8::Script::compile(scope, source, None).unwrap();
    let result = script.run(scope).unwrap();

    println!("Result: {:?}", result.to_string(scope).unwrap().to_rust_string_lossy(scope));

    Ok(())
}


