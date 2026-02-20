#![cfg(feature = "invocation")]

use jni::{
    jni_sig, jni_str,
    objects::{IntoAuto, JList, JString},
    sys::jint,
};

mod util;
use util::{attach_current_thread, unwrap};

#[test]
pub fn jlist_push_and_iterate() {
    attach_current_thread(|env| {
        let data = &[
            jni_str!("hello"),
            jni_str!("world"),
            jni_str!("from"),
            jni_str!("jlist"),
            jni_str!("test"),
        ];

        // Create a new ArrayList
        let list_object = unwrap(
            env.new_object(jni_str!("java/util/ArrayList"), jni_sig!("()V"), &[]),
            env,
        );
        let list = unwrap(JList::cast_local(env, list_object), env);

        // Add all strings to the list
        unwrap(
            data.iter().try_for_each(|s| {
                let string = JString::from_jni_str(env, s)?;
                let added = list.add(env, &string)?;
                assert!(added);
                Ok(())
            }),
            env,
        );

        // Verify the list size
        let size = unwrap(list.size(env), env);
        assert_eq!(size, data.len() as jint);

        // Collect the values using the JList iterator
        let mut collected = Vec::new();
        unwrap(
            list.iter(env).and_then(|iter| {
                while let Some(obj) = iter.next(env)? {
                    let s = env.cast_local::<JString>(obj)?;
                    let s = s.mutf8_chars(env)?;
                    collected.push(s.to_owned());
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

#[test]
pub fn jlist_get_and_set() {
    attach_current_thread(|env| {
        // Create a new ArrayList
        let list_object = unwrap(
            env.new_object(jni_str!("java/util/ArrayList"), jni_sig!("()V"), &[]),
            env,
        );
        let list = unwrap(JList::cast_local(env, list_object), env);

        // Add some initial elements
        let hello_str = unwrap(env.new_string("hello"), env);
        let world_str = unwrap(env.new_string("world"), env);

        unwrap(list.add(env, &hello_str.into()), env);
        unwrap(list.add(env, &world_str.into()), env);

        // Test get method
        let first_obj = unwrap(list.get(env, 0), env);
        assert!(!first_obj.is_null());
        let first_jstring = unwrap(env.cast_local::<JString>(first_obj), env);
        let first_str = unwrap(first_jstring.mutf8_chars(env), env);
        assert_eq!(first_str.to_str().as_ref(), "hello");

        let second_obj = unwrap(list.get(env, 1), env);
        assert!(!second_obj.is_null());
        let second_jstring = unwrap(env.cast_local::<JString>(second_obj), env);
        let second_str = unwrap(second_jstring.mutf8_chars(env), env);
        assert_eq!(second_str.to_str().as_ref(), "world");

        // Test get with invalid index (should throw IndexOutOfBoundsException)
        let invalid = list.get(env, 10);
        assert!(invalid.is_err());

        assert!(matches!(
            env.exception_catch().unwrap_err(),
            jni::errors::Error::CaughtJavaException {
                ref name, ..
            } if name == "java.lang.IndexOutOfBoundsException"
        ));

        Ok(())
    })
    .unwrap();
}

#[test]
pub fn jlist_insert_and_remove() {
    attach_current_thread(|env| {
        // Create a new ArrayList
        let list_object = unwrap(
            env.new_object(jni_str!("java/util/ArrayList"), jni_sig!("()V"), &[]),
            env,
        );
        let list = unwrap(JList::cast_local(env, list_object), env);

        // Add initial elements
        let first_str = unwrap(env.new_string("first"), env);
        let third_str = unwrap(env.new_string("third"), env);

        unwrap(list.add(env, &first_str.into()), env);
        unwrap(list.add(env, &third_str.into()), env);

        // Insert in the middle
        let second_str = unwrap(env.new_string("second"), env);
        unwrap(list.insert(env, 1, &second_str), env);

        // Verify the size is now 3
        let size = unwrap(list.size(env), env);
        assert_eq!(size, 3);

        // Verify the order
        let items: Vec<String> = (0..3)
            .map(|i| {
                let obj = unwrap(list.get(env, i), env);
                let jstring = unwrap(env.cast_local::<JString>(obj), env);
                String::from(unwrap(jstring.mutf8_chars(env), env))
            })
            .collect();

        assert_eq!(items, vec!["first", "second", "third"]);

        // Remove the middle element
        let removed_obj = unwrap(list.remove(env, 1), env);
        let removed_jstring = unwrap(env.cast_local::<JString>(removed_obj), env);
        let removed_str = unwrap(removed_jstring.mutf8_chars(env), env);
        assert_eq!(removed_str.to_str().as_ref(), "second");

        // Verify size is now 2
        let size = unwrap(list.size(env), env);
        assert_eq!(size, 2);

        // Verify remaining elements
        let first_remaining = unwrap(list.get(env, 0), env);
        let first_jstring = unwrap(env.cast_local::<JString>(first_remaining), env);
        let first_str = unwrap(first_jstring.mutf8_chars(env), env);
        assert_eq!(first_str.to_str().as_ref(), "first");

        let second_remaining = unwrap(list.get(env, 1), env);
        let second_jstring = unwrap(env.cast_local::<JString>(second_remaining), env);
        let second_str = unwrap(second_jstring.mutf8_chars(env), env);
        assert_eq!(second_str.to_str().as_ref(), "third");

        Ok(())
    })
    .unwrap();
}

#[test]
pub fn jlist_size_and_remove() {
    attach_current_thread(|env| {
        // Create a new ArrayList
        let list_object = unwrap(
            env.new_object(jni_str!("java/util/ArrayList"), jni_sig!("()V"), &[]),
            env,
        );
        let list = unwrap(JList::cast_local(env, list_object), env);

        // Test size on empty list
        let size = unwrap(list.size(env), env);
        assert_eq!(size, 0);

        // Add some elements
        let first_str = unwrap(env.new_string("first"), env);
        let second_str = unwrap(env.new_string("second"), env);
        let third_str = unwrap(env.new_string("third"), env);

        unwrap(list.add(env, &first_str), env);
        unwrap(list.add(env, &second_str), env);
        unwrap(list.add(env, &third_str), env);

        // Verify size is now 3
        let size = unwrap(list.size(env), env);
        assert_eq!(size, 3);

        // Remove the last element
        let removed_obj = unwrap(list.remove(env, 2), env);
        let removed_jstring = unwrap(env.cast_local::<JString>(removed_obj), env);
        let removed_str = unwrap(removed_jstring.mutf8_chars(env), env);
        assert_eq!(removed_str.to_str().as_ref(), "third");

        // Verify size is now 2
        let size = unwrap(list.size(env), env);
        assert_eq!(size, 2);

        // Remove another element
        let removed_obj = unwrap(list.remove(env, 1), env);
        let removed_jstring = unwrap(env.cast_local::<JString>(removed_obj), env);
        let removed_str = unwrap(removed_jstring.mutf8_chars(env), env);
        assert_eq!(removed_str.to_str().as_ref(), "second");

        // Verify size is now 1
        let size = unwrap(list.size(env), env);
        assert_eq!(size, 1);

        // Remove the last element
        let removed_obj = unwrap(list.remove(env, 0), env);
        let removed_jstring = unwrap(env.cast_local::<JString>(removed_obj), env);
        let removed_str = unwrap(removed_jstring.mutf8_chars(env), env);
        assert_eq!(removed_str.to_str().as_ref(), "first");

        // Verify list is now empty
        let size = unwrap(list.size(env), env);
        assert_eq!(size, 0);

        Ok(())
    })
    .unwrap();
}

#[test]
pub fn jlist_iterator_empty() {
    attach_current_thread(|env| {
        // Create an empty ArrayList
        let list_object = unwrap(
            env.new_object(jni_str!("java/util/ArrayList"), jni_sig!("()V"), &[]),
            env,
        );
        let list = unwrap(JList::cast_local(env, list_object), env);

        // Test iterator on empty list
        let mut collected = Vec::new();
        unwrap(
            list.iter(env).and_then(|iter| {
                while let Some(obj) = iter.next(env)? {
                    let s = env.cast_local::<JString>(obj)?;
                    let s = s.mutf8_chars(env)?;
                    collected.push(s.to_owned());
                }
                Ok(())
            }),
            env,
        );

        assert!(collected.is_empty());
        Ok(())
    })
    .unwrap();
}

#[test]
pub fn jlist_iterator_with_auto() {
    attach_current_thread(|env| {
        let data = &[jni_str!("item1"), jni_str!("item2"), jni_str!("item3")];

        // Create a new ArrayList
        let list_object = unwrap(
            env.new_object(jni_str!("java/util/ArrayList"), jni_sig!("()V"), &[]),
            env,
        );
        let list = unwrap(JList::cast_local(env, list_object), env);

        // Add all strings to the list
        unwrap(
            data.iter().try_for_each(|s| {
                let string = JString::from_jni_str(env, s)?;
                let added = list.add(env, &string)?;
                assert!(added);
                Ok(())
            }),
            env,
        );

        // Test iterator with Auto<T> to prevent memory leaks
        let mut collected = Vec::new();
        unwrap(
            list.iter(env).and_then(|iter| {
                while let Some(obj) = iter.next(env)? {
                    let obj = obj.auto(); // Wrap as Auto<T> to avoid leaking while iterating
                    let s = env.as_cast::<JString>(&obj)?;
                    let s = s.mutf8_chars(env)?;
                    collected.push(s.to_owned());
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
