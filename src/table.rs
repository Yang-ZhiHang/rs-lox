use crate::{chunk::Value, object::ObjString};

const INIT_TABLE_SIZE: usize = 256;

#[derive(Clone)]
pub struct Entry {
    k: ObjString,
    v: Value,
}

impl Entry {
    pub fn new(k: ObjString, v: Value) -> Self {
        Self { k, v }
    }
}

/// Hash table
pub struct Table {
    count: usize,
    capacity: usize,
    /// Use `Vec` to ensure the array could be expanded as we needed at runtime.
    values: Vec<Option<Entry>>,
}

impl Default for Table {
    fn default() -> Self {
        Self::new()
    }
}

impl Table {
    /// Creates a empty hash table.
    pub fn new() -> Self {
        Self {
            count: 0,
            capacity: INIT_TABLE_SIZE,
            values: vec![None; INIT_TABLE_SIZE],
        }
    }

    /// Find the index of given key `k` and return the index else return the first
    /// empty index.
    pub fn find_index(&self, k: &ObjString, capacity: usize) -> Result<usize, usize> {
        let mut idx = (k.hash % capacity as u64) as usize;
        loop {
            match &self.values[idx] {
                Some(entry) if entry.k == *k => return Ok(idx),
                Some(_) => idx = (idx + 1) % capacity,
                None => return Err(idx),
            }
        }
    }

    /// Get a immutable reference of the value of the key `k`.
    pub fn get(&self, k: &ObjString) -> Option<&Entry> {
        match self.find_index(k, self.capacity) {
            Ok(idx) => self.values[idx].as_ref(),
            Err(_) => None,
        }
    }

    /// Get a mutable reference of the value of the key `k`.
    pub fn get_mut(&mut self, k: &ObjString) -> Option<&mut Entry> {
        match self.find_index(k, self.capacity) {
            Ok(idx) => self.values[idx].as_mut(),
            Err(_) => None,
        }
    }

    /// Set the value of the key `k`.
    pub fn set(&mut self, k: ObjString, v: Value) {
        if let Some(e) = self.get_mut(&k) {
            e.v = v;
            return;
        }
        self.count += 1;
        // The vector will automatically expand, we need re-insert the hash entry.
        if self.count > self.capacity {
            self.adjust_capacity(self.capacity * 2);
        }
        let idx = (k.hash % self.capacity as u64) as usize;
        self.values[idx] = Some(Entry::new(k, v));
    }

    /// Delete the value of given key `k`.
    pub fn del(&mut self, k: &ObjString) {
        match self.find_index(k, self.capacity) {
            Ok(idx) => self.values[idx] = None,
            Err(_) => println!("The key {} not found.", k),
        }
    }

    /// Increase capacity of hash table and re-insert hash data.
    pub fn adjust_capacity(&mut self, capacity: usize) {
        self.capacity = capacity;
        let mut new_table: Vec<Option<Entry>> = Vec::with_capacity(capacity);
        let mut old_table = std::mem::take(&mut self.values);
        self.table_transfer(&mut old_table, &mut new_table, capacity);
        self.values = new_table;
    }

