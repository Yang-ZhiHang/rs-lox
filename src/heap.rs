use crate::object::{ObjData, ObjString};

pub struct Heap {
    /// The list of object.
    objs: Vec<ObjData>,
}

impl Default for Heap {
    fn default() -> Self {
        Self::new()
    }
}

impl Heap {
    /// Creates a heap with empty object list.
    pub fn new() -> Self {
        Self { objs: vec![] }
    }

    /// Get a immutable reference of object by index.
    pub fn get(&self, idx: usize) -> &ObjData {
        &self.objs[idx]
    }

    /// Get a mutable reference of object by index.
    pub fn get_mut(&mut self, idx: usize) -> &mut ObjData {
        &mut self.objs[idx]
    }

    /// Write the object into heap (object list) and return the index.
    pub fn write(&mut self, obj: ObjData) -> usize {
        self.objs.push(obj);
        self.objs.len() - 1
    }

    /// Write the string object into heap and return the index.
    pub fn write_string(&mut self, s: &str) -> usize {
        let obj = ObjData::String(ObjString::new(s));
        self.write(obj)
    }
}
