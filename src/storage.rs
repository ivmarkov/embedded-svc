extern crate alloc;

pub trait Storage {
    type Error;

    fn contains(&self, key: impl AsRef<str>) -> Result<bool, Self::Error>;

    fn remove(&mut self, key: impl AsRef<str>) -> Result<bool, Self::Error>;

    fn len(&self, key: impl AsRef<str>) -> Result<Option<usize>, Self::Error> {
        Ok(self.get_raw(key)?.map(|v| v.len()))
    }

    fn get_raw(&self, key: impl AsRef<str>) -> Result<Option<alloc::vec::Vec<u8>>, Self::Error>;

    fn put_raw(&mut self, key: impl AsRef<str>, value: impl Into<alloc::vec::Vec<u8>>) -> Result<bool, Self::Error>;

    #[cfg(feature = "use_serde")]
    fn get<T: serde::de::DeserializeOwned>(&self, key: impl AsRef<str>) -> Result<Option<T>, Self::Error> {
        let data = self.get_raw(key)?;

        Ok(if let Some(data) = data {
            Some(serde_json::from_slice::<T>(&data).unwrap()) // TODO
        } else {
            None
        })
    }

    #[cfg(feature = "use_serde")]
    fn put(&mut self, key: impl AsRef<str>, value: &impl serde::Serialize) -> Result<bool, Self::Error> {
        self.put_raw(key, &*serde_json::to_vec(value).unwrap()) // TODO
    }
}

pub struct AnyhowStorage<T>(pub T);

impl<E, S> Storage for AnyhowStorage<S> where E: Into<anyhow::Error>, S: Storage<Error = E> {
    type Error = anyhow::Error;

    fn contains(&self, key: impl AsRef<str>) -> Result<bool, Self::Error> {
        self.0.contains(key).map_err(Into::into)
    }

    fn remove(&mut self, key: impl AsRef<str>) -> Result<bool, Self::Error> {
        self.0.remove(key).map_err(Into::into)
    }

    fn len(&self, key: impl AsRef<str>) -> Result<Option<usize>, Self::Error> {
        self.0.len(key).map_err(Into::into)
    }

    fn get_raw(&self, key: impl AsRef<str>) -> Result<Option<alloc::vec::Vec<u8>>, Self::Error> {
        self.0.get_raw(key).map_err(Into::into)
    }

    fn put_raw(&mut self, key: impl AsRef<str>, value: impl Into<alloc::vec::Vec<u8>>) -> Result<bool, Self::Error> {
        self.0.put_raw(key, value).map_err(Into::into)
    }

    #[cfg(feature = "use_serde")]
    fn get<T: serde::de::DeserializeOwned>(&self, key: impl AsRef<str>) -> Result<Option<T>, Self::Error> {
        self.0.get(key).map_err(Into::into)
    }

    #[cfg(feature = "use_serde")]
    fn put(&mut self, key: impl AsRef<str>, value: &impl serde::Serialize) -> Result<bool, Self::Error> {
        self.0.put(key, value).map_err(Into::into)
    }
}

pub struct MemoryStorage(alloc::collections::BTreeMap<alloc::string::String, alloc::vec::Vec<u8>>);

impl MemoryStorage {
    pub fn new() -> Self {
        Self(alloc::collections::BTreeMap::new())
    }
}

impl Storage for MemoryStorage {
    type Error = core::convert::Infallible;

    fn contains(&self, key: impl AsRef<str>) -> Result<bool, Self::Error> {
        Ok(self.0.contains_key(key.as_ref()))
    }

    fn remove(&mut self, key: impl AsRef<str>) -> Result<bool, Self::Error> {
        Ok(self.0.remove(key.as_ref()).is_some())
    }

    fn get_raw(&self, key: impl AsRef<str>) -> Result<Option<alloc::vec::Vec<u8>>, Self::Error> {
        Ok(self.0.get(key.as_ref()).map(Clone::clone))
    }

    fn put_raw(&mut self, key: impl AsRef<str>, value: impl Into<alloc::vec::Vec<u8>>) -> Result<bool, Self::Error> {
        Ok(if let Some(r) = self.0.get_mut(key.as_ref()) {
            *r = value.into();
            true
        } else {
            self.0.insert(key.as_ref().to_owned(), value.into()).is_some()
        })
    }
}

pub struct StorageCache<T: Storage>(T, core::cell::RefCell<MemoryStorage>);

impl<T: Storage> StorageCache<T> {
    pub fn new(storage: T) -> Self {
        Self(storage, core::cell::RefCell::new(MemoryStorage::new()))
    }

    pub fn flush(&mut self) {
        *&mut self.1 = core::cell::RefCell::new(MemoryStorage::new());
    }
}

impl<T: Storage> Storage for StorageCache<T> {
    type Error = T::Error;

    fn contains(&self, key: impl AsRef<str>) -> Result<bool, Self::Error> {
        Ok(self.1.borrow().contains(&key).unwrap() || self.0.contains(key)?)
    }

    fn remove(&mut self, key: impl AsRef<str>) -> Result<bool, Self::Error> {
        self.1.borrow_mut().remove(&key).unwrap(); // TODO
        self.0.remove(key)
    }

    fn get_raw(&self, key: impl AsRef<str>) -> Result<Option<alloc::vec::Vec<u8>>, Self::Error> {
        if let Some(data) = self.1.borrow().get_raw(&key).unwrap() {
            Ok(Some(data))
        } else {
            if let Some(data) = self.0.get_raw(&key)? {
                self.1.borrow_mut().put_raw(&key, data.clone()).unwrap(); // TODO
                Ok(Some(data))
            } else {
                self.1.borrow_mut().remove(key).unwrap();
                Ok(None)
            }
        }
    }

    fn put_raw(&mut self, key: impl AsRef<str>, value: impl Into<alloc::vec::Vec<u8>>) -> Result<bool, Self::Error> {
        let value = value.into();
        self.1.borrow_mut().put_raw(&key, value.clone()).unwrap(); // TODO
        self.0.put_raw(key, value)
    }
}
