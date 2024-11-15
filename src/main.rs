use rust_htslib::bcf::{header::TagLength, Read, Reader, Record};
use v8;

struct Variant {
    record: Record,
}

impl v8::cppgc::GarbageCollected for Variant {}

fn start_getter(
    scope: &mut v8::HandleScope,
    _key: v8::Local<v8::Name>,
    args: v8::PropertyCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let this = args.this();
    let variant = unsafe { v8::Object::unwrap::<TAG, Variant>(scope, this) }
        .expect("Failed to unwrap Variant");
    rv.set(v8::Number::new(scope, (variant.record.pos() + 1) as f64).into());
}

fn stop_getter(
    scope: &mut v8::HandleScope,
    _key: v8::Local<v8::Name>,
    args: v8::PropertyCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let this = args.this();
    let variant = unsafe { v8::Object::unwrap::<TAG, Variant>(scope, this) }
        .expect("Failed to unwrap Variant");
    let stop = variant.record.end();
    rv.set(v8::Number::new(scope, stop as f64).into());
}

fn chrom_getter(
    scope: &mut v8::HandleScope,
    _key: v8::Local<v8::Name>,
    args: v8::PropertyCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let this = args.this();
    let variant = unsafe { v8::Object::unwrap::<TAG, Variant>(scope, this) }
        .expect("Failed to unwrap Variant");
    let chrom = variant
        .record
        .header()
        .rid2name(variant.record.rid().unwrap())
        .map(|s| std::str::from_utf8(s).unwrap_or("."))
        .unwrap_or(".");
    let chrom_str = v8::String::new(scope, chrom).unwrap();
    rv.set(chrom_str.into());
}

fn filter_getter(
    scope: &mut v8::HandleScope,
    _key: v8::Local<v8::Name>,
    args: v8::PropertyCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let this = args.this();
    let variant = unsafe { v8::Object::unwrap::<TAG, Variant>(scope, this) }
        .expect("Failed to unwrap Variant");

    let filter_ids: Vec<_> = variant.record.filters().collect();
    let h = variant.record.header();
    eprintln!("filter_ids: {:?}", filter_ids);
    // get a string of all filters comma delimited
    let filter_str = filter_ids
        .iter()
        .map(|id| {
            let name = String::from_utf8(h.id_to_name(*id)).unwrap();
            eprintln!("id: {:?}, name: {}", id, name);
            name
        })
        .collect::<Vec<_>>()
        .join(",");
    eprintln!("filter_str: {}", filter_str);

    let js_str = v8::String::new(scope, &filter_str).unwrap();
    rv.set(js_str.into());
}

fn info_callback(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    if args.length() < 1 {
        return;
    }

    let this = args.this();
    let variant = unsafe { v8::Object::unwrap::<TAG, Variant>(scope, this) }
        .expect("Failed to unwrap Variant");

    let key = args
        .get(0)
        .to_string(scope)
        .unwrap()
        .to_rust_string_lossy(scope);
    let header = variant.record.header();

    // Get info from header
    if let Ok((tagtype, taglen)) = header.info_type(&key.as_bytes()) {
        let count = taglen;
        let type_id = tagtype;

        match type_id {
            rust_htslib::bcf::header::TagType::Integer => {
                let mut values = Vec::new();
                if let Ok(v) = variant.record.info(key.as_bytes()).integer() {
                    if v.is_some() {
                        values = v.unwrap().to_vec();
                    }
                }

                if count == TagLength::Fixed(1) && !values.is_empty() {
                    rv.set(v8::Number::new(scope, values[0] as f64).into());
                } else {
                    let arr = v8::Array::new(scope, values.len() as i32);
                    for (i, &val) in values.iter().enumerate() {
                        let num = v8::Number::new(scope, val as f64);
                        arr.set_index(scope, i as u32, num.into());
                    }
                    rv.set(arr.into());
                }
            }
            rust_htslib::bcf::header::TagType::Float => {
                let mut values = Vec::new();
                if let Ok(v) = variant.record.info(key.as_bytes()).float() {
                    if v.is_some() {
                        values = v.unwrap().to_vec();
                    }
                }

                if count == TagLength::Fixed(1) && !values.is_empty() {
                    rv.set(v8::Number::new(scope, values[0] as f64).into());
                } else {
                    let arr = v8::Array::new(scope, values.len() as i32);
                    for (i, &val) in values.iter().enumerate() {
                        let num = v8::Number::new(scope, val as f64);
                        arr.set_index(scope, i as u32, num.into());
                    }
                    rv.set(arr.into());
                }
            }
            rust_htslib::bcf::header::TagType::String => {
                let mut values = Vec::new();
                if let Ok(v) = variant.record.info(key.as_bytes()).string() {
                    if v.is_some() {
                        values = v.unwrap().to_vec();
                    }
                }

                if count == TagLength::Fixed(1) && !values.is_empty() {
                    if let Ok(s) = std::str::from_utf8(&values[0]) {
                        rv.set(v8::String::new(scope, s).unwrap().into());
                    }
                } else {
                    let arr = v8::Array::new(scope, values.len() as i32);
                    for (i, val) in values.iter().enumerate() {
                        if let Ok(s) = std::str::from_utf8(val) {
                            let str = v8::String::new(scope, s).unwrap();
                            arr.set_index(scope, i as u32, str.into());
                        }
                    }
                    rv.set(arr.into());
                }
            }
            _ => {
                rv.set(v8::null(scope).into());
            }
        }
    } else {
        rv.set(v8::null(scope).into());
    }
}

