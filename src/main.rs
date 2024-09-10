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

fn attr_getter(
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
    object_template.set_accessor(start_name.into(), attr_getter);
    object_template.set_accessor(stop_name.into(), attr_getter);
    object_template.set_accessor(chrom_name.into(), attr_getter);

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


fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize V8
    let platform = v8::new_default_platform(0, false).make_shared();
    v8::V8::initialize_platform(platform);
    v8::V8::initialize();

    let isolate = &mut v8::Isolate::new(Default::default());
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
        let (variant_object, weak) = create_variant_object(scope, variant.clone());

        //weak_refs.push(weak);

        // Set the variant object in the global context
        let global = context.global(scope);
        let variant_name = v8::String::new(scope, "variant").unwrap();
        global.set(scope, variant_name.into(), variant_object.into());

        // Run the JavaScript code
        let code = v8::String::new(scope, "variant.start").unwrap();
        let script = v8::Script::compile(scope, code, None).unwrap();
        let result = script.run(scope).unwrap();

        // Convert the result to a string and print it
        let result_str = result.to_string(scope).unwrap();
        println!("variant.start: {}, /{}", result_str.to_rust_string_lossy(scope), n);
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


