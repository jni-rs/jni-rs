#![cfg(feature = "invocation")]

use jni::objects::{JMap, JObject};

mod util;
use util::{attach_current_thread, unwrap};

#[test]
pub fn jmap_push_and_iterate() {
    let env = attach_current_thread();
    let data = &["hello", "world", "from", "test"];

    // Create a new map. Use LinkedHashMap to have predictable iteration order
    let map_object = unwrap(&env, env.new_object("java/util/LinkedHashMap", "()V", &[]));
    let map = unwrap(&env, JMap::from_env(&env, map_object));

    // Push all strings
    unwrap(
        &env,
        data.iter().try_for_each(|s| {
            env.new_string(s)
                .map(JObject::from)
                .and_then(|s| map.put(s, s).map(|_| ()))
        }),
    );

    // Collect the keys using the JMap iterator
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

    let orig = data.to_vec();
    assert_eq!(orig, collected);
}
