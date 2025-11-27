//! Example demonstrating a wrapper macro that can inject the jni crate path
//! and a type_map into jni_sig! invocations.
//!
//! Notably the wrapper doesn't need to know anything about the syntax of jni_sig!
//! and it doesn't block the user from adding their own type_map.

use jni::signature::MethodSignature;

extern crate jni as jni2;

// Example wrapper macro that always uses a custom jni path
// This could be useful in a workspace with a renamed jni dependency or with
// custom types that are used across many signatures.
macro_rules! my_jni_sig {
    ($($tt:tt)*) => {
        jni_macros::jni_sig!(
            jni = ::jni2,
            type_map = {
                BuiltinType => java.lang.BuiltinType,
            },
            $($tt)*
        )
    };
}

fn main() {
    println!("=== Wrapper Macro Example ===\n");

    // The wrapper macro works with all syntax variations:

    // 1. Simple unnamed signature
    let sig1: MethodSignature = my_jni_sig!((a: jint, b: BuiltinType) -> void);
    println!("Simple: {}", sig1.sig());
    assert_eq!(sig1.sig().to_bytes(), b"(ILjava/lang/BuiltinType;)V");

    // 2. With type_map
    let sig2: MethodSignature = my_jni_sig!(
        (a: MyType, b: BuiltinType) -> void,
        type_map = {
            MyType => com.example.MyType,
        }
    );
    println!("With type_map: {}", sig2.sig());
    assert_eq!(
        sig2.sig().to_bytes(),
        b"(Lcom/example/MyType;Ljava/lang/BuiltinType;)V"
    );

    // 3. Named signature
    let sig3: MethodSignature = my_jni_sig!(
        sig = (a: jint) -> void
    );
    println!("Named sig: {}", sig3.sig());
    assert_eq!(sig3.sig().to_bytes(), b"(I)V");

    // 4. With trailing comma
    let sig4: MethodSignature = my_jni_sig!(
        (a: jint) -> void,
    );
    println!("With trailing comma: {}", sig4.sig());
    assert_eq!(sig4.sig().to_bytes(), b"(I)V");
}
