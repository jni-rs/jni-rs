use crate::{
    errors::*,
    objects::{AutoLocal, JMethodID, JObject},
    signature::{JavaType, Primitive},
    JNIEnv,
};

/// Wrapper for JObjects that implement `java/util/Map`. Provides methods to get
/// and set entries and a way to iterate over key/value pairs.
///
/// Looks up the class and method ids on creation rather than for every method
/// call.
pub struct JMap<'a: 'b, 'b> {
    internal: JObject<'a>,
    class: AutoLocal<'a, 'b>,
    get: JMethodID<'a>,
    put: JMethodID<'a>,
    remove: JMethodID<'a>,
    env: &'b JNIEnv<'a>,
}

impl<'a: 'b, 'b> ::std::ops::Deref for JMap<'a, 'b> {
    type Target = JObject<'a>;

    fn deref(&self) -> &Self::Target {
        &self.internal
    }
}

impl<'a: 'b, 'b> From<JMap<'a, 'b>> for JObject<'a> {
    fn from(other: JMap<'a, 'b>) -> JObject<'a> {
        other.internal
    }
}

impl<'a: 'b, 'b> JMap<'a, 'b> {
    /// Create a map from the environment and an object. This looks up the
    /// necessary class and method ids to call all of the methods on it so that
    /// exra work doesn't need to be done on every method call.
    pub fn from_env(env: &'b JNIEnv<'a>, obj: JObject<'a>) -> Result<JMap<'a, 'b>> {
        let class = env.auto_local(env.find_class("java/util/Map")?);

        let get = env.get_method_id(&class, "get", "(Ljava/lang/Object;)Ljava/lang/Object;")?;
        let put = env.get_method_id(
            &class,
            "put",
            "(Ljava/lang/Object;Ljava/lang/Object;\
             )Ljava/lang/Object;",
        )?;

        let remove =
            env.get_method_id(&class, "remove", "(Ljava/lang/Object;)Ljava/lang/Object;")?;

        Ok(JMap {
            internal: obj,
            class,
            get,
            put,
            remove,
            env,
        })
    }

    /// Look up the value for a key. Returns `Some` if it's found and `None` if
    /// a null pointer would be returned.
    pub fn get(&self, key: JObject<'a>) -> Result<Option<JObject<'a>>> {
        let result = self.env.call_method_unchecked(
            self.internal,
            self.get,
            JavaType::Object("java/lang/Object".into()),
            &[key.into()],
        );

        match result {
            Ok(val) => Ok(Some(val.l()?)),
            Err(e) => match e {
                Error::NullPtr(_) => Ok(None),
                _ => Err(e),
            },
        }
    }

    /// Look up the value for a key. Returns `Some` with the old value if the
    /// key already existed and `None` if it's a new key.
    pub fn put(&self, key: JObject<'a>, value: JObject<'a>) -> Result<Option<JObject<'a>>> {
        let result = self.env.call_method_unchecked(
            self.internal,
            self.put,
            JavaType::Object("java/lang/Object".into()),
            &[key.into(), value.into()],
        );

        match result {
            Ok(val) => Ok(Some(val.l()?)),
            Err(e) => match e {
                Error::NullPtr(_) => Ok(None),
                _ => Err(e),
            },
        }
    }

    /// Remove a value from the map. Returns `Some` with the removed value and
    /// `None` if there was no value for the key.
    pub fn remove(&self, key: JObject<'a>) -> Result<Option<JObject<'a>>> {
        let result = self.env.call_method_unchecked(
            self.internal,
            self.remove,
            JavaType::Object("java/lang/Object".into()),
            &[key.into()],
        );

        match result {
            Ok(val) => Ok(Some(val.l()?)),
            Err(e) => match e {
                Error::NullPtr(_) => Ok(None),
                _ => Err(e),
            },
        }
    }

    /// Get key/value iterator for the map. This is done by getting the
    /// `EntrySet` from java and iterating over it.
    pub fn iter(&self) -> Result<JMapIter<'a, 'b, '_>> {
        let iter_class = self
            .env
            .auto_local(self.env.find_class("java/util/Iterator")?);

        let has_next = self.env.get_method_id(&iter_class, "hasNext", "()Z")?;

        let next = self
            .env
            .get_method_id(&iter_class, "next", "()Ljava/lang/Object;")?;

        let entry_class = self
            .env
            .auto_local(self.env.find_class("java/util/Map$Entry")?);

        let get_key = self
            .env
            .get_method_id(&entry_class, "getKey", "()Ljava/lang/Object;")?;

        let get_value = self
            .env
            .get_method_id(&entry_class, "getValue", "()Ljava/lang/Object;")?;

        // Get the iterator over Map entries.
        // Use the local frame till #109 is resolved, so that implicitly looked-up
        // classes are freed promptly.
        let iter = self.env.with_local_frame(16, || {
            let entry_set = self
                .env
                .call_method_unchecked(
                    self.internal,
                    (&self.class, "entrySet", "()Ljava/util/Set;"),
                    JavaType::Object("java/util/Set".into()),
                    &[],
                )?
                .l()?;

            let iter = self
                .env
                .call_method_unchecked(
                    entry_set,
                    ("java/util/Set", "iterator", "()Ljava/util/Iterator;"),
                    JavaType::Object("java/util/Iterator".into()),
                    &[],
                )?
                .l()?;

            Ok(iter)
        })?;
        let iter = self.env.auto_local(iter);

        Ok(JMapIter {
            map: self,
            has_next,
            next,
            get_key,
            get_value,
            iter,
        })
    }
}

/// An iterator over the keys and values in a map.
///
/// TODO: make the iterator implementation for java iterators its own thing
/// and generic enough to use elsewhere.
pub struct JMapIter<'a, 'b, 'c> {
    map: &'c JMap<'a, 'b>,
    has_next: JMethodID<'a>,
    next: JMethodID<'a>,
    get_key: JMethodID<'a>,
    get_value: JMethodID<'a>,
    iter: AutoLocal<'a, 'b>,
}

impl<'a: 'b, 'b: 'c, 'c> JMapIter<'a, 'b, 'c> {
    fn get_next(&self) -> Result<Option<(JObject<'a>, JObject<'a>)>> {
        let iter = self.iter.as_obj();
        let has_next = self
            .map
            .env
            .call_method_unchecked(
                iter,
                self.has_next,
                JavaType::Primitive(Primitive::Boolean),
                &[],
            )?
            .z()?;

        if !has_next {
            return Ok(None);
        }
        let next = self
            .map
            .env
            .call_method_unchecked(
                iter,
                self.next,
                JavaType::Object("java/util/Map$Entry".into()),
                &[],
            )?
            .l()?;

        let key = self
            .map
            .env
            .call_method_unchecked(
                next,
                self.get_key,
                JavaType::Object("java/lang/Object".into()),
                &[],
            )?
            .l()?;

        let value = self
            .map
            .env
            .call_method_unchecked(
                next,
                self.get_value,
                JavaType::Object("java/lang/Object".into()),
                &[],
            )?
            .l()?;

        Ok(Some((key, value)))
    }
}

impl<'a: 'b, 'b: 'c, 'c> Iterator for JMapIter<'a, 'b, 'c> {
    type Item = (JObject<'a>, JObject<'a>);

    fn next(&mut self) -> Option<Self::Item> {
        match self.get_next() {
            Ok(Some(n)) => Some(n),
            _ => None,
        }
    }
}
