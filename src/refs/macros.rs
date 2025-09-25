//use crate::objects::JClass;
//use crate::refs::LoaderContext;
//use crate::Env;

#[cfg(doc)]
use crate::{objects::JObject, refs::Reference};

/// Call the initializer through a shim to help with type inference.
#[doc(hidden)]
#[macro_export]
macro_rules! __drt__expand_init {
    ($Type:ident, $env:ident, $loader:ident, __drt__InitKindEnvClass, $Init:expr) => {{
        let class = $loader.load_class_for_type::<$Type>(true, $env)?;
        Self::call_init($env, &class, $Init)
    }};
    ($Type:ident, $env:ident, $loader:ident, __drt__InitKindLoader, $Init:expr) => {
        Self::call_init_with_loader($env, $loader, $Init)
    };
}

/// The actual emitter, parameterized by the resolved API ident.
#[doc(hidden)]
#[macro_export]
macro_rules! __drt__emit_with_api {
    (
        $ApiTy:ident,
        $Type:ident,
        $Class:expr,
        $RawIdent:ident,
        $RawTy:path,
        $InitKind:ident,
        $Init:expr,
        [ $($Aliases:tt)* ],
        { $($Methods:tt)* },
        { $($StaticMethods:tt)* },
        { $($Fields:tt)* },
        { $($StaticFields:tt)* } $(,)?
    ) => {
        paste::paste!{
            // ---------- API struct ----------
            //pub(crate) struct $ApiTy {
            //    class: $crate::refs::Global<$crate::objects::JClass<'static>>,
            //    $( $Fields )*
            //}

            impl $ApiTy {
                #[allow(unused)]
                fn call_init<F, R>(env: &mut $crate::Env, class: &JClass, init: F) -> R
                where
                    F: FnOnce(&mut $crate::Env, &JClass) -> R,
                {
                    init(env, class)
                }

                #[allow(unused)]
                fn call_init_with_loader<F, R>(env: &mut $crate::Env, loader: &$crate::refs::LoaderContext, init: F) -> R
                where
                    F: FnOnce(&mut $crate::Env, &$crate::refs::LoaderContext) -> R,
                {
                    init(env, loader)
                }

                fn get<'any_local>(
                    env: &$crate::Env<'_>,
                    loader_context: &$crate::refs::LoaderContext<'any_local, '_>,
                ) -> $crate::errors::Result<&'static Self> {
                    static CELL: once_cell::sync::OnceCell<$ApiTy> = once_cell::sync::OnceCell::new();
                    CELL.get_or_try_init(|| {
                        env.with_local_frame($crate::DEFAULT_LOCAL_FRAME_CAPACITY, |env| {
                            $crate::__drt__expand_init!($Type,
                                env,
                                loader_context,
                                $InitKind,
                                $Init
                            )
                        })
                    })
                }
            }

            // ---------- Wrapper and impls (unchanged from your version) ----------
            #[doc = concat!(r#"A `"#, $Class, r#"` wrapper that is tied to a JNI local reference frame.

See the [`JObject`] documentation for more information about reference
wrappers, how to cast them, and local reference frame lifetimes.

[`JObject`]: $crate::objects::JObject
"#)]
            #[repr(transparent)]
            #[derive(Debug, Default)]
            pub struct $Type<'local>($crate::objects::JObject<'local>);

            impl<'local> AsRef<$Type<'local>> for $Type<'local> {
                #[inline] fn as_ref(&self) -> &$Type<'local> { self }
            }
            impl<'local> AsRef<$crate::objects::JObject<'local>> for $Type<'local> {
                #[inline] fn as_ref(&self) -> &$crate::objects::JObject<'local> { self }
            }
            impl<'local> ::std::ops::Deref for $Type<'local> {
                type Target = $crate::objects::JObject<'local>;
                #[inline] fn deref(&self) -> &Self::Target { &self.0 }
            }
            impl<'local> From<$Type<'local>> for $crate::objects::JObject<'local> {
                #[inline] fn from(other: $Type<'local>) -> $crate::objects::JObject<'local> { other.0 }
            }

            impl<'local> $Type<'local> {
                #[doc = concat!(r#"Creates a [`"#, stringify!($Type), r#"`] that wraps the given `raw` [jobject]

# Safety

- `raw` must be a valid raw JNI local reference (or `null`).
- `raw` must be an instance of `"#, $Class, r#"`.
- There must not be any other owning [Reference] wrapper for the same reference.
- The local reference must belong to the current thread and not outlive the
  JNI stack frame associated with the [Env] `'local` lifetime.

[jobject]: crate::sys::jobject
[Reference]: crate::refs::Reference
[Env]: crate::Env
"#)]
                #[inline]
                pub unsafe fn from_raw<'env_inner>(
                    env: &$crate::Env<'env_inner>,
                    raw: $RawTy,
                ) -> $Type<'env_inner> {
                    let jobj: $crate::sys::jobject = raw as $crate::sys::jobject;
                    $Type($crate::objects::JObject::from_raw(env, jobj))
                }

                #[doc = concat!(r#"Creates a new null reference.

