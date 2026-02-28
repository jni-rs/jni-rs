use jni::{
    bind_java_type,
    objects::{JList, JString},
};

fn main() {
    let vm = jni::JavaVM::new(
        jni::vm::InitArgsBuilder::new()
            .option("-Xcheck:jni")
            .build()
            .unwrap(),
    )
    .unwrap();

    vm.attach_current_thread(|env| {
        ArrayListAPI::get(env, &jni::refs::LoaderContext::None)?;
        //let ar = ArrayList::null();
        let ar = ArrayList::new(env, 10)?;
        let li = AsRef::<JList>::as_ref(&ar);
        assert_eq!(li.as_raw(), ar.as_raw(), "The wrong thing has been cast!");
        let s0 = env.new_string("0")?;
        li.add(env, &s0)?;
        let elem = ar.get(env, 0)?;
        let elem = JString::cast_local(env, elem)?;
        assert!(elem.try_to_string(env).unwrap() == "0");
        Result::<_, jni::errors::Error>::Ok(())
    })
    .unwrap();
}

bind_java_type! {
    pub ArrayList => java.util.ArrayList,
    is_instance_of {
        JList,
    },
    constructors {
        fn new(cap: jint)
    },
    methods {
        fn get(i: jint) -> JObject
    }
}
