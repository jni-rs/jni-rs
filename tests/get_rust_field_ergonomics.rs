// Based on the example from https://github.com/jni-rs/jni-rs/issues/539
// this checks the ergonomics of using get_rust_field to lock multiple fields
// without losing access to any `&mut Env` reference you might have.

use jni::{objects::JObject, EnvUnowned};

pub struct Renderer {
    // fields
}
impl Renderer {
    pub fn render(&mut self, _surface: &Surface) {
        // rendering logic
    }
}

pub struct Surface {
    // fields
}

#[no_mangle]
pub extern "system" fn Java_Renderer_render<'local>(
    mut unowned_env: EnvUnowned<'local>,
    _this: JObject<'local>,
    renderer: JObject<'local>,
    surface: JObject<'local>,
) {
    unowned_env
        .with_env(|env| -> jni::errors::Result<_> {
            let mut renderer =
                unsafe { env.get_rust_field::<_, _, Renderer>(renderer, c"mNative")? };
            let surface = unsafe { env.get_rust_field::<_, _, Surface>(surface, c"mNative")? };
            renderer.render(&surface);

            // Check we can still call something requiring `&mut Env` after locking multiple fields
            let _s = env.new_string(c"hello")?;

            Ok(())
        })
        .resolve::<jni::errors::LogErrorAndDefault>()
}
