pub struct Constant {
    values: Vec<f64>,
}

impl Constant {
    /// Create constant area with empty vector.
    pub fn new() -> Self {
        Self { values: vec![] }
    }

    /// Write a constant value to the constant area and return the value index
    /// in the constant area.
    pub fn write(&mut self, value: f64) -> usize {
        self.values.push(value);
        self.values.len() - 1
    }

    /// Getter of member `values`.
    pub fn values(&self, index: usize) -> f64 {
        self.values[index]
    }
}
