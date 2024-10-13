use std::marker::PhantomData;
use windows::core::Error;
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT};

pub(crate) struct ComThreadGuard(PhantomData<()>);

impl ComThreadGuard {
    pub(crate) fn new(coinit: COINIT) -> Result<Self, Error> {
        // SAFETY: Our drop impl ensures that COM is uninitialized.
        let result = unsafe { CoInitializeEx(None, coinit) };
        result.map(|| Self(PhantomData))
    }
}

impl Drop for ComThreadGuard {
    fn drop(&mut self) {
        // SAFETY: Instances of this type are only created
        // when COM was successfully initialized.
        unsafe { CoUninitialize() };
    }
}
