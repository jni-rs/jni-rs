//! This example demonstrates the key features of the `bind_java_type!` macro
//!
//! Run with: `cargo run --example bind_java_type`

use jni::objects::JString;
use jni::refs::LoaderContext;
use jni::sys::{jint, jlong};
use jni::{Env, EnvUnowned};
use jni_macros::bind_java_type;

#[path = "utils/lib.rs"]
mod utils;

struct RustThing {
    pub message: String,
}

// Sometimes bindings need to pass raw pointers around as Java long values, so
// we define a simple wrapper type as an example.
// (this is not a well-considered design for real-world use)
#[repr(transparent)]
#[derive(Copy, Clone)]
struct ThingHandle(*const RustThing);
impl ThingHandle {
    pub fn new(thing: RustThing) -> Self {
        let boxed = Box::new(thing);
        ThingHandle(Box::into_raw(boxed))
    }

    unsafe fn as_ref(&self) -> &RustThing {
        unsafe { &*self.0 }
    }

    // Safety: only convert back to Box (to drop) when sure handle is no longer shared.
    pub unsafe fn into_box(self) -> Box<RustThing> {
        unsafe { Box::from_raw(self.0 as *mut RustThing) }
    }
}
// In order to pass ThingHandle values to/from Java as jlong, the macro expects
// a From<ThingHandle> for jlong implementation
impl From<ThingHandle> for jlong {
    fn from(handle: ThingHandle) -> Self {
        handle.0 as jlong
    }
}

// Trivial bindings can use an abbreviated syntax
bind_java_type! {
    JCustomType => "com.example.BindJavaTypeOverview$CustomType",
    constructors {
        fn new(),
    }
}

bind_java_type! {
    rust_type = JBindJavaTypeOverview,
    java_type = "com.example.BindJavaTypeOverview",

    type_map = {
        // Type mappings allow signatures to refer to other Rust binding types
        //
        // Note this uses :: for an inner class. This could also be quoted
        // as a string (and use '$') like "com.example.BindJavaTypeOverview$CustomType"
        crate::JCustomType => com.example.BindJavaTypeOverview::CustomType,

        // aliases can help with readability when mapping fully qualified types
        // so signatures can use JCustomType instead of crate::JCustomType
        // (this is more relevant to wrapper macros since we could have just mapped
        // JCustomType directly above)
        typealias JCustomType => crate::JCustomType,

        // Unsafe primitive type mappings can be used for handle types
        //
        // The bindings will statically check the size/alignment at compile time
        unsafe ThingHandle => long,
    },

    constructors {
        /// Creates a new instance with default values
        fn new(),
        /// Creates an instance with an initial value
        fn with_value(initial_value: jint),
    },

    methods {
        /// Method with no arguments or return value
        fn do_nothing(),
        /// Method with primitive arguments and return value
        fn add_numbers(a: jint, b: jint) -> jint,
        /// Method with reference type arguments and return value
        fn concatenate(a: JString, b: JString) -> JString,
        /// Method that forwards a boxed pointer to the `native_process_handle` native method
        fn process_handle(handle: ThingHandle) -> JString,
        /// Static method with 1D primitive array
        static fn echo_int_array(arr: jint[]) -> jint[],
        /// Static method with 2D primitive array
        static fn echo_int_2d_array(arr: jint[][]) -> jint[][],
        /// Static method with 1D reference array
        static fn echo_string_array(arr: JString[]) -> JString[],
        /// Static method with 2D reference array
        static fn echo_string_2d_array(arr: JString[][]) -> JString[][],

        /// Static method that refers to a type-mapped custom type
        static fn echo_custom_type(custom: crate::JCustomType) -> JCustomType,
        /// Static method that refers to a non-mapped Java class (an inner class in this case)
        /// Note: either syntax for the OtherType here is equivalent
        static fn echo_other_type(other: "com.example.BindJavaTypeOverview$OtherType") -> com.example.BindJavaTypeOverview::OtherType,

        /// Private method binding with underscore prefix
        priv fn _internal_get_info() -> JString,

        /// Test method that calls all native methods from Java
        fn test_all_native_methods() -> JString,
    },

    fields {
        /// Primitive field
        value: jint,
        /// Reference type field
        name: JString,
        custom: JCustomType,
        other: com.example.BindJavaTypeOverview::OtherType,
        /// Static primitive field
        static static_value: jint,
        static CONSTANT_FIVE {
            sig = jint,
            /// Static constant field
            #[allow(non_snake_case)]
            get = CONSTANT_FIVE,
            // with a `get = ` override but no `set = ` override then no setter is generated
        },
    },

    native_methods {
        /// Typical instance native method (exported, via trait)
        extern fn native_add(a: jint, b: jint) -> jint,
        /// Static native method (exported, via trait)
        static extern fn native_greet(name: JString) -> JString,
        /// Raw native method receives EnvUnowned directly
        raw extern fn native_raw(value: jint) -> jint,
        /// Native method with direct function implementation (bypasses trait)
        fn native_with_function {
            sig = (value: jint) -> jint,
            fn = native_with_function_impl,
        },
        /// Native method with export disabled
        fn native_not_exported {
            sig = (value: jint) -> jint,
            export = false,
        },
        /// Native method with catch_unwind disabled
        extern fn native_no_unwind {
            sig = (value: jint) -> jint,
            catch_unwind = false,
        },
        /// Native method with custom error policy
        extern fn native_custom_error_policy {
            sig = (value: jint) -> jint,
            error_policy = jni::errors::LogErrorAndDefault,
        },
        extern fn native_process_handle(handle: ThingHandle) -> JString,
    }
}