Null references are always valid and do not belong to a local reference frame. Therefore,
the returned [`"#, stringify!($Type), r#"`] always has the `'static` lifetime."#)]
                #[inline]
                pub const fn null() -> $Type<'static> {
                    $Type($crate::objects::JObject::null())
                }

                /// Unwrap to the raw jni type.
                #[inline]
                pub fn into_raw(self) -> $RawTy {
                    (self.0.into_raw()) as $RawTy
                }

                #[doc = concat!(r#"Cast a local reference to a [`"#, stringify!($Type), r#"`]

This will do a runtime (`IsInstanceOf`) check that the object is an instance of `"#, $Class, r#"`.

Also see these other options for casting local or global references to a [`"#, stringify!($Type), r#"`]:
- [Env::as_cast]
- [Env::new_cast_local_ref]
- [Env::cast_local]
- [Env::new_cast_global_ref]
- [Env::cast_global]

# Errors

Returns [Error::WrongObjectType] if the `IsInstanceOf` check fails.

[Error::WrongObjectType]: crate::errors::Error::WrongObjectType
"#)]
                #[inline]
                pub fn cast_local<'any_local>(
                    obj: impl $crate::refs::Reference
                       + Into<$crate::objects::JObject<'any_local>>
                       + AsRef<$crate::objects::JObject<'any_local>>,
                    env: &mut $crate::Env<'_>,
                ) -> $crate::errors::Result<$Type<'any_local>> {
                    env.cast_local::<$Type>(obj)
                }
            }

            // ---------- Safe upcasts ----------
            /* FIXME: refer to Aliases
            $(
                impl<'l> From<$Type<'l>> for $AsTy<'l> {
                    #[inline]
                    fn from(value: $Type<'l>) -> $AsTy<'l> {
                        let raw = value.into_jobject_raw();
                        unsafe { <$AsTy as $crate::refs::Reference>::kind_from_raw(raw) }
                    }
                }
            )*
            */

            // ---------- Reference impl ----------
            unsafe impl $crate::refs::Reference for $Type<'_> {
                type Kind<'env> = $Type<'env>;
                type GlobalKind = $Type<'static>;

                #[inline]
                fn as_raw(&self) -> $crate::sys::jobject { self.0.as_raw() }

                #[inline]
                fn class_name() -> ::std::borrow::Cow<'static, $crate::strings::JNIStr> {
                    const CLASS_NAME: &$crate::strings::JNIStr = $crate::strings::JNIStr::from_cstr(
                        match ::std::ffi::CStr::from_bytes_with_nul(concat!($Class, "\0").as_bytes()) {
                            Ok(cstr) => cstr,
                            Err(_) => panic!("Class name is not a valid C string"),
                        }
                    );
                    ::std::borrow::Cow::Borrowed(CLASS_NAME)
                }

                #[inline]
                fn lookup_class<'caller>(
                    env: &$crate::Env<'_>,
                    loader_context: $crate::refs::LoaderContext,
                ) -> $crate::errors::Result<
                    impl ::std::ops::Deref<
                        Target = $crate::refs::Global<$crate::objects::JClass<'static>>
                    > + 'caller
                > {
                    let api = $ApiTy::get(env, &loader_context)?;
                    Ok(&api.class)
                }

                #[inline]
                unsafe fn kind_from_raw<'env>(local_ref: $crate::sys::jobject) -> Self::Kind<'env> {
                    $Type($crate::objects::JObject::kind_from_raw(local_ref))
                }

                #[inline]
                unsafe fn global_kind_from_raw(global_ref: $crate::sys::jobject) -> Self::GlobalKind {
                    $Type($crate::objects::JObject::global_kind_from_raw(global_ref))
                }
            }
        }
    };
}

