use crate::{
    errors::*,
    objects::{JMethodID, JObject, JValue},
    signature::{Primitive, ReturnType},
    sys::jint,
    JNIEnv,
};

/// Wrapper for JObjects that implement `java/util/List`. Provides methods to get,
/// add, and remove elements.
///
/// Looks up the class and method ids on creation rather than for every method
/// call.
pub struct JList<'a: 'b, 'b> {
    internal: JObject<'a>,
    get: JMethodID,
    add: JMethodID,
    add_idx: JMethodID,
    remove: JMethodID,
    size: JMethodID,
    env: &'b JNIEnv<'a>,
}

impl<'a: 'b, 'b> ::std::ops::Deref for JList<'a, 'b> {
    type Target = JObject<'a>;

    fn deref(&self) -> &Self::Target {
        &self.internal
    }
}

impl<'a: 'b, 'b> From<JList<'a, 'b>> for JObject<'a> {
    fn from(other: JList<'a, 'b>) -> JObject<'a> {
        other.internal
    }
}

impl<'a: 'b, 'b> JList<'a, 'b> {
    /// Create a map from the environment and an object. This looks up the
    /// necessary class and method ids to call all of the methods on it so that
    /// exra work doesn't need to be done on every method call.
    pub fn from_env(env: &'b JNIEnv<'a>, obj: JObject<'a>) -> Result<JList<'a, 'b>> {
        let class = env.auto_local(env.find_class("java/util/List")?);

        let get = env.get_method_id(&class, "get", "(I)Ljava/lang/Object;")?;
        let add = env.get_method_id(&class, "add", "(Ljava/lang/Object;)Z")?;
        let add_idx = env.get_method_id(&class, "add", "(ILjava/lang/Object;)V")?;
        let remove = env.get_method_id(&class, "remove", "(I)Ljava/lang/Object;")?;
        let size = env.get_method_id(&class, "size", "()I")?;

        Ok(JList {
            internal: obj,
            get,
            add,
            add_idx,
            remove,
            size,
            env,
        })
    }

    /// Look up the value for a key. Returns `Some` if it's found and `None` if
    /// a null pointer would be returned.
    pub fn get(&self, idx: jint) -> Result<Option<JObject<'a>>> {
        let result = self.env.call_method_unchecked(
            self.internal,
            self.get,
            ReturnType::Object,
            &[JValue::from(idx).to_jni()],
        );

        match result {
            Ok(val) => Ok(Some(val.l()?)),
            Err(e) => match e {
                Error::NullPtr(_) => Ok(None),
                _ => Err(e),
            },
        }
    }

    /// Append an element to the list
    pub fn add(&self, value: JObject<'a>) -> Result<()> {
        let result = self.env.call_method_unchecked(
            self.internal,
            self.add,
            ReturnType::Primitive(Primitive::Boolean),
            &[JValue::from(value).to_jni()],
        );

        let _ = result?;
        Ok(())
    }

    /// Insert an element at a specific index
    pub fn insert(&self, idx: jint, value: JObject<'a>) -> Result<()> {
        let result = self.env.call_method_unchecked(
            self.internal,
            self.add_idx,
            ReturnType::Primitive(Primitive::Void),
            &[JValue::from(idx).to_jni(), JValue::from(value).to_jni()],
        );

        let _ = result?;
        Ok(())
    }

    /// Remove an element from the list by index
    pub fn remove(&self, idx: jint) -> Result<Option<JObject<'a>>> {
        let result = self.env.call_method_unchecked(
            self.internal,
            self.remove,
            ReturnType::Object,
            &[JValue::from(idx).to_jni()],
        );

        match result {
            Ok(val) => Ok(Some(val.l()?)),
            Err(e) => match e {
                Error::NullPtr(_) => Ok(None),
                _ => Err(e),
            },
        }
    }

    /// Get the size of the list
    pub fn size(&self) -> Result<jint> {
        let result = self.env.call_method_unchecked(
            self.internal,
            self.size,
            ReturnType::Primitive(Primitive::Int),
            &[],
        );

        result.and_then(|v| v.i())
    }

    /// Pop the last element from the list
    ///
    /// Note that this calls `size()` to determine the last index.
    pub fn pop(&self) -> Result<Option<JObject<'a>>> {
        let size = self.size()?;
        if size == 0 {
            return Ok(None);
        }

        let result = self.env.call_method_unchecked(
            self.internal,
            self.remove,
            ReturnType::Object,
            &[JValue::from(size - 1).to_jni()],
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
    pub fn iter(&self) -> Result<JListIter<'a, 'b, '_>> {
        Ok(JListIter {
            list: self,
            current: 0,
            size: self.size()?,
        })
    }
}

/// An iterator over the keys and values in a map.
///
/// TODO: make the iterator implementation for java iterators its own thing
/// and generic enough to use elsewhere.
pub struct JListIter<'a: 'b, 'b: 'c, 'c> {
    list: &'c JList<'a, 'b>,
    current: jint,
    size: jint,
}

impl<'a: 'b, 'b: 'c, 'c> Iterator for JListIter<'a, 'b, 'c> {
    type Item = JObject<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current == self.size {
            return None;
        }
        let res = self.list.get(self.current);
        match res {
            Ok(elem) => {
                self.current += 1;
                elem
            }
            Err(_) => {
                self.current = self.size;
                None
            }
        }
    }
}
