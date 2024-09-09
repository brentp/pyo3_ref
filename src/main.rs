use std::sync::Arc;

use rust_htslib::bcf::{self, Record};
use rust_htslib::bcf::record::GenotypeAllele;
use v8;

// see: https://github.com/denoland/deno/blob/41cad2179fb36c2371ab84ce587d3460af64b5fb/ext/napi/lib.rs#L522-L527

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

fn create_header() -> bcf::Header {
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
    header
}

fn create_vcf_record(_header: &bcf::Header, vcf: &bcf::Writer) -> Result<Record, Box<dyn std::error::Error>> {
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

impl Drop for Variant {
    fn drop(&mut self) {
        eprintln!("Dropping variant");
        // Cleanup code if necessary
    }
}

fn start_getter(
    scope: &mut v8::HandleScope,
    key: v8::Local<v8::Name>,
    args: v8::PropertyCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let this = args.this();
    //eprintln!("key: {:?}", key.to_rust_string_lossy(scope));
    let internal_field = this.get_internal_field(scope, 0).unwrap();
    let external = v8::Local::<v8::External>::try_from(internal_field).unwrap();
    let variant_ptr = external.value() as *const Variant;
    let variant = unsafe { &*variant_ptr };

    match key.to_rust_string_lossy(scope).as_bytes() {
        b"start" => {
            rv.set(v8::Number::new(scope, variant.start() as f64).into());
        }
        b"stop" => {
            rv.set(v8::Number::new(scope, variant.record.end() as f64).into());
        }
        b"chrom" => {
            let tid = variant.record.rid().unwrap();
            let name = unsafe {String::from_utf8_unchecked(variant.record.header().rid2name(tid).unwrap().to_vec()) }; 
            let name_str = v8::String::new(scope, &name).unwrap();
            rv.set(name_str.into());
        }
        _ => {
            //rv.set(v8::Undefined(scope).into());
            // set an error
            let message = v8::String::new(scope, "Invalid key").unwrap();
            let error = v8::Exception::error(scope, message);
            rv.set(error.into());
        }
    }

}

fn weak_callback(
    isolate: &mut v8::Isolate,
    weak: v8::Weak<v8::External>,
    _parameter: *mut std::ffi::c_void,
) {
    // Recover the Arc<Variant> from the raw pointer and drop it
    let scope = &mut v8::HandleScope::new(isolate);
    let external = weak.to_local(scope).expect("error getting to_local");
    let variant_ptr = external.value() as *const Variant;
    unsafe {
        // Convert back to Arc and drop it, freeing memory if this is the last reference
        Arc::from_raw(variant_ptr);
    }
}

fn create_variant_object<'a>(
    scope: &mut v8::HandleScope<'a>,
    variant: Arc<Variant>,
) -> (v8::Local<'a, v8::Object>, v8::Weak<v8::Object>) {
    let object_template = v8::ObjectTemplate::new(scope);
    object_template.set_internal_field_count(1);

    let start_name = v8::String::new(scope, "start").unwrap();
    let stop_name = v8::String::new(scope, "stop").unwrap();
    let chrom_name = v8::String::new(scope, "chrom").unwrap();
    object_template.set_accessor(start_name.into(), start_getter);
    object_template.set_accessor(stop_name.into(), start_getter);
    object_template.set_accessor(chrom_name.into(), start_getter);

    let object = object_template.new_instance(scope).unwrap();

    // NOTE this will leak unless we call Arc::from_raw 
    let variant_ptr = Arc::into_raw(variant);

    let external_variant = v8::External::new(scope, variant_ptr as *mut _);
    object.set_internal_field(0, external_variant.into());
    // TODO: copy what's done here: https://github.com/denoland/deno/blob/41cad2179fb36c2371ab84ce587d3460af64b5fb/ext/napi/lib.rs#L522-L527 ?

    let weak = v8::Weak::with_guaranteed_finalizer(
        scope,
        object,
        Box::new(move || {
            eprintln!("Finalizing weak reference");
            unsafe { Arc::from_raw(variant_ptr); }
        }),
    );

    (object, weak)
}

fn trigger_gc(isolate: &mut v8::Isolate) {
    isolate.request_garbage_collection_for_testing(v8::GarbageCollectionType::Full);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize V8
    let platform = v8::new_default_platform(0, false).make_shared();
    v8::V8::initialize_platform(platform);
    v8::V8::initialize();

    let isolate = &mut v8::Isolate::new(Default::default());
    let handle_scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(handle_scope, Default::default());
    let scope = &mut v8::ContextScope::new(handle_scope, context);

    // Create header and VCF record
    let header = create_header();
    let vcf = bcf::Writer::from_path("_test.vcf", &header, true, bcf::Format::Vcf).unwrap();
    let mut weak_refs : Vec<v8::Weak<v8::Object>> = Vec::new();
    let n = 1000000;
    for _i in 0..n {
        //isolate.adjust_amount_of_external_allocated_memory(128);
        let record = create_vcf_record(&header, &vcf)?;
        let variant = Arc::new(Variant::new(record));

        // Create the variant object in V8
        let (variant_object, weak) = create_variant_object(scope, variant.clone());

        weak_refs.push(weak);

        // Set the variant object in the global context
        let global = context.global(scope);
        let variant_name = v8::String::new(scope, "variant").unwrap();
        global.set(scope, variant_name.into(), variant_object.into());

        // Run the JavaScript code
        let code = v8::String::new(scope, "variant.chrom").unwrap();
        let script = v8::Script::compile(scope, code, None).unwrap();
        let result = script.run(scope).unwrap();

        // Convert the result to a string and print it
        let result_str = result.to_string(scope).unwrap();
        println!("variant.start: {}, i: {}/{}", result_str.to_rust_string_lossy(scope), _i, n);
    }
    unsafe {
        v8::V8::dispose();
        v8::V8::dispose_platform();
    }
    eprintln!("done");
    // sleep for 100 seconds
    std::thread::sleep(std::time::Duration::from_secs(20));


    Ok(())
}