    /// Transfer a hash table from `src` to `dest`.
    ///
    /// The hash data will be re-insert to the new table according to the capacity.
    pub fn table_transfer(
        &self,
        src: &mut Vec<Option<Entry>>,
        dest: &mut [Option<Entry>],
        capacity: usize,
    ) {
        let old_entries = std::mem::take(src);
        for old_entry in old_entries.into_iter().flatten() {
            match self.find_index(&old_entry.k, capacity) {
                Err(idx) => {
                    dest[idx] = Some(old_entry);
                }
                Ok(_) => {
                    // Unreachable
                    panic!("Collision occured.")
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_string(s: &str) -> ObjString {
        ObjString::new(s)
    }

    #[test]
    fn test_table_insert_and_get() {
        let mut table = Table::new();
        let k1 = make_string("hello");
        let v1 = Value::Number(42.0);

        table.set(k1.clone(), v1);

        let result = table.get(&k1);
        assert!(result.is_some());
        assert_eq!(result.unwrap().v, v1);
    }

    #[test]
    fn test_table_insert_multiple_values() {
        let mut table = Table::new();
        let k1 = make_string("key1");
        let k2 = make_string("key2");
        let k3 = make_string("key3");

        let v1 = Value::Number(1.0);
        let v2 = Value::Number(2.0);
        let v3 = Value::Number(3.0);

        table.set(k1.clone(), v1);
        table.set(k2.clone(), v2);
        table.set(k3.clone(), v3);

        assert_eq!(table.get(&k1).unwrap().v, v1);
        assert_eq!(table.get(&k2).unwrap().v, v2);
        assert_eq!(table.get(&k3).unwrap().v, v3);
    }

    #[test]
    fn test_table_get_nonexistent_key() {
        let table = Table::new();
        let k = make_string("nonexistent");

        let result = table.get(&k);
        assert!(result.is_none());
    }

    #[test]
    fn test_table_update_existing_value() {
        let mut table = Table::new();
        let k = make_string("key");
        let v1 = Value::Number(1.0);
        let v2 = Value::Number(2.0);

        table.set(k.clone(), v1);
        assert_eq!(table.get(&k).unwrap().v, v1);

        table.set(k.clone(), v2);
        assert_eq!(table.get(&k).unwrap().v, v2);
        assert_eq!(table.count, 1); // Still only one entry
    }

    #[test]
    fn test_table_count() {
        let mut table = Table::new();
        assert_eq!(table.count, 0);

        table.set(make_string("k1"), Value::Number(1.0));
        assert_eq!(table.count, 1);

        table.set(make_string("k2"), Value::Number(2.0));
        assert_eq!(table.count, 2);

        table.set(make_string("k3"), Value::Number(3.0));
        assert_eq!(table.count, 3);
    }

    #[test]
    fn test_table_find_index_existing() {
        let mut table = Table::new();
        let k = make_string("test_key");
        let v = Value::Bool(true);

        table.set(k.clone(), v);

        let result = table.find_index(&k, table.capacity);
        assert!(result.is_ok());
        let idx = result.unwrap();
        assert!(table.values[idx].is_some());
    }

    #[test]
    fn test_table_find_index_empty_slot() {
        let table = Table::new();
        let k = make_string("unused_key");

        let result = table.find_index(&k, table.capacity);
        assert!(result.is_err());
        let empty_idx = result.unwrap_err();
        assert!(table.values[empty_idx].is_none());
    }

    #[test]
    fn test_table_collision_handling() {
        // Test that linear probing handles collisions correctly
        let mut table = Table::new();
        let k1 = make_string("a");
        let k2 = make_string("b");

        table.set(k1.clone(), Value::Number(1.0));
        table.set(k2.clone(), Value::Number(2.0));

        // Both keys should be retrievable
        assert!(table.get(&k1).is_some());
        assert!(table.get(&k2).is_some());
    }

    #[test]
    fn test_table_bool_and_nil_values() {
        let mut table = Table::new();

        table.set(make_string("bool_true"), Value::Bool(true));
        table.set(make_string("bool_false"), Value::Bool(false));
        table.set(make_string("nil"), Value::Nil);

        assert_eq!(
            table.get(&make_string("bool_true")).unwrap().v,
            Value::Bool(true)
        );
        assert_eq!(
            table.get(&make_string("bool_false")).unwrap().v,
            Value::Bool(false)
        );
        assert_eq!(table.get(&make_string("nil")).unwrap().v, Value::Nil);
    }

    #[test]
    fn test_table_capacity_and_count() {
        let table = Table::new();
        assert_eq!(table.capacity, INIT_TABLE_SIZE);
        assert_eq!(table.count, 0);
    }
}
