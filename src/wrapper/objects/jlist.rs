use JNIEnv;

use errors::*;

use objects::JMethodID;
use objects::JObject;

use sys::jint;

use signature::JavaType;
use signature::Primitive;

/// Wrapper for JObjects that implement `java/util/Map`. Provides methods to get
/// and set entries and a way to iterate over key/value pairs.
///
/// Looks up the class and method ids on creation rather than for every method
/// call.
pub struct JList<'a> {
    internal: JObject<'a>,
    get: JMethodID<'a>,
    add: JMethodID<'a>,
    add_idx: JMethodID<'a>,
    remove: JMethodID<'a>,
    size: JMethodID<'a>,
    env: &'a JNIEnv<'a>,
}

impl<'a> ::std::ops::Deref for JList<'a> {
    type Target = JObject<'a>;

    fn deref(&self) -> &Self::Target {
        &self.internal
    }
}

impl<'a> From<JList<'a>> for JObject<'a> {
    fn from(other: JList) -> JObject {
        other.internal
    }
}

impl<'a> JList<'a> {
    /// Create a map from the environment and an object. This looks up the
    /// necessary class and method ids to call all of the methods on it so that
    /// exra work doesn't need to be done on every method call.
    pub fn from_env(env: &'a JNIEnv<'a>, obj: JObject<'a>) -> Result<JList<'a>> {
        let class = env.find_class("java/util/List")?;

        let get = env.get_method_id(class, "get", "(I)Ljava/lang/Object;")?;
        let add = env.get_method_id(class, "add", "(Ljava/lang/Object;)Z")?;
        let add_idx = env.get_method_id(class, "add", "(ILjava/lang/Object;)V")?;
        let remove = env.get_method_id(class, "remove", "(I)Ljava/lang/Object;")?;
        let size = env.get_method_id(class, "size", "()I")?;

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
    pub fn get(&self, idx: jint) -> Result<Option<JObject>> {
        let result = self.env.call_method_unsafe(
            self.internal,
            self.get,
            JavaType::Object("java/lang/Object".into()),
            &[idx.into()],
        );

        match result {
            Ok(val) => Ok(Some(val.l()?)),
            Err(e) => match *e.kind() {
                ErrorKind::NullPtr(_) => Ok(None),
                _ => Err(e),
            },
        }
    }

    /// Append an element to the list
    pub fn add(&self, value: JObject<'a>) -> Result<()> {
        let result = self.env.call_method_unsafe(
            self.internal,
            self.add,
            JavaType::Primitive(Primitive::Boolean),
            &[value.into()],
        );

        let _ = result?;
        Ok(())
    }

    /// Insert an element at a specific index
    pub fn insert(&self, idx: jint, value: JObject<'a>) -> Result<()> {
        let result = self.env.call_method_unsafe(
            self.internal,
            self.add_idx,
            JavaType::Primitive(Primitive::Void),
            &[idx.into(), value.into()],
        );

        let _ = result?;
        Ok(())
    }

    /// Remove an element from the list by index
    pub fn remove(&self, idx: jint) -> Result<Option<JObject<'a>>> {
        let result = self.env.call_method_unsafe(
            self.internal,
            self.remove,
            JavaType::Object("java/lang/Object".into()),
            &[idx.into()],
        );

        match result {
            Ok(val) => Ok(Some(val.l()?)),
            Err(e) => match *e.kind() {
                ErrorKind::NullPtr(_) => Ok(None),
                _ => Err(e),
            },
        }
    }

    /// Get the size of the list
    pub fn size(&self) -> Result<jint> {
        let result = self.env.call_method_unsafe(
            self.internal,
            self.size,
            JavaType::Primitive(Primitive::Int),
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

        let result = self.env.call_method_unsafe(
            self.internal,
            self.remove,
            JavaType::Object("java/lang/Object".into()),
            &[(size - 1).into()],
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
    pub fn iter(&'a self) -> Result<JListIter<'a>> {
        Ok(JListIter {
            list: &self,
            current: 0,
            size: self.size()?,
        })
    }
}

/// An iterator over the keys and values in a map.
///
/// TODO: make the iterator implementation for java iterators its own thing
/// and generic enough to use elsewhere.
pub struct JListIter<'a> {
    list: &'a JList<'a>,
    current: jint,
    size: jint,
}

impl<'a> Iterator for JListIter<'a> {
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
