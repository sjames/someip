struct Field<T>(T);

impl<T> Field<T> {
    pub fn new(val: T) -> Self {
        Self(val)
    }

    pub async fn set(&mut self, val: T) {
        self.0 = val;
    }

    pub async fn get(&self) -> &T {
        &self.0
    }
}
