//! Example demonstrating `is_instance_of` functionality
//!
//! This example shows how to:
//! - Bind a Java class that extends ArrayList (java.util.List)
//! - Use `is_instance_of` to declare that instances can be cast to JList + JCollection
//! - Use stem name to generate convenient `as_<stem>()` methods
//! - Cast from the bound reference type to the is_instance_of type using From + AsRef traits
//!
//! Note: The bindings will include a runtime self-check within `<Type>API::get()` to do
//! a one-time verification that the Java object is indeed an instance of the declared
//! types in `is_instance_of`.

use jni::objects::{JCollection, JList};
use jni::refs::LoaderContext;
use jni_macros::bind_java_type;

#[path = "utils/lib.rs"]
mod utils;

// Bind our custom Java class that extends ArrayList
bind_java_type! {
    rust_type = JInstanceOf,
    java_type = "com.example.InstanceOf",

    is_instance_of = {
        // Using a stem name "list" generates an as_list() method
        list = JList,
        JCollection,
    },

    constructors {
        fn new(),
    },

    methods {
        fn add_upper(val: JString) -> bool,
    }
}

fn main() {
    utils::attach_current_thread(|env| {
        utils::load_class(env, "InstanceOf")?;

        // Get the API (requires LoaderContext for class loading)
        // This loads the class and caches method/field IDs
        let _api = JInstanceOfAPI::get(env, &LoaderContext::default())?;

        let instance = JInstanceOf::new(env)?;
        println!("Created empty JInstanceOf instance");

        // Call methods from our custom class via the API
        let item = env.new_string("hello")?;
        instance.add_upper(env, &item)?;
        println!("Added upper-case element to list");

        // Demonstrate as_list() method (generated because we used a stem name)
        println!("\nUsing as_list() method (generated from stem name):");
        let as_list = instance.as_list();
        let size = as_list.size(env)?;
        println!("  List size: {}", size);

        // Add an item to the list
        let item = env.new_string("World")?;
        as_list.add(env, &item)?;

        let new_size = as_list.size(env)?;
        println!("  List size after adding another item: {}", new_size);

        // Demonstrate AsRef trait conversion to &JList
        println!("\nUsing AsRef trait to convert to &JList:");
        let as_list_from: &JList = instance.as_ref();
        let size_from = as_list_from.size(env)?;
        println!("  List size (via AsRef): {}", size_from);

        println!("\nUsing AsRef trait to convert &JList to &JCollection:");
        let as_collection_via_list: &JCollection = as_list_from.as_ref();
        let size_collection_via_list = as_collection_via_list.size(env)?;
        println!(
            "  Collection size (via AsRef from &JList): {}",
            size_collection_via_list
        );

        println!("\nUsing Into trait to convert JInstanceOf to JCollection:");
        let as_collection_from: JCollection = instance.into();
        let size_collection_from = as_collection_from.size(env)?;
        println!("  Collection size (via Into): {}", size_collection_from);

        Ok(())
    })
    .expect("Failed to run example");
}
