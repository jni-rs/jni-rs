extern crate jni;

// This is the interface to the JVM that we'll call the majority of our methods on.
use jni::JNIEnv;

// These objects are what you should use as arguments to your native function.
// They carry extra lifetime information to prevent them escaping this context
// and getting used after being GC'd.
use jni::objects::{JClass, JString};

// This is just a pointer. We'll be returning it from our function.
// We can't return one of the objects with lifetime information because the
// lifetime checker won't let us.
use jni::sys::jstring;

// This keeps rust from "mangling" the name and making it unique for this crate.
#[no_mangle]
// This turns off linter warnings because the name doesn't conform to conventions.
#[allow(non_snake_case)]
pub extern "C" fn Java_HelloWorld_hello(env: JNIEnv,
                                        // this is the class that owns our
                                        // static method. Not going to be used,
                                        // but still needs to have an argument
                                        // slot
                                        class: JClass,
                                        input: JString)
                                        -> jstring {
    // First, we have to get the string out of java. Check out the `strings`
    // module for more info on how this works.
    let input: String =
        env.get_string(input).expect("Couldn't get java string!").into();

    // Then we have to create a new java string to return. Again, more info
    // in the `strings` module.
    let output = env.new_string(format!("Hello, {}!", input))
        .expect("Couldn't create java string!");

    // Finally, extract the raw pointer to return.
    output.into_inner()
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}
