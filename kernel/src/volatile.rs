pub struct Volatile<T> {
    value: T,
}

impl<T: Copy> Copy for Volatile<T> {}
impl<T: Clone> Clone for Volatile<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
        }
    }
}

impl<T> From<T> for Volatile<T> {
    fn from(value: T) -> Self {
        Self { value }
    }
}

impl<T> Volatile<T> {
    pub fn new(value: T) -> Self {
        value.into()
    }

    pub fn read(&self) -> T
    where
        T: Copy,
    {
        unsafe { core::ptr::read_volatile(&self.value) }
    }

    pub fn write(&mut self, value: T) {
        unsafe { core::ptr::write_volatile(&mut self.value, value) }
    }
}
