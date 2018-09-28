use JNIEnv;

use errors::*;

use objects::JClass;
use objects::JMethodID;
use objects::JObject;

use signature::JavaType;
use signature::Primitive;

/// Wrapper for JObjects that implement `java/util/Map`. Provides methods to get
/// and set entries and a way to iterate over key/value pairs.
///
/// Looks up the class and method ids on creation rather than for every method
/// call.
pub struct JMap<'a> {
    internal: JObject<'a>,
    class: JClass<'a>,
    get: JMethodID<'a>,
    put: JMethodID<'a>,
    remove: JMethodID<'a>,
    env: &'a JNIEnv<'a>,
}

impl<'a> ::std::ops::Deref for JMap<'a> {
    type Target = JObject<'a>;

    fn deref(&self) -> &Self::Target {
        &self.internal
    }
}

impl<'a> From<JMap<'a>> for JObject<'a> {
    fn from(other: JMap) -> JObject {
        other.internal
    }
}

impl<'a> JMap<'a> {
    /// Create a map from the environment and an object. This looks up the
    /// necessary class and method ids to call all of the methods on it so that
    /// exra work doesn't need to be done on every method call.
    pub fn from_env(env: &'a JNIEnv<'a>, obj: JObject<'a>) -> Result<JMap<'a>> {
        let class = env.find_class("java/util/Map")?;

        let get = env.get_method_id(class, "get", "(Ljava/lang/Object;)Ljava/lang/Object;")?;
        let put = env.get_method_id(
            class,
            "put",
            "(Ljava/lang/Object;Ljava/lang/Object;\
             )Ljava/lang/Object;",
        )?;

        let remove = env.get_method_id(class, "remove", "(Ljava/lang/Object;)Ljava/lang/Object;")?;

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
    pub fn get(&self, key: JObject<'a>) -> Result<Option<JObject>> {
        let result = self.env.call_method_unsafe(
            self.internal,
            self.get,
            JavaType::Object("java/lang/Object".into()),
            &[key.into()],
        );

        match result {
            Ok(val) => Ok(Some(val.l()?)),
            Err(e) => match *e.kind() {
                ErrorKind::NullPtr(_) => Ok(None),
                _ => Err(e),
            },
        }
    }

    /// Look up the value for a key. Returns `Some` with the old value if the
    /// key already existed and `None` if it's a new key.
    pub fn put(&self, key: JObject<'a>, value: JObject<'a>) -> Result<Option<JObject>> {
        let result = self.env.call_method_unsafe(
            self.internal,
            self.put,
            JavaType::Object("java/lang/Object".into()),
            &[key.into(), value.into()],
        );

        match result {
            Ok(val) => Ok(Some(val.l()?)),
            Err(e) => match *e.kind() {
                ErrorKind::NullPtr(_) => Ok(None),
                _ => Err(e),
            },
        }
    }

    /// Remove a value from the map. Returns `Some` with the removed value and
    /// `None` if there was no value for the key.
    pub fn remove(&self, key: JObject<'a>) -> Result<Option<JObject<'a>>> {
        let result = self.env.call_method_unsafe(
            self.internal,
            self.remove,
            JavaType::Object("java/lang/Object".into()),
            &[key.into()],
        );

        match result {
            Ok(val) => Ok(Some(val.l()?)),
            Err(e) => match *e.kind() {
                ErrorKind::NullPtr(_) => Ok(None),
                _ => Err(e),
            },
        }
    }

    /// Get key/value iterator for the map. This is done by getting the
    /// `EntrySet` from java and iterating over it.
    pub fn iter(&'a self) -> Result<JMapIter<'a>> {
        let set = self.env.call_method_unsafe(
            self.internal,
            (self.class, "entrySet", "()Ljava/util/Set;"),
            JavaType::Object("java/util/Set".into()),
            &[],
        )?.l()?;

        let iter = self.env.call_method_unsafe(
            set,
            ("java/util/Set", "iterator", "()Ljava/util/Iterator;"),
            JavaType::Object("java/util/Iterator".into()),
            &[],
        )?.l()?;

        let iter_class = self.env.find_class("java/util/Iterator")?;

        let has_next = self.env.get_method_id(iter_class, "hasNext", "()Z")?;

        let next = self.env
            .get_method_id(iter_class, "next", "()Ljava/lang/Object;")?;

        let entry_class = self.env.find_class("java/util/Map$Entry")?;

        let get_key = self.env
            .get_method_id(entry_class, "getKey", "()Ljava/lang/Object;")?;

        let get_value = self.env
            .get_method_id(entry_class, "getValue", "()Ljava/lang/Object;")?;

        Ok(JMapIter {
            map: &self,
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
pub struct JMapIter<'a> {
    map: &'a JMap<'a>,
    has_next: JMethodID<'a>,
    next: JMethodID<'a>,
    get_key: JMethodID<'a>,
    get_value: JMethodID<'a>,
    iter: JObject<'a>,
}

impl<'a> JMapIter<'a> {
    fn get_next(&self) -> Result<Option<(JObject<'a>, JObject<'a>)>> {
        let has_next = self.map.env.call_method_unsafe(
            self.iter,
            self.has_next,
            JavaType::Primitive(Primitive::Boolean),
            &[],
        )?.z()?;

        if !has_next {
            return Ok(None);
        }
        let next = self.map.env.call_method_unsafe(
            self.iter,
            self.next,
            JavaType::Object("java/util/Map$Entry".into()),
            &[],
        )?.l()?;

        let key = self.map.env.call_method_unsafe(
            next,
            self.get_key,
            JavaType::Object("java/lang/Object".into()),
            &[],
        )?.l()?;

        let value = self.map.env.call_method_unsafe(
            next,
            self.get_value,
            JavaType::Object("java/lang/Object".into()),
            &[],
        )?.l()?;

        Ok(Some((key, value)))
    }
}

impl<'a> Iterator for JMapIter<'a> {
    type Item = (JObject<'a>, JObject<'a>);

    fn next(&mut self) -> Option<Self::Item> {
        match self.get_next() {
            Ok(Some(n)) => Some(n),
            _ => None,
        }
    }
}
