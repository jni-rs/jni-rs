#[path = "../../util/mod.rs"]
mod util;
use util::attach_current_thread;

use jni::Env;

pub fn main() {
    attach_current_thread(|env0: &mut Env| {
        env0.with_local_frame(10, |_env1: &mut Env| -> jni::errors::Result<_> {
            let _s = env0.new_string(c"hello").unwrap();
            eprintln!("BUG: this shouldn't compile since env0 shouldn't be mutable and new_string requires a mutable Env");
            Ok(())
        })
        .unwrap();
        Ok(())
    })
    .unwrap();
}