macro_rules! set_property_accessor {
    // Version with explicit setter
    ($template:expr, $scope:expr, $name:expr, $getter:expr, $setter:expr) => {
        $template.set_accessor_with_setter(
            v8::String::new($scope, $name).unwrap().into(),
            $getter,
            $setter,
        );
    };
    // Version with just getter (what we're using)
    ($template:expr, $scope:expr, $name:expr, $getter:expr) => {
        $template.set_accessor(v8::String::new($scope, $name).unwrap().into(), $getter);
    };
}

const TAG: u16 = 1;

fn create_object_template<'a>(
    scope: &mut v8::HandleScope<'a>,
) -> v8::Local<'a, v8::ObjectTemplate> {
    let object_template = v8::ObjectTemplate::new(scope);
    object_template.set_internal_field_count(1);

    set_property_accessor!(object_template, scope, "start", start_getter);
    set_property_accessor!(object_template, scope, "stop", stop_getter);
    set_property_accessor!(object_template, scope, "chrom", chrom_getter);
    set_property_accessor!(object_template, scope, "filter", filter_getter);

    // Add the info function
    let info_func = v8::FunctionTemplate::new(scope, info_callback);
    object_template.set(
        v8::String::new(scope, "info").unwrap().into(),
        info_func.into(),
    );

    object_template
}

fn create_variant_object<'a>(
    scope: &mut v8::HandleScope<'a>,
    object_template: v8::Local<'a, v8::ObjectTemplate>,
    record: Record,
) -> v8::Local<'a, v8::Object> {
    let object = object_template.new_instance(scope).unwrap();
    let variant = Variant { record };

    let wrapper = unsafe {
        v8::cppgc::make_garbage_collected::<Variant>(scope.get_cpp_heap().unwrap(), variant)
    };

    unsafe {
        v8::Object::wrap::<TAG, Variant>(scope, object, &wrapper);
    }

    // Calculate and report the memory used by Variant
    let variant_size = std::mem::size_of::<Variant>();
    scope.adjust_amount_of_external_allocated_memory(variant_size as i64);

    object
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize V8 with cppgc
    //let platform = v8::new_default_platform(0, false).make_shared();
    let platform = v8::new_unprotected_default_platform(0, false).make_shared();
    /*
    v8::V8::set_flags_from_string(
        "--no_freeze_flags_after_init --expose-gc --trace_gc --trace_gc_verbose --trace_gc_timer",
    );
    */

    v8::V8::initialize_platform(platform.clone());
    v8::V8::initialize();
    v8::cppgc::initalize_process(platform.clone());

    let mut heap =
        v8::cppgc::Heap::create(platform.clone(), v8::cppgc::HeapCreateParams::default());

    //let isolate = &mut v8::Isolate::new(v8::CreateParams::default().cpp_heap(heap));
    let isolate = &mut v8::Isolate::new(v8::CreateParams::default());
    isolate.attach_cpp_heap(&mut heap);

    let handle_scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(handle_scope, Default::default());
    let scope = &mut v8::ContextScope::new(handle_scope, context);

    let object_template = create_object_template(scope);
    let code = v8::String::new(scope, "variant.filters").unwrap();
    let script = v8::Script::compile(scope, code, None).unwrap();
    let global = context.global(scope);
    let variant_name = v8::String::new(scope, "variant").unwrap();

    let mut vcf = Reader::from_path("/home/brentp/data/output/HG002.DV.g.vcf.gz")?;

    let mut record_count = 0;
    let mut record = vcf.empty_record();
    while vcf.read(&mut record).is_some() {
        {
            let local_scope = &mut v8::HandleScope::new(scope);

            let variant_object =
                create_variant_object(local_scope, object_template, record.clone());
            global.set(local_scope, variant_name.into(), variant_object.into());

            // Run the JavaScript code
            let result = script.run(local_scope).unwrap();
            //global.delete(local_scope, variant_name.into());

            // Convert the result to a string and print it
            let result_str = result.to_string(local_scope).unwrap();
            if record_count % 1000 == 0 {
                println!(
                    "variant.start: {}, /{}",
                    result_str.to_rust_string_lossy(local_scope),
                    record_count
                );
            }
        }
        record_count += 1;
    }

    // cleanup
    unsafe {
        v8::cppgc::shutdown_process();
        v8::V8::dispose();
    }
    v8::V8::dispose_platform();

    eprintln!("done");
    std::thread::sleep(std::time::Duration::from_secs(2));

    Ok(())
}
