use std::ops::{Deref, DerefMut};

use v8;

#[derive(Debug)]
struct Variant {
    _chrom: String,
    _start: i32,
    _end: i32,
    _make_this_use_memory: [u64; 128],
}

impl Variant {
    fn new(chrom: String, start: i32, end: i32) -> Self {
        Self {
            _chrom: chrom,
            _start: start,
            _end: end,
            _make_this_use_memory: [0; 128],
        }
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
    // doesn't get called until v8 is disposed
    fn drop(&mut self) {
        eprintln!("Dropping variant");
    }
}

impl v8::cppgc::GarbageCollected for Variant {
    fn trace(&self, _visitor: &v8::cppgc::Visitor) {}
}

fn attr_getter(
    scope: &mut v8::HandleScope,
    key: v8::Local<v8::Name>,
    args: v8::PropertyCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let this = args.this();

    let wrapper = unsafe { v8::Object::unwrap::<TAG, Variant>(scope, this) }
        .expect("Failed to unwrap VariantWrapper");
    let variant = &*wrapper;

    match key.to_rust_string_lossy(scope).as_bytes() {
        b"start" => {
            rv.set(v8::Number::new(scope, variant.start() as f64).into());
        }
        b"stop" => {
            rv.set(v8::Number::new(scope, variant.end() as f64).into());
        }
        b"chrom" => {
            let name = variant.chrom();
            let name_str = v8::String::new(scope, name).unwrap();
            rv.set(name_str.into());
        }
        _ => {
            let message = v8::String::new(scope, "Invalid key").unwrap();
            let error = v8::Exception::error(scope, message);
            rv.set(error.into());
        }
    }
}

const TAG: u16 = 1;

fn create_object_template<'a>(
    scope: &mut v8::HandleScope<'a>,
) -> v8::Local<'a, v8::ObjectTemplate> {
    let object_template = v8::ObjectTemplate::new(scope);
    object_template.set_internal_field_count(1);

    let start_name = v8::String::new(scope, "start").unwrap();
    let stop_name = v8::String::new(scope, "stop").unwrap();
    let chrom_name = v8::String::new(scope, "chrom").unwrap();
    object_template.set_accessor(start_name.into(), attr_getter);
    object_template.set_accessor(stop_name.into(), attr_getter);
    object_template.set_accessor(chrom_name.into(), attr_getter);

    object_template
}

fn create_variant_object<'a>(
    scope: &mut v8::HandleScope<'a>,
    object_template: v8::Local<'a, v8::ObjectTemplate>,
    variant: Variant,
) -> v8::Local<'a, v8::Object> {
    let object = object_template.new_instance(scope).unwrap();

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
    let platform = v8::new_default_platform(1, false).make_shared();
    v8::V8::set_flags_from_string(
        "--no_freeze_flags_after_init --expose-gc --trace_gc --trace_gc_verbose --trace_gc_timer",
    );

    v8::V8::initialize_platform(platform.clone());
    v8::V8::initialize();

    v8::cppgc::initalize_process(platform.clone());

    let heap = v8::cppgc::Heap::create(platform.clone(), v8::cppgc::HeapCreateParams::default());

    let isolate = &mut v8::Isolate::new(v8::CreateParams::default().cpp_heap(heap));

    let handle_scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(handle_scope, Default::default());
    let scope = &mut v8::ContextScope::new(handle_scope, context);

    let object_template = create_object_template(scope);
    let code = v8::String::new(scope, "variant.start").unwrap();
    let script = v8::Script::compile(scope, code, None).unwrap();
    let global = context.global(scope);
    let variant_name = v8::String::new(scope, "variant").unwrap();

    let n = 2000000;
    for i in 0..n {
        let record = Variant::new("chr1".to_string(), i, i + 1);
        scope.clear_kept_objects();

        //let local_scope = &mut *scope;
        // deref the scope into another scope
        let local_scope = scope.deref_mut();

        let variant_object = create_variant_object(local_scope, object_template, record);
        global.set(local_scope, variant_name.into(), variant_object.into());

        // Run the JavaScript code
        let result = script.run(local_scope).unwrap();
        drop(local_scope);

        if i % 100000 == 0 {
            scope.low_memory_notification();
            scope.request_garbage_collection_for_testing(v8::GarbageCollectionType::Full);
            unsafe {
                scope.get_cpp_heap().unwrap().collect_garbage_for_testing(
                    v8::cppgc::EmbedderStackState::MayContainHeapPointers,
                );
            }
            // Report memory decrease after garbage collection
            //let variant_size = std::mem::size_of::<Variant>();
            //scope.adjust_amount_of_external_allocated_memory(-(variant_size as i64 * 100000));
        }
        global.delete(scope, variant_name.into());

        // Convert the result to a string and print it
        let result_str = result.to_string(scope).unwrap();
        if i % 1000 == 0 {
            println!(
                "variant.start: {}, /{}",
                result_str.to_rust_string_lossy(scope),
                n
            );
        }
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
