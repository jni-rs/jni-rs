#![cfg(feature = "invocation")]

use jni::objects::{JMap, JObject, JString};

mod util;
use util::{attach_current_thread, unwrap};

#[test]
pub fn jmap_push_and_iterate() {
    attach_current_thread(|env| {
        let data = &["hello", "world", "from", "test"];

        // Create a new map. Use LinkedHashMap to have predictable iteration order
        let map_object = unwrap(env.new_object("java/util/LinkedHashMap", "()V", &[]), env);
        let map = unwrap(JMap::from_env(env, &map_object), env);

        // Push all strings
        unwrap(
            data.iter().try_for_each(|s| {
                env.new_string(s)
                    .map(JObject::from)
                    .and_then(|s| map.put(env, &s, &s).map(|_| ()))
            }),
            env,
        );

        // Collect the keys using the JMap iterator
        let mut collected = Vec::new();
        unwrap(
            map.iter(env).and_then(|mut iter| {
                while let Some(e) = iter.next(env)? {
                    let s = env.cast_local::<JString>(e.0)?;
                    let s = env.get_string(&s)?;
                    collected.push(String::from(s));
                }
                Ok(())
            }),
            env,
        );

        let orig = data.to_vec();
        assert_eq!(orig, collected);
        Ok(())
    })
    .unwrap();
}
