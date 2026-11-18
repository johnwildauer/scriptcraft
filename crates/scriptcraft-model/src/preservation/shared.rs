use std::ops::{Deref, DerefMut};
use std::sync::Arc;

/// A vector whose clones share immutable storage and detach on mutable access.
/// Its vector-like API keeps the preservation tree's ordered semantic surface.
#[derive(Debug)]
pub struct SharedVec<T>(Arc<Vec<T>>);

impl<T> SharedVec<T> {
    pub fn new(values: Vec<T>) -> Self {
        Self(Arc::new(values))
    }

    pub fn shares_storage_with(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl<T> Clone for SharedVec<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<T> Default for SharedVec<T> {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl<T> From<Vec<T>> for SharedVec<T> {
    fn from(values: Vec<T>) -> Self {
        Self::new(values)
    }
}

impl<T> Deref for SharedVec<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Clone> DerefMut for SharedVec<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        Arc::make_mut(&mut self.0)
    }
}

impl<T: PartialEq> PartialEq for SharedVec<T> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0) || **self == **other
    }
}

impl<T: Eq> Eq for SharedVec<T> {}

impl<T> AsRef<[T]> for SharedVec<T> {
    fn as_ref(&self) -> &[T] {
        self.0.as_slice()
    }
}

impl<T> FromIterator<T> for SharedVec<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self::new(iter.into_iter().collect())
    }
}

impl<T: Clone> IntoIterator for SharedVec<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        match Arc::try_unwrap(self.0) {
            Ok(values) => values.into_iter(),
            Err(values) => values.as_ref().clone().into_iter(),
        }
    }
}

impl<'a, T> IntoIterator for &'a SharedVec<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T: Clone> IntoIterator for &'a mut SharedVec<T> {
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

/// An immutable UTF-8 payload shared by preserved text, CDATA, and comments.
#[derive(Debug)]
pub struct SharedText(Arc<String>);

impl SharedText {
    pub fn shares_storage_with(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Clone for SharedText {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl From<String> for SharedText {
    fn from(value: String) -> Self {
        Self(Arc::new(value))
    }
}

impl From<&str> for SharedText {
    fn from(value: &str) -> Self {
        Self(Arc::new(value.to_owned()))
    }
}

impl Deref for SharedText {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for SharedText {
    fn deref_mut(&mut self) -> &mut Self::Target {
        Arc::make_mut(&mut self.0)
    }
}

impl PartialEq for SharedText {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0) || **self == **other
    }
}

impl Eq for SharedText {}

impl PartialEq<String> for SharedText {
    fn eq(&self, other: &String) -> bool {
        self.as_str() == other.as_str()
    }
}

impl PartialEq<str> for SharedText {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for SharedText {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<SharedText> for String {
    fn eq(&self, other: &SharedText) -> bool {
        self.as_str() == other.as_str()
    }
}

impl From<SharedText> for String {
    fn from(value: SharedText) -> Self {
        Arc::try_unwrap(value.0).unwrap_or_else(|shared| shared.as_ref().clone())
    }
}

/// Immutable byte storage used for original source and opaque image payloads.
#[derive(Debug)]
pub struct SharedBytes {
    bytes: Arc<Vec<u8>>,
    image_payload: bool,
}

impl SharedBytes {
    pub fn source(bytes: Vec<u8>) -> Self {
        Self {
            bytes: Arc::new(bytes),
            image_payload: false,
        }
    }

    pub fn image(bytes: Vec<u8>) -> Self {
        Self {
            bytes: Arc::new(bytes),
            image_payload: true,
        }
    }

    pub fn shares_storage_with(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.bytes, &other.bytes)
    }
}

impl Default for SharedBytes {
    fn default() -> Self {
        Self::source(Vec::new())
    }
}

impl Clone for SharedBytes {
    fn clone(&self) -> Self {
        Self {
            bytes: Arc::clone(&self.bytes),
            image_payload: self.image_payload,
        }
    }
}

impl From<Vec<u8>> for SharedBytes {
    fn from(bytes: Vec<u8>) -> Self {
        Self::source(bytes)
    }
}

impl Deref for SharedBytes {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.bytes
    }
}

impl DerefMut for SharedBytes {
    fn deref_mut(&mut self) -> &mut Self::Target {
        Arc::make_mut(&mut self.bytes)
    }
}

impl PartialEq for SharedBytes {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.bytes, &other.bytes) || **self == **other
    }
}

impl Eq for SharedBytes {}

impl PartialEq<Vec<u8>> for SharedBytes {
    fn eq(&self, other: &Vec<u8>) -> bool {
        **self == *other
    }
}

impl<const N: usize> PartialEq<&[u8; N]> for SharedBytes {
    fn eq(&self, other: &&[u8; N]) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl<const N: usize> PartialEq<[u8; N]> for SharedBytes {
    fn eq(&self, other: &[u8; N]) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl PartialEq<SharedBytes> for Vec<u8> {
    fn eq(&self, other: &SharedBytes) -> bool {
        self.as_slice() == other.as_slice()
    }
}
