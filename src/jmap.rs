use errors::*;

use desc::Desc;

use jnienv::JNIEnv;
use jobject::JObject;
use jclass::JClass;
use jmethodid::JMethodID;

use sys::{jobject, jclass};

use signature::JavaType;
use signature::Primitive;

pub struct JMap<'a> {
    internal: JObject<'a>,
    get: JMethodID<'a>,
    put: JMethodID<'a>,
    entry_set: JMethodID<'a>,
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
    pub fn from_env(env: &'a JNIEnv<'a>, obj: JObject<'a>) -> Result<JMap<'a>> {
        let class = env.find_class(Desc::Descriptor("java/util/Map"))?;
        let class = move || {
            let c: Desc<&'static str, JClass> = Desc::Value(class);
            c
        };

        let get = env.get_method_id(Desc::Descriptor((class(),
                                                      "get",
                                                      "(Ljava/lang/Object;\
                                                       )Ljava/lang/Object;")))?;
        let put = env.get_method_id(Desc::Descriptor((class(),
                                                      "put",
                                                      "(Ljava/lang/Object;\
                                                       Ljava/lang/Object;\
                                                       )Ljava/lang/Object;")))?;
        let entry_set = env.get_method_id(Desc::Descriptor((class(),
                                                            "entrySet",
                                                            "()Ljava/util/Set;")))?;
        Ok(JMap {
            internal: obj,
            get: get,
            put: put,
            entry_set: entry_set,
            env: env,
        })
    }

    pub fn get(&self, key: JObject<'a>) -> Result<Option<JObject>> {
        let result = unsafe {
            self.env.call_method_unsafe::<&str, &str>(
                self.internal,
                Desc::Value(self.get),
                JavaType::Object("java/lang/Object".into()),
                &[key.into()],
            )
        };

        match result {
            Ok(val) => Ok(Some(val.l()?)),
            Err(e) => {
                match e.kind() {
                    &ErrorKind::NullPtr(_) => Ok(None),
                    _ => Err(e),
                }
            }
        }
    }

    pub fn put(&self,
               key: JObject<'a>,
               value: JObject<'a>)
               -> Result<Option<JObject>> {
        let result = unsafe {
            self.env.call_method_unsafe::<&str, &str>(
                self.internal,
                Desc::Value(self.put),
                JavaType::Object("java/lang/Object".into()),
                &[key.into(), value.into()],
            )
        };

        match result {
            Ok(val) => Ok(Some(val.l()?)),
            Err(e) => {
                match e.kind() {
                    &ErrorKind::NullPtr(_) => Ok(None),
                    _ => Err(e),
                }
            }
        }
    }

    pub fn iter(&'a self) -> Result<JMapIter<'a>> {
        let set = unsafe {
            self.env
                .call_method_unsafe::<&str, &str>(self.internal,
                                                  Desc::Value(self.entry_set),
                                                  JavaType::Object("java/util/Set".into()),
                                                  &[])?
            .l()?
        };

        let iter = unsafe {
            self.env
                .call_method_unsafe::<&str, &str>(set,
                                                  Desc::Descriptor(("iterator", "()Ljava/util/Iterator;")),
                                                  JavaType::Object("java/util/Iterator".into()),
                                                  &[])?
            .l()?
        };

        let iter_class = self.env.find_class(Desc::Descriptor("java/util/Iterator"))?;
        let has_next = self.env
            .get_method_id::<&str, &str, &str>(Desc::Descriptor((Desc::Value(iter_class),
                                                                 "hasNext",
                                                                 "()Z")))?;
        let next = self.env
            .get_method_id::<&str, &str, &str>(Desc::Descriptor((Desc::Value(iter_class),
                                                                 "next",
                                                                 "()Ljava/lang/Object;")))?;

        let entry_class = self.env
            .find_class(Desc::Descriptor("java/util/Map$Entry"))?;

        let get_key = self.env
            .get_method_id::<&str, &str, &str>(
                Desc::Descriptor((Desc::Value(entry_class),
                                  "getKey",
                                  "()Ljava/lang/Object;")))?;
        let get_value = self.env
            .get_method_id::<&str, &str, &str>(
                Desc::Descriptor((Desc::Value(entry_class),
                                  "getValue",
                                  "()Ljava/lang/Object;")))?;

        Ok(JMapIter {
            map: &self,
            has_next: has_next,
            next: next,
            get_key: get_key,
            get_value: get_value,
            iter: iter,
        })
    }
}

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
        let has_next = unsafe {
            self.map.env.call_method_unsafe::<&str, &str>(
                self.iter,
                Desc::Value(self.has_next),
                JavaType::Primitive(Primitive::Boolean),
                &[]
            )?.z()?
        };

        if !has_next {
            return Ok(None);
        }
        let next = unsafe {
            self.map.env.call_method_unsafe::<&str, &str>(
                self.iter,
                Desc::Value(self.next),
                JavaType::Object("java/util/Map$Entry".into()),
                &[],
            )?.l()?
        };

        let key = unsafe {
            self.map.env.call_method_unsafe::<&str, &str>(
                next,
                Desc::Value(self.get_key),
                JavaType::Object("java/lang/Object".into()),
                &[],
            )?.l()?
        };

        let value = unsafe {
            self.map.env.call_method_unsafe::<&str, &str>(
                next,
                Desc::Value(self.get_value),
                JavaType::Object("java/lang/Object".into()),
                &[],
            )?.l()?
        };

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
