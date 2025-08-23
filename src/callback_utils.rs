use std::{error, fmt};

/// A collection of callbacks optimized for
/// iteration and adding while being slow on removal.
#[derive(Debug)]
pub(crate) struct Callbacks<T> {
    callbacks: Vec<Callback<T>>,
    next_handle: usize,
}

impl<T> Callbacks<T> {
    pub(crate) const fn new() -> Self {
        /// The first handle is 1 and not 0 so that
        /// we can have an "invalid" handle that never matches anything.
        const FIRST_HANDLE: usize = 1;
        Self {
            callbacks: Vec::new(),
            next_handle: FIRST_HANDLE,
        }
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &T> {
        self.callbacks.iter().map(|c| &c.value)
    }

    pub(crate) fn add(&mut self, value: T) -> Result<CallbackHandle, OverflowError> {
        let handle = self.next_handle;
        self.next_handle = self.next_handle.checked_add(1).ok_or(OverflowError)?;
        self.callbacks.push(Callback { handle, value });
        Ok(CallbackHandle(handle))
    }

    pub(crate) fn remove(&mut self, handle: CallbackHandle) {
        self.callbacks.retain(|c| c.handle != handle.0);
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct OverflowError;

impl fmt::Display for OverflowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("failed to add callback: no more handles left")
    }
}

impl error::Error for OverflowError {}

/// An opaque handle used to remove a callback.
#[derive(Debug, Default)]
#[must_use]
pub(crate) struct CallbackHandle(usize);

#[derive(Debug)]
struct Callback<T> {
    handle: usize,
    value: T,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds_an_entry() {
        let mut callbacks = Callbacks::new();
        let _foo = callbacks.add("foo");
        assert_eq!(vec![&"foo"], callbacks.iter().collect::<Vec<_>>());
    }

    #[test]
    fn adds_multiple_entries() {
        let mut callbacks = Callbacks::new();
        let _foo = callbacks.add("foo").expect("no overflow");
        let _bar = callbacks.add("bar").expect("no overflow");
        let _baz = callbacks.add("baz").expect("no overflow");
        assert_eq!(
            vec![&"foo", &"bar", &"baz"],
            callbacks.iter().collect::<Vec<_>>()
        );
    }

    #[test]
    fn removes_an_entry() {
        let mut callbacks = Callbacks::new();
        let _foo = callbacks.add("foo").expect("no overflow");
        let bar = callbacks.add("bar").expect("no overflow");
        let _baz = callbacks.add("baz").expect("no overflow");
        callbacks.remove(bar);
        assert_eq!(vec![&"foo", &"baz"], callbacks.iter().collect::<Vec<_>>());
    }

    #[test]
    fn removes_all_entry() {
        let mut callbacks = Callbacks::new();
        let foo = callbacks.add("foo").expect("no overflow");
        let bar = callbacks.add("bar").expect("no overflow");
        let baz = callbacks.add("baz").expect("no overflow");
        callbacks.remove(bar);
        callbacks.remove(baz);
        callbacks.remove(foo);
        assert_eq!(0, callbacks.iter().count());
    }

    #[test]
    fn default_handle_does_not_remove_first_entry() {
        let mut callbacks = Callbacks::new();
        let _foo = callbacks.add("foo").expect("no overflow");
        callbacks.remove(CallbackHandle::default());
        assert_eq!(vec![&"foo"], callbacks.iter().collect::<Vec<_>>());
    }
}
