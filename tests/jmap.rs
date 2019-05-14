#![cfg(feature = "invocation")]

extern crate error_chain;
extern crate jni;

use jni::objects::{
    JMap,
    JObject,
};

mod util;
use util::{
    attach_current_thread,
    unwrap,
};

#[test]
pub fn jmap_push_and_iterate() {
    let env = attach_current_thread();
    let data = &["hello", "world", "from", "test"];

    let map_object = unwrap(&env, env.new_object("java/util/HashMap", "()V", &[]));
    let map = unwrap(&env, JMap::from_env(&env, map_object));

    // Push all strings
    unwrap(
        &env,
        data.iter().try_for_each(|s| {
            env.new_string(s)
                .map(|s| JObject::from(s))
                .and_then(|s| map.put(s, s).map(|_| ()))
        }),
    );

    let mut collected = Vec::new();
    unwrap(
        &env,
        map.iter().and_then(|mut iter| {
            iter.try_for_each(|e| {
                env.get_string(e.0.into())
                    .map(|s| collected.push(String::from(s)))
            })
        }),
    );
    collected.sort();

    let mut orig = data.to_vec();
    orig.sort();
    assert_eq!(orig, collected);
}
