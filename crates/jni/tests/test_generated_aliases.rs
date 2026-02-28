#![cfg(feature = "invocation")]

use jni::{
    jni_sig, jni_str,
    objects::{JCollection, JList, JSet},
};

mod util;
use util::{attach_current_thread, unwrap};

#[test]
pub fn test_generated_alias_methods() {
    use jni::errors::Result;
    let _: Result<()> = attach_current_thread(|env| {
        // First just do an `AsRef` cast of a `null` object and check `.is_null()`
        // Covers: https://github.com/jni-rs/jni-rs/issues/773 bug
        let null_list = JList::null();
        let null_collection_ref: &JCollection = null_list.as_ref();
        assert!(null_collection_ref.is_null());

        let list_object = unwrap(
            env.new_object(jni_str!("java/util/ArrayList"), jni_sig!("()V"), &[]),
            env,
        );
        // Test `cast_local` method
        let list = unwrap(JList::cast_local(env, list_object), env);

        // Test AsRef cast
        // Also covers: https://github.com/jni-rs/jni-rs/issues/773 bug
        let collection_via_as_ref: &JCollection = list.as_ref();
        assert_eq!(collection_via_as_ref.as_raw(), list.as_raw());
        let _size0 = unwrap(collection_via_as_ref.size(env), env);

        // Test `as_collection` cast method
        let collection_via_method = list.as_collection();
        assert_eq!(collection_via_method.as_raw(), list.as_raw());
        let _size1 = unwrap(collection_via_method.size(env), env);

        // Test JList From implementation (using a fresh instance)
        let list_object2 = unwrap(
            env.new_object(jni_str!("java/util/ArrayList"), jni_sig!("()V"), &[]),
            env,
        );
        let list2 = unwrap(JList::cast_local(env, list_object2), env);
        let collection_via_from: JCollection = list2.into();
        let _size2 = unwrap(collection_via_from.size(env), env);

        // Test JSet as_collection method
        let set_object = unwrap(
            env.new_object(jni_str!("java/util/HashSet"), jni_sig!("()V"), &[]),
            env,
        );
        let set = unwrap(JSet::cast_local(env, set_object), env);
        let set_collection_via_method = set.as_collection();
        let _size3 = unwrap(set_collection_via_method.size(env), env);

        // Test JSet From implementation (using a fresh instance)
        let set_object2 = unwrap(
            env.new_object(jni_str!("java/util/HashSet"), jni_sig!("()V"), &[]),
            env,
        );
        let set2 = unwrap(JSet::cast_local(env, set_object2), env);
        let set_collection_via_from: JCollection = set2.into();
        let _size4 = unwrap(set_collection_via_from.size(env), env);

        Ok(())
    });
}
