use std::collections::HashMap;

use crate::{
    chunk::Value,
    object::{ObjClosure, ObjData, ObjFunction, ObjIndex, ObjNative, ObjString, ObjUpvalue},
};

pub struct Heap {
    /// The list of object.
    objs: Vec<ObjData>,
    /// Use to judge if a string is already allocated. If so, we could return the same index to the string instead
    /// of a new string to save the memory and perform better string matching in hash table.
    interned_strings: HashMap<String, usize>,
}

impl Default for Heap {
    fn default() -> Self {
        Self::new()
    }
}

impl Heap {
    /// Creates a heap with empty object list.
    pub fn new() -> Self {
        Self {
            objs: vec![],
            interned_strings: HashMap::new(),
        }
    }

    /// Get a immutable reference of object by index.
    pub fn get(&self, idx: ObjIndex) -> &ObjData {
        &self.objs[idx.val]
    }

    /// Get a mutable reference of object by index.
    pub fn get_mut(&mut self, idx: ObjIndex) -> &mut ObjData {
        &mut self.objs[idx.val]
    }

    #[inline(always)]
    /// Return a immutable reference of string object.
    pub fn get_string(&self, idx: ObjIndex) -> &ObjString {
        match self.objs.get(idx.val) {
            Some(ObjData::String(s)) => s,
            _ => unreachable!(),
        }
    }

    #[inline(always)]
    /// Return a immutable reference of function object.
    /// Ensure the passing-in index is a index of function object.
    pub fn get_func(&self, idx: ObjIndex) -> &ObjFunction {
        match self.objs.get(idx.val) {
            Some(ObjData::Function(f)) => f,
            _ => unreachable!(),
        }
    }

    #[inline(always)]
    /// Return a mutable reference of function object.
    /// Ensure the passing-in index is a index of function object.
    pub fn get_func_mut(&mut self, idx: ObjIndex) -> &mut ObjFunction {
        match self.objs.get_mut(idx.val) {
            Some(ObjData::Function(f)) => f,
            _ => unreachable!(),
        }
    }

    #[inline(always)]
    /// Return a immutable reference of closure object.
    /// Ensure the passing-in index is a index of closure object.
    pub fn get_closure(&self, idx: ObjIndex) -> &ObjClosure {
        match self.objs.get(idx.val) {
            Some(ObjData::Closure(c)) => c,
            _ => unreachable!(),
        }
    }

    #[inline(always)]
    /// Return a immutable reference of upvalue object.
    /// Ensure the passing-in index is a index of upvalue object.
    pub fn get_upvalue(&self, idx: ObjIndex) -> &ObjUpvalue {
        match self.objs.get(idx.val) {
            Some(ObjData::Upvalue(v)) => v,
            _ => unreachable!(),
        }
    }

    #[inline(always)]
    /// Return a mutable reference of upvalue object.
    /// Ensure the passing-in index is a index of upvalue object.
    pub fn get_upvalue_mut(&mut self, idx: ObjIndex) -> &mut ObjUpvalue {
        match self.objs.get_mut(idx.val) {
            Some(ObjData::Upvalue(v)) => v,
            _ => unreachable!(),
        }
    }

    /// Write the object into heap (object list) and return the index.
    fn write(&mut self, obj: ObjData) -> usize {
        self.objs.push(obj);
        self.objs.len() - 1
    }

    /// Write the object into heap at the given index.
    pub fn write_at(&mut self, obj_idx: ObjIndex, obj: ObjData) {
        self.objs[obj_idx.val] = obj;
    }

    /// Write the string object into heap and return the index.
    ///
    /// Returning a same string's object index If a same string already exists in the heap.
    pub fn write_string(&mut self, s: &str) -> ObjIndex {
        if self.interned_strings.contains_key(s) {
            return self.interned_strings.get(s).copied().unwrap().into();
        }
        let obj = ObjData::String(ObjString::new(s));
        let idx = self.write(obj);
        self.interned_strings.insert(s.to_string(), idx);
        ObjIndex::new(idx)
    }

    /// Write the function object into heap and return the index.
    pub fn write_func(&mut self, name_id: ObjIndex, arity: usize) -> ObjIndex {
        let func = ObjData::Function(ObjFunction::new(name_id, arity));
        let idx = self.write(func);
        ObjIndex::new(idx)
    }

    /// Write the native function object into heap and return the index.
    pub fn write_native_func(
        &mut self,
        func: fn(usize, &[Option<Value>], &Heap) -> Value,
    ) -> ObjIndex {
        let native_func = ObjData::Native(ObjNative::new(func));
        let idx = self.write(native_func);
        ObjIndex::new(idx)
    }

    /// Write the closure object into heap and return the index.
    pub fn write_closure(&mut self, closure: ObjClosure) -> ObjIndex {
        let idx = self.write(ObjData::Closure(Box::new(closure)));
        ObjIndex::new(idx)
    }

    /// Write the upvalue object into heap and return the index.
    pub fn write_upvalue(&mut self, upvalue: ObjUpvalue) -> ObjIndex {
        let idx = self.write(ObjData::Upvalue(upvalue));
        ObjIndex::new(idx)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_intern() {
        let mut heap = Heap::new();
        let idx1 = heap.write_string("test_intern");
        let idx2 = heap.write_string("test_intern");
        assert_eq!(idx1, idx2);
    }
}
