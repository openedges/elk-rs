use std::fmt;

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Maybe<T> {
    object: Option<T>,
}

impl<T> Maybe<T> {
    pub fn create() -> Self {
        Self::new()
    }

    pub fn new() -> Self {
        Maybe { object: None }
    }

    pub fn with(object: T) -> Self {
        Maybe {
            object: Some(object),
        }
    }

    pub fn set(&mut self, object: T) {
        self.object = Some(object);
    }

    pub fn get(&self) -> Option<&T> {
        self.object.as_ref()
    }

    pub fn get_mut(&mut self) -> Option<&mut T> {
        self.object.as_mut()
    }

    pub fn take(&mut self) -> Option<T> {
        self.object.take()
    }

    pub fn clear(&mut self) {
        self.object = None;
    }

    pub fn is_empty(&self) -> bool {
        self.object.is_none()
    }

    pub fn iter(&self) -> std::option::Iter<'_, T> {
        self.object.iter()
    }
}

impl<T> IntoIterator for Maybe<T> {
    type Item = T;
    type IntoIter = std::option::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.object.into_iter()
    }
}

impl<'a, T> IntoIterator for &'a Maybe<T> {
    type Item = &'a T;
    type IntoIter = std::option::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.object.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut Maybe<T> {
    type Item = &'a mut T;
    type IntoIter = std::option::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.object.iter_mut()
    }
}

impl<T: fmt::Display> fmt::Display for Maybe<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.object {
            Some(value) => write!(f, "maybe({value})"),
            None => write!(f, "maybe(null)"),
        }
    }
}
