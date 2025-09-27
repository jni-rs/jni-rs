/// Standalone test for define_reference_type! macro parsing
use std::ops::Deref;

#[derive(Default)]
struct Env;
#[allow(unused)]
impl Env {
    fn new_global_ref(&self, class: &JClass) -> Result<String, String> {
        Ok(format!("global_ref::{}", class.name()))
    }
    fn get_method_id(&self, class: &JClass, _name: &str, _sig: &str) -> Result<String, String> {
        Ok(format!("method_id::{}", class.name()))
    }
}

#[derive(Default)]
struct LoaderContext;
impl LoaderContext {
    fn load_class_for_type(&self, _env: &mut Env) -> Result<String, String> {
        Ok("loaded_class".to_string())
    }
}

#[derive(Clone)]
struct JClass(&'static str);

impl JClass {
    fn new(name: &'static str) -> Self {
        Self(name)
    }

    fn name(&self) -> &'static str {
        self.0
    }
}

impl Deref for JClass {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

fn call_init<F, R>(init: F, env: &mut Env, class: &JClass) -> Result<R, String>
where
    F: FnOnce(&mut Env, &JClass) -> Result<R, String>,
{
    init(env, class)
}

fn call_init_with_loader<F, R>(init: F, env: &mut Env, loader: &LoaderContext) -> Result<R, String>
where
    F: FnOnce(&mut Env, &LoaderContext) -> Result<R, String>,
{
    init(env, loader)
}

// Dummy name mappings for testing that avoids paste crate dependency
#[doc(hidden)]
#[macro_export]
macro_rules! __drt__with_api_ident {
    (__auto_api, Test0, $callback:ident $(, $args:tt)*) => {
        $callback!(Test0API $(, $args)*)
    };
    (__auto_api, Test1, $callback:ident $(, $args:tt)*) => {
        $callback!(Test1API $(, $args)*)
    };
    (__auto_api, Test2, $callback:ident $(, $args:tt)*) => {
        $callback!(Test2API $(, $args)*)
    };
    (__auto_api, Test3, $callback:ident $(, $args:tt)*) => {
        $callback!(Test3API $(, $args)*)
    };
    (__auto_api, Test4, $callback:ident $(, $args:tt)*) => {
        $callback!(Test4API $(, $args)*)
    };
    (__auto_api, Test5, $callback:ident $(, $args:tt)*) => {
        $callback!(Test5API $(, $args)*)
    };
    (__auto_api, Test6, $callback:ident $(, $args:tt)*) => {
        $callback!(Test6API $(, $args)*)
    };
    (__auto_api, Test7, $callback:ident $(, $args:tt)*) => {
        $callback!(Test7API $(, $args)*)
    };
    (__auto_api, Test8, $callback:ident $(, $args:tt)*) => {
        $callback!(Test8API $(, $args)*)
    };
    (__auto_api, JThrowable, $callback:ident $(, $args:tt)*) => {
        $callback!(JThrowableAPI $(, $args)*)
    };
    (Custom0API, $Type:ident, $callback:ident $(, $args:tt)*) => {
        $callback!(Custom0API $(, $args)*)
    };
    (Custom1API, $Type:ident, $callback:ident $(, $args:tt)*) => {
        $callback!(Custom1API $(, $args)*)
    };
    ($Api:ident, $Type:ident, $callback:ident $(, $args:tt)*) => {
        $callback!($Api $(, $args)*)
    };
}

// Simplified emit macro that receives individual tokens (not named parameters)
macro_rules! __drt__init_kind_report {
    (__drt__InitKindEnvClass) => {
        println!("Init kind: init");
    };
    (__drt__InitKindLoader) => {
        println!("Init kind: init_with_loader");
    };
}

macro_rules! __drt__call_init_kind {
    (__drt__InitKindEnvClass, $Init:expr, $env:ident, $Class:expr) => {{
        let class = JClass::new($Class);
        call_init($Init, &mut $env, &class)
    }};
    (__drt__InitKindLoader, $Init:expr, $env:ident, $Class:expr) => {{
        let loader = LoaderContext::default();
        call_init_with_loader($Init, &mut $env, &loader)
    }};
}

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
        impl $ApiTy {
            #[allow(unused)]
            fn get() -> Self {
                println!("Generated API for type: {}", stringify!($Type));
                println!("Class: {}", $Class);
                println!("Raw ident: {}", stringify!($RawIdent));
                println!("Init tokens: {}", stringify!($Init));
                println!("Aliases: [{}]", stringify!($($Aliases)*));
                println!("Methods: {{ {} }}", stringify!($($Methods)*));
                println!("Static methods: {{ {} }}", stringify!($($StaticMethods)*));
                println!("Fields: {{ {} }}", stringify!($($Fields)*));
                println!("Static fields: {{ {} }}", stringify!($($StaticFields)*));

                let mut env = Env::default();
                __drt__init_kind_report!($InitKind);
                __drt__call_init_kind!($InitKind, $Init, env, $Class).unwrap()
            }
        }
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

#[doc(hidden)]
#[macro_export]
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

fn main() {
    struct Test0API;
    define_reference_type!(
        type = Test0,
        class = "java.lang.Object",
        raw = jstring,
        init = |_env, _class| {
            println!("Test0: Custom init called");
            Ok(Test0API)
        }
    );
    let _ = Test0API::get();

    struct Custom0API;
    define_reference_type!(
        type = Test1,
        class = "java.lang.Object",
        init = (|_env, _class| {
            println!("Custom init called");
            Ok(Custom0API)
        }),
        raw = jstring,
        api = Custom0API
    );

    struct Custom1API;
    define_reference_type!(
        type = Test2,
        class = "java.lang.Object",
        api = Custom1API,
        init = |_env, _class| {
            println!("Custom init called");
            Ok(Custom1API)
        },
        raw = jstring,
    );

    struct Test3API;
    define_reference_type!(
        type = Test3,
        class = "java.lang.Object",
        init = |_env, _class| {
            println!("Custom init called");
            Ok(Test3API)
        },
        raw = jstring,
    );

    struct Test4API;
    define_reference_type!(
        type = Test4,
        class = "java.lang.Object",
        raw = jstring,
        init = |_env, _class| {
            println!("Custom init called");
            Ok(Test4API)
        }
    );

    struct Test5API;
    define_reference_type!(
        type = Test5,
        class = "java.lang.Object",
        raw = jstring,
        init = |_env, _class| {
            println!("Custom init called");
            Ok(Test5API)
        },
        as = [Test2, Test3],
    );

    struct Test6API;
    define_reference_type!(
        type = Test6,
        class = "java.lang.Object",
        raw = jstring,
        init = |_env, _class| {
            println!("Custom init called");
            Ok(Test6API)
        },
        as = [Test2, Test3],
        members = {
            methods = {
                get_message = {
                    name = "getMessage",
                    sig = "()Ljava/lang/String;",
                    ret = JString,
                },
                set_message = {
                    name = "setMessage",
                    sig = "(Ljava/lang/String;)V",
                    ret = void
                }
            },
            static_methods = {
                example_static = {
                    name = "exampleStatic",
                    sig = "(I)I",
                    ret = jint,
                }
            },
            fields {
                example_field = {
                    name = "exampleField",
                    sig = "I",
                    ty = jint
                }
            },
            static_fields {
                example_static_field = {
                    name = "exampleStaticField",
                    sig = "I",
                    ty = jint,
                }
            }
        }
    );

    struct Test7API;
    define_reference_type!(
        type = Test7,
        class = "java.lang.Object",
        raw = jstring,
        init = |_env, _class| {
            println!("Test7: Custom init called");
            Ok(Test7API)
        },
        as = [Test2, Test3],
        members = {
            fields {
                example_field = {
                    name = "exampleField",
                    sig = "I",
                    ty = jint,
                }
            },
            methods = {
                get_message = {
                    name = "getMessage",
                    sig = "()Ljava/lang/String;",
                    ret = JString
                },
                set_message = {
                    name = "setMessage",
                    sig = "(Ljava/lang/String;)V",
                    ret = void,
                }
            },
            static_methods = {
                example_static = {
                    name = "exampleStatic",
                    sig = "(I)I",
                    ret = jint,
                }
            },
        }
    );

    #[allow(dead_code)]
    struct JThrowableAPI {
        class: String,
        get_message_method: String,
        get_cause_method: String,
        get_stack_trace_method: String,
    }
    define_reference_type!(
        type = JThrowable,
        class = "java.lang.Throwable",
        init = |env, class| {
            println!("JThrowable: Custom init called");
            Ok(JThrowableAPI {
                class: env.new_global_ref(class)?,
                get_message_method: env.get_method_id(class, "getMessage", "()Ljava/lang/String;")?,
                get_cause_method: env.get_method_id(class, "getCause", "()Ljava/lang/Throwable;")?,
                get_stack_trace_method: env.get_method_id(class, "getStackTrace", "()[Ljava/lang/StackTraceElement;")?,
            })
        }
    );

    struct Test8API;
    define_reference_type!(
        type = Test8,
        class = "java.lang.Object",
        raw = jstring,
        init_with_loader = |env, loader_context| {
            let _class = loader_context.load_class_for_type(env).unwrap();
            println!("Test8: Custom init called");
            Ok(Test8API)
        }
    );

    Test7API::get();
    Test8API::get();

    println!("All macro parsing tests passed!");
}
