#![cfg(feature = "invocation")]

use jni::{
    jni_sig,
    objects::{JCollection, JList, JSet},
};

mod util;
use util::{attach_current_thread, unwrap};

#[test]
pub fn test_generated_alias_methods() {
    use jni::errors::Result;
    let _: Result<()> = attach_current_thread(|env| {
        // Test JList as_collection method
        let list_object = unwrap(
            env.new_object(c"java/util/ArrayList", jni_sig!("()V"), &[]),
            env,
        );
        let list = unwrap(JList::cast_local(env, list_object), env);
        let collection_via_method = list.as_collection();
        let _size1 = unwrap(collection_via_method.size(env), env);

        // Test JList From implementation (using a fresh instance)
        let list_object2 = unwrap(
            env.new_object(c"java/util/ArrayList", jni_sig!("()V"), &[]),
            env,
        );
        let list2 = unwrap(JList::cast_local(env, list_object2), env);
        let collection_via_from: JCollection = list2.into();
        let _size2 = unwrap(collection_via_from.size(env), env);

        // Test JSet as_collection method
        let set_object = unwrap(
            env.new_object(c"java/util/HashSet", jni_sig!("()V"), &[]),
            env,
        );
        let set = unwrap(JSet::cast_local(env, set_object), env);
        let set_collection_via_method = set.as_collection();
        let _size3 = unwrap(set_collection_via_method.size(env), env);

        // Test JSet From implementation (using a fresh instance)
        let set_object2 = unwrap(
            env.new_object(c"java/util/HashSet", jni_sig!("()V"), &[]),
            env,
        );
        let set2 = unwrap(JSet::cast_local(env, set_object2), env);
        let set_collection_via_from: JCollection = set2.into();
        let _size4 = unwrap(set_collection_via_from.size(env), env);

        Ok(())
    });
}