// Implement the native methods trait
impl JBindJavaTypeOverviewNativeInterface for JBindJavaTypeOverviewAPI {
    type Error = jni::errors::Error;

    fn native_add<'local>(
        _env: &mut Env<'local>,
        _this: JBindJavaTypeOverview<'local>,
        a: jint,
        b: jint,
    ) -> Result<jint, Self::Error> {
        Ok(a + b)
    }

    fn native_greet<'local>(
        env: &mut Env<'local>,
        _class: jni::objects::JClass<'local>,
        name: JString<'local>,
    ) -> Result<JString<'local>, Self::Error> {
        let name_str = name.try_to_string(env)?;
        JString::from_str(env, format!("Hello, {}!", name_str))
    }

    fn native_raw<'local>(
        _unowned_env: EnvUnowned<'local>,
        _this: JBindJavaTypeOverview<'local>,
        value: jint,
    ) -> jint {
        value * 2
    }

    fn native_not_exported<'local>(
        _env: &mut Env<'local>,
        _this: JBindJavaTypeOverview<'local>,
        value: jint,
    ) -> Result<jint, Self::Error> {
        Ok(value + 100)
    }

    fn native_no_unwind<'local>(
        _env: &mut Env<'local>,
        _this: JBindJavaTypeOverview<'local>,
        value: jint,
    ) -> Result<jint, Self::Error> {
        Ok(value - 1)
    }

    fn native_custom_error_policy<'local>(
        _env: &mut Env<'local>,
        _this: JBindJavaTypeOverview<'local>,
        value: jint,
    ) -> Result<jint, Self::Error> {
        if value < 0 {
            Err(jni::errors::Error::JniCall(jni::errors::JniError::Unknown))
        } else {
            Ok(value)
        }
    }

    fn native_process_handle<'local>(
        env: &mut ::jni::Env<'local>,
        _this: JBindJavaTypeOverview<'local>,
        handle: ThingHandle,
    ) -> ::std::result::Result<::jni::objects::JString<'local>, Self::Error> {
        let thing_ref = unsafe { handle.as_ref() };
        let response = format!("RustThing says: {}", thing_ref.message);
        JString::from_str(env, response)
    }
}

