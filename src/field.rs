struct Field<T>(T);

impl<T> Field<T> {
    pub fn new(val: T) -> Self {
        Self(val)
    }

    pub fn set(&mut self, val: T) {
        self.0 = val;
    }

    pub fn get(&self) -> &T {
        &self.0
    }
}
