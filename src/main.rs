use std::sync::Arc;
use v8;

// see: https://github.com/denoland/deno/blob/41cad2179fb36c2371ab84ce587d3460af64b5fb/ext/napi/lib.rs#L522-L527

struct Variant {
    _chrom: String,
    _start: i32,
    _end: i32,
    _make_this_use_memory: [u64; 128]
}

impl Variant {
    fn new(chrom: String, start: i32, end: i32) -> Self {
        Self { _chrom: chrom, _start: start, _end: end, _make_this_use_memory: [0; 128] }
    }

    fn start(&self) -> i64 {
        self._start as i64
    }
    fn end(&self) -> i64 {
        self._end as i64
    }
    fn chrom(&self) -> &str {
        &self._chrom
    }
}

impl Drop for Variant {
    // never gets called.
    fn drop(&mut self) {
        eprintln!("Dropping variant");
    }
}

struct VariantWrapper {
    variant: Arc<Variant>,
}

impl v8::cppgc::GarbageCollected for VariantWrapper {
    /* 
    fn trace(&self, _visitor: &v8::cppgc::Visitor) {
        // No need to trace Arc<Variant> as it doesn't contain any GC references
    }
    */
}

fn attr_getter(
    scope: &mut v8::HandleScope,
    key: v8::Local<v8::Name>,
    args: v8::PropertyCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let this = args.this();
    let wrapper = v8::Local::<v8::Object>::try_from(this)
        .ok()
        .and_then(|obj| obj.get_internal_field(scope, 0))
        .and_then(|field| v8::Local::<v8::External>::try_from(field).ok())
        .and_then(|ext| Some(ext.value() as *mut VariantWrapper))
        .expect("Failed to get VariantWrapper");

    let variant = unsafe { &(*wrapper).variant };

    match key.to_rust_string_lossy(scope).as_bytes() {
        b"start" => {
            rv.set(v8::Number::new(scope, variant.start() as f64).into());
        }
        b"stop" => {
            rv.set(v8::Number::new(scope, variant.end() as f64).into());
        }
        b"chrom" => {
            let name = unsafe {String::from_utf8_unchecked(variant.chrom().into()) }; 
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

const TAG: u16 = 1;

fn create_variant_object<'a>(
    scope: &mut v8::HandleScope<'a>,
    variant: Arc<Variant>,
) -> v8::Local<'a, v8::Object> {
    let wrapper = unsafe { v8::cppgc::make_garbage_collected::<VariantWrapper>(
        scope.get_cpp_heap().unwrap(),
        VariantWrapper { variant },
    ) };

    let object_template = v8::ObjectTemplate::new(scope);
    object_template.set_internal_field_count(1);

    let start_name = v8::String::new(scope, "start").unwrap();
    let stop_name = v8::String::new(scope, "stop").unwrap();
    let chrom_name = v8::String::new(scope, "chrom").unwrap();
    object_template.set_accessor(start_name.into(), attr_getter);
    object_template.set_accessor(stop_name.into(), attr_getter);
    object_template.set_accessor(chrom_name.into(), attr_getter);

    let object = object_template.new_instance(scope).unwrap();
    unsafe {
        v8::Object::wrap::<TAG, VariantWrapper>(scope, object, &wrapper);
    }

    object
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize V8 with cppgc
    let platform = v8::new_default_platform(0, false).make_shared();
    v8::V8::initialize_platform(platform.clone());
    v8::V8::initialize();

    v8::cppgc::initalize_process(platform.clone());


    let heap =
    v8::cppgc::Heap::create(platform, v8::cppgc::HeapCreateParams::default());


    let isolate = &mut v8::Isolate::new(v8::CreateParams::default().cpp_heap(heap));

    let handle_scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(handle_scope, Default::default());
    let scope = &mut v8::ContextScope::new(handle_scope, context);

    let mut weak_refs : Vec<v8::Weak<v8::Object>> = Vec::new();
    let n = 1000000;
    for i in 0..n {
        //isolate.adjust_amount_of_external_allocated_memory(128);
        let record = Variant::new("chr1".to_string(), i, i + 1);
        let variant = Arc::new(record);

        // Create the variant object in V8
        let variant_object = create_variant_object(scope, variant.clone());

        //weak_refs.push(weak);

        // Set the variant object in the global context
        let global = context.global(scope);
        let variant_name = v8::String::new(scope, "variant").unwrap();
        global.set(scope, variant_name.into(), variant_object.into());

        // Run the JavaScript code
        let code = v8::String::new(scope, "variant.start").unwrap();
        let script = v8::Script::compile(scope, code, None).unwrap();
        let result = script.run(scope).unwrap();
        global.delete(scope, variant_name.into());

        // Convert the result to a string and print it
        let result_str = result.to_string(scope).unwrap();
        println!("variant.start: {}, /{}", result_str.to_rust_string_lossy(scope), n);
    }

    // Perform garbage collection
    scope.request_garbage_collection_for_testing(v8::GarbageCollectionType::Full);

    unsafe {
        v8::V8::dispose();
        v8::V8::dispose_platform();
    }
    eprintln!("done");
    // sleep for 100 seconds
    std::thread::sleep(std::time::Duration::from_secs(20));

    Ok(())
}