// Direct function implementation for native_with_function
fn native_with_function_impl<'local>(
    _env: &mut Env<'local>,
    _this: JBindJavaTypeOverview<'local>,
    value: jint,
) -> Result<jint, jni::errors::Error> {
    Ok(value * 3)
}

// Public wrapper for the private method binding
impl<'local> JBindJavaTypeOverview<'local> {
    /// Public wrapper that calls the private internal method
    pub fn get_info(&self, env: &mut Env<'local>) -> jni::errors::Result<String> {
        let info = self._internal_get_info(env)?;
        info.try_to_string(env)
    }
}

fn main() {
    utils::attach_current_thread(|env| {
        utils::load_class(env, "BindJavaTypeOverview")?;

        println!("=== bind_java_type! Overview Example ===\n");

        // Get the API (registers native methods)
        let _api = JBindJavaTypeOverviewAPI::get(env, &LoaderContext::default())?;

        // Constructors
        println!("--- Constructors ---");
        let obj1 = JBindJavaTypeOverview::new(env)?;
        println!("Created with new()");
        let obj2 = JBindJavaTypeOverview::with_value(env, 42)?;
        println!("Created with with_value(42)");

        // Methods
        println!("\n--- Methods ---");
        obj1.do_nothing(env)?;
        println!("Called do_nothing()");

        let sum = obj1.add_numbers(env, 10, 20)?;
        println!("add_numbers(10, 20) = {}", sum);

        let str1 = env.new_string("Hello, ")?;
        let str2 = env.new_string("World!")?;
        let result = obj1.concatenate(env, &str1, &str2)?;
        println!("concatenate result: {}", result.try_to_string(env)?);

        // Array methods
        let int_array = env.new_int_array(3)?;
        let echoed = JBindJavaTypeOverview::echo_int_array(env, &int_array)?;
        println!(
            "echo_int_array returned array of length {}",
            echoed.len(env)?
        );

        // Custom method wrapper
        println!("\n--- Custom Method Wrapper ---");
        let info = obj2.get_info(env)?;
        println!("get_info(): {}", info);

        // Fields
        println!("\n--- Fields ---");
        obj2.set_value(env, 99)?;
        let val = obj2.value(env)?;
        println!("value field: {}", val);

        let name = env.new_string("example")?;
        obj2.set_name(env, &name)?;
        let retrieved_name = obj2.name(env)?;
        println!("name field: {}", retrieved_name.try_to_string(env)?);

        let custom = JCustomType::new(env)?;
        obj2.set_custom(env, &custom)?;
        let _retrieved_custom = obj2.custom(env)?;
        println!("CustomType field set and retrieved");

        let _other = obj2.other(env)?;
        println!("un-mapped OtherType (JObject) field retrieved");

        let static_val = JBindJavaTypeOverview::static_value(env)?;
        println!("static_value: {}", static_val);

        let const_five = JBindJavaTypeOverview::CONSTANT_FIVE(env)?;
        println!("CONSTANT_FIVE: {}", const_five);

        // Native methods
        println!("\n--- Native Methods ---");
        let thing = RustThing {
            message: "Hello from Rust!".to_string(),
        };
        let handle = ThingHandle::new(thing);
        let response = obj1.process_handle(env, handle)?;
        println!(
            "native_process_handle response: {}",
            response.try_to_string(env)?
        );

        // Safety: we are done with the handle, so convert back to Box to drop
        unsafe {
            drop(handle.into_box());
        }

        // Note: Native methods are registered with the JVM but don't create direct Rust APIs.
        // They can only be called from Java. We bind a test method that calls them.
        let native_results = obj1.test_all_native_methods(env)?;
        println!(
            "Native methods test: {}",
            native_results.try_to_string(env)?
        );

        println!("\n=== Example completed successfully ===");
        Ok(())
    })
    .expect("Failed to run example");
}