/// Resolve an API ident (auto => `<Type>API`) and call a callback with it.
#[doc(hidden)]
#[macro_export]
macro_rules! __drt__with_api_ident {
    (__auto_api, $Type:ident, $callback:ident $(, $args:tt)*) => {
        paste::paste! { $crate::$callback!([<$Type API>] $(, $args)*); }
    };
    ($Api:ident, $Type:ident, $callback:ident $(, $args:tt)*) => {
        $crate::$callback!($Api $(, $args)*)
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __define_reference_type_gen {
    (
        type      = $Type:ident,
        class     = $Class:expr,
        raw_ident = $RawIdent:ident,
        raw_path  = $RawTy:path,
        api       = $ApiName:ident,
        init_kind = $InitKind:ident,
        init      = $Init:expr,
        aliases   = [ $($Aliases:tt)* ],
        methods   = { $($Methods:tt)* },
        static_methods = { $($StaticMethods:tt)* },
        fields    = { $($Fields:tt)* },
        static_fields = { $($StaticFields:tt)* },
        $(,)?
    ) => {
        $crate::__drt__with_api_ident!(
            $ApiName,
            $Type,
            __drt__emit_with_api,
            $Type,
            $Class,
            $RawIdent,
            $RawTy,
            $InitKind,
            $Init,
            [ $($Aliases)* ],
            { $($Methods)* },
            { $($StaticMethods)* },
            { $($Fields)* },
            { $($StaticFields)* }
        );
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __drt__emit_init_wrapper {
    (
        $InitKind:ident,
        $InitExpr:expr,
        type   = $Type:ident,
        class  = $Class:expr,
        raw    = $RawIdent:ident,
        api    = $Api:ident,
        aliases = [ $($Aliases:tt)* ],
        methods = { $($Methods:tt)* },
        static_methods = { $($StaticMethods:tt)* },
        fields = { $($Fields:tt)* },
        static_fields = { $($StaticFields:tt)* },
    ) => {
        $crate::__define_reference_type_gen! {
            type      = $Type,
            class     = $Class,
            raw_ident = $RawIdent,
            raw_path  = $crate::sys::$RawIdent,
            api       = $Api,
            init_kind = $InitKind,
            init      = $InitExpr,
            aliases   = [ $($Aliases)* ],
            methods   = { $($Methods)* },
            static_methods = { $($StaticMethods)* },
            fields    = { $($Fields)* },
            static_fields = { $($StaticFields)* },
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __drt__dispatch_init {
    (__drt_Closure(()), __drt_Closure(()), $emit:ident, { $($args:tt)* }) => {
        compile_error!("define_reference_type!: expected exactly one of `init` or `init_with_loader`");
    };
    (__drt_Closure($Init:tt), __drt_Closure(()), $emit:ident, { $($args:tt)* }) => {
        $crate::$emit! {
            __drt__InitKindEnvClass,
            $Init,
            $($args)*
        }
    };
    (__drt_Closure(()), __drt_Closure($Init:tt), $emit:ident, { $($args:tt)* }) => {
        $crate::$emit! {
            __drt__InitKindLoader,
            $Init,
            $($args)*
        }
    };
    (__drt_Closure($Init:tt), __drt_Closure($InitWithLoader:tt), $emit:ident, { $($args:tt)* }) => {
        compile_error!(concat!("define_reference_type!: expected exactly one of `init` or `init_with_loader`, but both were provided, init = ", stringify!($Init), ", init_with_loader = ", stringify!($InitWithLoader)));
    };
}

// Emit hook that normalizes and hands off to codegen
#[doc(hidden)]
#[macro_export]
macro_rules! __def_ref_emit {
    (
        type   = $Type:ident,
        class  = $Class:expr,
        raw    = $RawIdent:ident,
        api    = $Api:ident,
        init   = __drt_Closure($Init:tt),
        init_with_loader = __drt_Closure($InitWithLoader:tt),
        aliases = [ $($Aliases:tt)* ],
        methods = { $($Methods:tt)* },
        static_methods = { $($StaticMethods:tt)* },
        fields = { $($Fields:tt)* },
        static_fields = { $($StaticFields:tt)* },
    ) => {
        $crate::__drt__dispatch_init!(
            __drt_Closure($Init),
            __drt_Closure($InitWithLoader),
            __drt__emit_init_wrapper,
            {
                type   = $Type,
                class  = $Class,
                raw    = $RawIdent,
                api    = $Api,
                aliases = [ $($Aliases)* ],
                methods = { $($Methods)* },
                static_methods = { $($StaticMethods)* },
                fields = { $($Fields)* },
                static_fields = { $($StaticFields)* },
            }
        );
    };
}

macro_rules! __def_ref_members_parse {
    (@parse
        (
            type = $Type:ident, class = $Class:expr,
            raw = $Raw:ident, api = $Api:ident,
            init = __drt_Closure($Init:tt),
            init_with_loader = __drt_Closure($InitWithLoader:tt),
            aliases = [ $($Aliases:tt)* ],
            methods = { $($Methods:tt)* },
            static_methods = { $($StaticMethods:tt)* },
            fields = { $($Fields:tt)* },
            static_fields = { $($StaticFields:tt)* },
        )
        ( $($rest:tt)* )
    ) => {
        $crate::__def_ref_members_parse! {
            @finish
            (
                type = $Type, class = $Class,
                raw = $Raw, api = $Api,
                init = __drt_Closure($Init),
                init_with_loader = __drt_Closure($InitWithLoader),
                aliases = [ $($Aliases)* ],
                methods = { $($Methods)* },
                static_methods = { $($StaticMethods)* },
                fields = { $($Fields)* },
                static_fields = { $($StaticFields)* },
            )
            ( $($rest)* )
        }
    };

    (@parse
        ($($acc:tt)*)
        ( $($rest:tt)* )
        , $($tail:tt)*
    ) => {
        $crate::__def_ref_members_parse! {
            @parse
            ( $($acc)* )
            ( $($rest)* )
            $($tail)*
        }
    };

    (@parse
        (
            type = $Type:ident, class = $Class:expr,
            raw = $Raw:ident, api = $Api:ident,
            init = __drt_Closure($Init:tt),
            init_with_loader = __drt_Closure($InitWithLoader:tt),
            aliases = [ $($Aliases:tt)* ],
            methods = { $($Methods:tt)* },
            static_methods = { $($StaticMethods:tt)* },
            fields = { $($Fields:tt)* },
            static_fields = { $($StaticFields:tt)* },
        )
        ( $($rest:tt)* )
        methods = { $($NewMethods:tt)* } $($tail:tt)*
    ) => {
        $crate::__def_ref_members_parse! {
            @parse
            (
                type = $Type, class = $Class,
                raw = $Raw, api = $Api,
                init = __drt_Closure($Init),
                init_with_loader = __drt_Closure($InitWithLoader),
                aliases = [ $($Aliases)* ],
                methods = { $($NewMethods)* },
                static_methods = { $($StaticMethods)* },
                fields = { $($Fields)* },
                static_fields = { $($StaticFields)* },
            )
            ( $($rest)* )
            $($tail)*
        }
    };

    (@parse
        (
            type = $Type:ident, class = $Class:expr,
            raw = $Raw:ident, api = $Api:ident,
            init = __drt_Closure($Init:tt),
            init_with_loader = __drt_Closure($InitWithLoader:tt),
            aliases = [ $($Aliases:tt)* ],
            methods = { $($Methods:tt)* },
            static_methods = { $($StaticMethods:tt)* },
            fields = { $($Fields:tt)* },
            static_fields = { $($StaticFields:tt)* },
        )
        ( $($rest:tt)* )
        methods { $($NewMethods:tt)* } $($tail:tt)*
    ) => {
        $crate::__def_ref_members_parse! {
            @parse
            (
                type = $Type, class = $Class,
                raw = $Raw, api = $Api,
                init = __drt_Closure($Init),
                init_with_loader = __drt_Closure($InitWithLoader),
                aliases = [ $($Aliases)* ],
                methods = { $($NewMethods)* },
                static_methods = { $($StaticMethods)* },
                fields = { $($Fields)* },
                static_fields = { $($StaticFields)* },
            )
            ( $($rest)* )
            $($tail)*
        }
    };

    (@parse
        (
            type = $Type:ident, class = $Class:expr,
            raw = $Raw:ident, api = $Api:ident,
            init = __drt_Closure($Init:tt),
            init_with_loader = __drt_Closure($InitWithLoader:tt),
            aliases = [ $($Aliases:tt)* ],
            methods = { $($Methods:tt)* },
            static_methods = { $($StaticMethods:tt)* },
            fields = { $($Fields:tt)* },
            static_fields = { $($StaticFields:tt)* },
        )
        ( $($rest:tt)* )
        static_methods = { $($NewStaticMethods:tt)* } $($tail:tt)*
    ) => {
        $crate::__def_ref_members_parse! {
            @parse
            (
                type = $Type, class = $Class,
                raw = $Raw, api = $Api,
                init = __drt_Closure($Init),
                init_with_loader = __drt_Closure($InitWithLoader),
                aliases = [ $($Aliases)* ],
                methods = { $($Methods)* },
                static_methods = { $($NewStaticMethods)* },
                fields = { $($Fields)* },
                static_fields = { $($StaticFields)* },
            )
            ( $($rest)* )
            $($tail)*
        }
    };

    (@parse
        (
            type = $Type:ident, class = $Class:expr,
            raw = $Raw:ident, api = $Api:ident,
            init = __drt_Closure($Init:tt),
            init_with_loader = __drt_Closure($InitWithLoader:tt),
            aliases = [ $($Aliases:tt)* ],
            methods = { $($Methods:tt)* },
            static_methods = { $($StaticMethods:tt)* },
            fields = { $($Fields:tt)* },
            static_fields = { $($StaticFields:tt)* },
        )
        ( $($rest:tt)* )
        static_methods { $($NewStaticMethods:tt)* } $($tail:tt)*
    ) => {
        $crate::__def_ref_members_parse! {
            @parse
            (
                type = $Type, class = $Class,
                raw = $Raw, api = $Api,
                init = __drt_Closure($Init),
                init_with_loader = __drt_Closure($InitWithLoader),
                aliases = [ $($Aliases)* ],
                methods = { $($Methods)* },
                static_methods = { $($NewStaticMethods)* },
                fields = { $($Fields)* },
                static_fields = { $($StaticFields)* },
            )
            ( $($rest)* )
            $($tail)*
        }
    };

    (@parse
        (
            type = $Type:ident, class = $Class:expr,
            raw = $Raw:ident, api = $Api:ident,
            init = __drt_Closure($Init:tt),
            init_with_loader = __drt_Closure($InitWithLoader:tt),
            aliases = [ $($Aliases:tt)* ],
            methods = { $($Methods:tt)* },
            static_methods = { $($StaticMethods:tt)* },
            fields = { $($Fields:tt)* },
            static_fields = { $($StaticFields:tt)* },
        )
        ( $($rest:tt)* )
        fields = { $($NewFields:tt)* } $($tail:tt)*
    ) => {
        $crate::__def_ref_members_parse! {
            @parse
            (
                type = $Type, class = $Class,
                raw = $Raw, api = $Api,
                init = __drt_Closure($Init),
                init_with_loader = __drt_Closure($InitWithLoader),
                aliases = [ $($Aliases)* ],
                methods = { $($Methods)* },
                static_methods = { $($StaticMethods)* },
                fields = { $($NewFields)* },
                static_fields = { $($StaticFields)* },
            )
            ( $($rest)* )
            $($tail)*
        }
    };

    (@parse
        (
            type = $Type:ident, class = $Class:expr,
            raw = $Raw:ident, api = $Api:ident,
            init = __drt_Closure($Init:tt),
            init_with_loader = __drt_Closure($InitWithLoader:tt),
            aliases = [ $($Aliases:tt)* ],
            methods = { $($Methods:tt)* },
            static_methods = { $($StaticMethods:tt)* },
            fields = { $($Fields:tt)* },
            static_fields = { $($StaticFields:tt)* },
        )
        ( $($rest:tt)* )
        fields { $($NewFields:tt)* } $($tail:tt)*
    ) => {
        $crate::__def_ref_members_parse! {
            @parse
            (
                type = $Type, class = $Class,
                raw = $Raw, api = $Api,
                init = __drt_Closure($Init),
                init_with_loader = __drt_Closure($InitWithLoader),
                aliases = [ $($Aliases)* ],
                methods = { $($Methods)* },
                static_methods = { $($StaticMethods)* },
                fields = { $($NewFields)* },
                static_fields = { $($StaticFields)* },
            )
            ( $($rest)* )
            $($tail)*
        }
    };

    (@parse
        (
            type = $Type:ident, class = $Class:expr,
            raw = $Raw:ident, api = $Api:ident,
            init = __drt_Closure($Init:tt),
            init_with_loader = __drt_Closure($InitWithLoader:tt),
            aliases = [ $($Aliases:tt)* ],
            methods = { $($Methods:tt)* },
            static_methods = { $($StaticMethods:tt)* },
            fields = { $($Fields:tt)* },
            static_fields = { $($StaticFields:tt)* },
        )
        ( $($rest:tt)* )
        static_fields = { $($NewStaticFields:tt)* } $($tail:tt)*
    ) => {
        $crate::__def_ref_members_parse! {
            @parse
            (
                type = $Type, class = $Class,
                raw = $Raw, api = $Api,
                init = __drt_Closure($Init),
                init_with_loader = __drt_Closure($InitWithLoader),
                aliases = [ $($Aliases)* ],
                methods = { $($Methods)* },
                static_methods = { $($StaticMethods)* },
                fields = { $($Fields)* },
                static_fields = { $($NewStaticFields)* },
            )
            ( $($rest)* )
            $($tail)*
        }
    };

    (@parse
        (
            type = $Type:ident, class = $Class:expr,
            raw = $Raw:ident, api = $Api:ident,
            init = __drt_Closure($Init:tt),
            init_with_loader = __drt_Closure($InitWithLoader:tt),
            aliases = [ $($Aliases:tt)* ],
            methods = { $($Methods:tt)* },
            static_methods = { $($StaticMethods:tt)* },
            fields = { $($Fields:tt)* },
            static_fields = { $($StaticFields:tt)* },
        )
        ( $($rest:tt)* )
        static_fields { $($NewStaticFields:tt)* } $($tail:tt)*
    ) => {
        $crate::__def_ref_members_parse! {
            @parse
            (
                type = $Type, class = $Class,
                raw = $Raw, api = $Api,
                init = __drt_Closure($Init),
                init_with_loader = __drt_Closure($InitWithLoader),
                aliases = [ $($Aliases)* ],
                methods = { $($Methods)* },
                static_methods = { $($StaticMethods)* },
                fields = { $($Fields)* },
                static_fields = { $($NewStaticFields)* },
            )
            ( $($rest)* )
            $($tail)*
        }
    };

    (@parse
        ($($acc:tt)*)
        ( $($rest:tt)* )
        $bad:tt $($tail:tt)*
    ) => {
        compile_error!(concat!(
            "Unknown token in members block: ", stringify!($bad),
            " | remaining: ", stringify!($($tail)*),
            " | accumulator: ", stringify!($($acc)*)
        ));
    };

    (@finish
        (
            type = $Type:ident, class = $Class:expr,
            raw = $Raw:ident, api = $Api:ident,
            init = __drt_Closure($Init:tt),
            init_with_loader = __drt_Closure($InitWithLoader:tt),
            aliases = [ $($Aliases:tt)* ],
            methods = { $($Methods:tt)* },
            static_methods = { $($StaticMethods:tt)* },
            fields = { $($Fields:tt)* },
            static_fields = { $($StaticFields:tt)* },
        )
        ( $($rest:tt)* )
    ) => {
        $crate::__def_ref_parse! {
            @parse
            (
                type = $Type, class = $Class,
                raw = $Raw, api = $Api,
                init = __drt_Closure($Init),
                init_with_loader = __drt_Closure($InitWithLoader),
                aliases = [ $($Aliases)* ],
                methods = { $($Methods)* },
                static_methods = { $($StaticMethods)* },
                fields = { $($Fields)* },
                static_fields = { $($StaticFields)* },
            )
            $($rest)*
        }
    };
}

// ===== Internal parser (Pattern A: destructure + replace) =====
#[doc(hidden)]
#[macro_export]
macro_rules! __def_ref_parse {
    // --- Done: dispatch to emit with exactly one binding per key ---
    (@parse ( $($acc:tt)* ) ) => {
        $crate::__def_ref_emit!{ $($acc)* }
    };

    // --- raw = <ident> ---
    (@parse
        (
            type = $Type:ident, class = $Class:expr,
            raw = $OldRaw:ident, api = $Api:ident,
            init = __drt_Closure($Init:tt),
            init_with_loader = __drt_Closure($InitWithLoader:tt),
            aliases = [ $($Aliases:tt)* ],
            methods = { $($Methods:tt)* },
            static_methods = { $($StaticMethods:tt)* },
            fields = { $($Fields:tt)* },
            static_fields = { $($StaticFields:tt)* },
        )
        raw = $NewRaw:ident
        $($rest:tt)*
    ) => {
        $crate::__def_ref_parse! {
            @parse
            (
                type = $Type, class = $Class,
                raw = $NewRaw, api = $Api,
                init = __drt_Closure($Init),
                init_with_loader = __drt_Closure($InitWithLoader),
                aliases = [ $($Aliases)* ],
                methods = { $($Methods)* },
                static_methods = { $($StaticMethods)* },
                fields = { $($Fields)* },
                static_fields = { $($StaticFields)* },
            )
            $($rest)*
        }
    };

    // --- api = <ident> ---
    (@parse
        (
            type = $Type:ident, class = $Class:expr,
            raw = $Raw:ident, api = $OldApi:ident,
            init = __drt_Closure($Init:tt),
            init_with_loader = __drt_Closure($InitWithLoader:tt),
            aliases = [ $($Aliases:tt)* ],
            methods = { $($Methods:tt)* },
            static_methods = { $($StaticMethods:tt)* },
            fields = { $($Fields:tt)* },
            static_fields = { $($StaticFields:tt)* },
        )
        api = $NewApi:ident
        $($rest:tt)*
    ) => {
    $crate::__def_ref_parse! {
            @parse
            (
                type = $Type, class = $Class,
                raw = $Raw, api = $NewApi,
                init = __drt_Closure($Init),
                init_with_loader = __drt_Closure($InitWithLoader),
                aliases = [ $($Aliases)* ],
                methods = { $($Methods)* },
                static_methods = { $($StaticMethods)* },
                fields = { $($Fields)* },
                static_fields = { $($StaticFields)* },
            )
            $($rest)*
        }
    };

    // --- init = <expr> ---
    (@parse
        (
            type = $Type:ident, class = $Class:expr,
            raw = $Raw:ident, api = $Api:ident,
            init = __drt_Closure($OldInit:tt),
            init_with_loader = __drt_Closure($InitWithLoader:tt),
            aliases = [ $($Aliases:tt)* ],
            methods = { $($Methods:tt)* },
            static_methods = { $($StaticMethods:tt)* },
            fields = { $($Fields:tt)* },
            static_fields = { $($StaticFields:tt)* },
        )
        init = $NewInit:expr,
        $($rest:tt)*
    ) => {
        $crate::__def_ref_parse! {
            @parse
            (
                type = $Type, class = $Class,
                raw = $Raw, api = $Api,
                init = __drt_Closure(($NewInit)),
                init_with_loader = __drt_Closure($InitWithLoader),
                aliases = [ $($Aliases)* ],
                methods = { $($Methods)* },
                static_methods = { $($StaticMethods)* },
                fields = { $($Fields)* },
                static_fields = { $($StaticFields)* },
            )
            , $($rest)*
        }
    };

    (@parse
        (
            type = $Type:ident, class = $Class:expr,
            raw = $Raw:ident, api = $Api:ident,
            init = __drt_Closure($OldInit:tt),
            init_with_loader = __drt_Closure($InitWithLoader:tt),
            aliases = [ $($Aliases:tt)* ],
            methods = { $($Methods:tt)* },
            static_methods = { $($StaticMethods:tt)* },
            fields = { $($Fields:tt)* },
            static_fields = { $($StaticFields:tt)* },
        )
        init = $NewInit:expr
    ) => {
        $crate::__def_ref_parse! {
            @parse
            (
                type = $Type, class = $Class,
                raw = $Raw, api = $Api,
                init = __drt_Closure(($NewInit)),
                init_with_loader = __drt_Closure($InitWithLoader),
                aliases = [ $($Aliases)* ],
                methods = { $($Methods)* },
                static_methods = { $($StaticMethods)* },
                fields = { $($Fields)* },
                static_fields = { $($StaticFields)* },
            )
        }
    };

    // --- init_with_loader = <expr> ---
    (@parse
        (
            type = $Type:ident, class = $Class:expr,
            raw = $Raw:ident, api = $Api:ident,
            init = __drt_Closure($Init:tt),
            init_with_loader = __drt_Closure($OldInitWithLoader:tt),
            aliases = [ $($Aliases:tt)* ],
            methods = { $($Methods:tt)* },
            static_methods = { $($StaticMethods:tt)* },
            fields = { $($Fields:tt)* },
            static_fields = { $($StaticFields:tt)* },
        )
        init_with_loader = $NewInitWithLoader:expr,
        $($rest:tt)*
    ) => {
        $crate::__def_ref_parse! {
            @parse
            (
                type = $Type, class = $Class,
                raw = $Raw, api = $Api,
                init = __drt_Closure($Init),
                init_with_loader = __drt_Closure(($NewInitWithLoader)),
                aliases = [ $($Aliases)* ],
                methods = { $($Methods)* },
                static_methods = { $($StaticMethods)* },
                fields = { $($Fields)* },
                static_fields = { $($StaticFields)* },
            )
            , $($rest)*
        }
    };

    (@parse
        (
            type = $Type:ident, class = $Class:expr,
            raw = $Raw:ident, api = $Api:ident,
            init = __drt_Closure($Init:tt),
            init_with_loader = __drt_Closure($OldInitWithLoader:tt),
            aliases = [ $($Aliases:tt)* ],
            methods = { $($Methods:tt)* },
            static_methods = { $($StaticMethods:tt)* },
            fields = { $($Fields:tt)* },
            static_fields = { $($StaticFields:tt)* },
        )
        init_with_loader = $NewInitWithLoader:expr
    ) => {
        $crate::__def_ref_parse! {
            @parse
            (
                type = $Type, class = $Class,
                raw = $Raw, api = $Api,
                init = __drt_Closure($Init),
                init_with_loader = __drt_Closure(($NewInitWithLoader)),
                aliases = [ $($Aliases)* ],
                methods = { $($Methods)* },
                static_methods = { $($StaticMethods)* },
                fields = { $($Fields)* },
                static_fields = { $($StaticFields)* },
            )
        }
    };

    // --- members = { ... } ---
    (@parse
        (
            type = $Type:ident, class = $Class:expr,
            raw = $Raw:ident, api = $Api:ident,
            init = __drt_Closure($Init:tt),
            init_with_loader = __drt_Closure($InitWithLoader:tt),
            aliases = [ $($Aliases:tt)* ],
            methods = { $($Methods:tt)* },
            static_methods = { $($StaticMethods:tt)* },
            fields = { $($Fields:tt)* },
            static_fields = { $($StaticFields:tt)* },
        )
        members = { $($MemberBody:tt)* },
        $($rest:tt)*
    ) => {
        $crate::__def_ref_members_parse! {
            @parse
            (
                type = $Type, class = $Class,
                raw = $Raw, api = $Api,
                init = __drt_Closure($Init),
                init_with_loader = __drt_Closure($InitWithLoader),
                aliases = [ $($Aliases)* ],
                methods = { $($Methods)* },
                static_methods = { $($StaticMethods)* },
                fields = { $($Fields)* },
                static_fields = { $($StaticFields)* },
            )
            ( $($rest)* )
            $($MemberBody)*
        }
    };

    (@parse
        (
            type = $Type:ident, class = $Class:expr,
            raw = $Raw:ident, api = $Api:ident,
            init = __drt_Closure($Init:tt),
            init_with_loader = __drt_Closure($InitWithLoader:tt),
            aliases = [ $($Aliases:tt)* ],
            methods = { $($Methods:tt)* },
            static_methods = { $($StaticMethods:tt)* },
            fields = { $($Fields:tt)* },
            static_fields = { $($StaticFields:tt)* },
        )
        members = { $($MemberBody:tt)* }
    ) => {
        $crate::__def_ref_members_parse! {
            @parse
            (
                type = $Type, class = $Class,
                raw = $Raw, api = $Api,
                init = __drt_Closure($Init),
                init_with_loader = __drt_Closure($InitWithLoader),
                aliases = [ $($Aliases)* ],
                methods = { $($Methods)* },
                static_methods = { $($StaticMethods)* },
                fields = { $($Fields)* },
                static_fields = { $($StaticFields)* },
            )
            ()
            $($MemberBody)*
        }
    };

    // --- as = [ ... ] ---
    (@parse
        (
            type = $Type:ident, class = $Class:expr,
            raw = $Raw:ident, api = $Api:ident,
            init = __drt_Closure($Init:tt),
            init_with_loader = __drt_Closure($InitWithLoader:tt),
            aliases = [ $($OldAliases:tt)* ],
            methods = { $($Methods:tt)* },
            static_methods = { $($StaticMethods:tt)* },
            fields = { $($Fields:tt)* },
            static_fields = { $($StaticFields:tt)* },
        )
        as = [ $($NewAliases:tt)* ],
        $($rest:tt)*
    ) => {
        $crate::__def_ref_parse! {
            @parse
            (
                type = $Type, class = $Class,
                raw = $Raw, api = $Api,
                init = __drt_Closure($Init),
                init_with_loader = __drt_Closure($InitWithLoader),
                aliases = [ $($NewAliases)* ],
                methods = { $($Methods)* },
                static_methods = { $($StaticMethods)* },
                fields = { $($Fields)* },
                static_fields = { $($StaticFields)* },
            )
            , $($rest)*
        }
    };

    (@parse
        (
            type = $Type:ident, class = $Class:expr,
            raw = $Raw:ident, api = $Api:ident,
            init = __drt_Closure($Init:tt),
            init_with_loader = __drt_Closure($InitWithLoader:tt),
            aliases = [ $($OldAliases:tt)* ],
            methods = { $($Methods:tt)* },
            static_methods = { $($StaticMethods:tt)* },
            fields = { $($Fields:tt)* },
            static_fields = { $($StaticFields:tt)* },
        )
        as = [ $($NewAliases:tt)* ]
    ) => {
        $crate::__def_ref_parse! {
            @parse
            (
                type = $Type, class = $Class,
                raw = $Raw, api = $Api,
                init = __drt_Closure($Init),
                init_with_loader = __drt_Closure($InitWithLoader),
                aliases = [ $($NewAliases)* ],
                methods = { $($Methods)* },
                static_methods = { $($StaticMethods)* },
                fields = { $($Fields)* },
                static_fields = { $($StaticFields)* },
            )
        }
    };

    // --- eat stray comma and continue ---
    (@parse ( $($acc:tt)* ) , $($rest:tt)* ) => {
        $crate::__def_ref_parse! { @parse ( $($acc)* ) $($rest)* }
    };

    // --- unknown token: nice error ---
    (@parse ( $($acc:tt)* ) $bad:tt $( $rest:tt )* ) => {
        compile_error!(concat!(
            "Unknown token: ", stringify!($bad),
            " | remaining: ", stringify!($($rest)*),
            " | accumulator: ", stringify!($($acc)*)
        ));
    };
}

/// Define a new reference type that wraps a `JObject` and implements `Reference`.
#[macro_export]
macro_rules! define_reference_type {
    (
        type = $Type:ident,
        class = $Class:expr
        $(, $($rest:tt)*)?
    ) => {
        $crate::__def_ref_parse! {
            @parse
            (
                type   = $Type,
                class  = $Class,
                raw    = jobject,
                api    = __auto_api,
                init   = __drt_Closure(()),
                init_with_loader = __drt_Closure(()),
                aliases = [],
                methods = {},
                static_methods = {},
                fields = {},
                static_fields = {},
            )
            $( , $($rest)* )?
        }
    };
}
