use crate::{chunk::Value, object::ObjString};

const INIT_TABLE_SIZE: usize = 256;

#[derive(Clone)]
pub enum EntryState {
    Empty,
    Deleted,
    Occupied(Entry),
}

impl EntryState {
    pub fn is_empty(&self) -> bool {
        matches!(&self, EntryState::Empty)
    }
    pub fn is_deleted(&self) -> bool {
        matches!(&self, EntryState::Deleted)
    }
    pub fn is_occuppied(&self) -> bool {
        matches!(&self, EntryState::Occupied(_))
    }
}

#[derive(Clone)]
pub struct Entry {
    /// Q(String Interning): Maybe we can change `ObjString` to `ObjId` to save the memory.
    pub k: ObjString,
    pub v: Value,
}

impl Entry {
    pub fn new(k: ObjString, v: Value) -> Self {
        Self { k, v }
    }
}

/// Hash table
#[derive(Clone)]
pub struct Table {
    count: usize,
    capacity: usize,
    /// Use `Vec` to ensure the array could be expanded as we needed at runtime.
    values: Vec<EntryState>,
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
            values: vec![EntryState::Empty; INIT_TABLE_SIZE],
        }
    }

    /// Find the index of given key `k` and return the index else return the first
    /// empty index.
    pub fn find_index(
        table: &[EntryState],
        k: &ObjString,
        capacity: usize,
    ) -> Result<usize, usize> {
        let mut idx = (k.hash % capacity as u64) as usize;
        loop {
            match &table[idx] {
                EntryState::Occupied(e) if e.k == *k => return Ok(idx),
                EntryState::Occupied(_) | EntryState::Deleted => idx = (idx + 1) % capacity,
                EntryState::Empty => return Err(idx),
            }
        }
    }

    /// Get a immutable reference of the value of the key `k`.
    pub fn get(&self, k: &ObjString) -> Option<&Entry> {
        match Self::find_index(&self.values, k, self.capacity) {
            Ok(idx) => match &self.values[idx] {
                EntryState::Occupied(e) => Some(e),
                _ => {
                    unreachable!()
                }
            },
            Err(_) => None,
        }
    }

    /// Get a mutable reference of the value of the key `k`.
    pub fn get_mut(&mut self, k: &ObjString) -> Option<&mut Entry> {
        match Self::find_index(&self.values, k, self.capacity) {
            Ok(idx) => match &mut self.values[idx] {
                EntryState::Occupied(e) => Some(e),
                _ => {
                    unreachable!()
                }
            },
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
        self.values[idx] = EntryState::Occupied(Entry::new(k, v));
    }

    /// Delete the value of given key `k`.
    pub fn del(&mut self, k: &ObjString) {
        match Self::find_index(&self.values, k, self.capacity) {
            Ok(idx) => self.values[idx] = EntryState::Deleted,
            Err(_) => println!("The key {} not found.", k),
        }
    }

    /// Increase capacity of hash table and re-insert hash data.
    pub fn adjust_capacity(&mut self, capacity: usize) {
        self.capacity = capacity;
        let mut new_table: Vec<EntryState> = vec![EntryState::Empty; capacity];
        let mut old_table = std::mem::take(&mut self.values);
        self.table_transfer(&mut old_table, &mut new_table, capacity);
        self.values = new_table;
    }

    /// Transfer a hash table from `src` to `dest`.
    ///
    /// The hash data will be re-insert to the new table according to the capacity.
    pub fn table_transfer(
        &mut self,
        src: &mut Vec<EntryState>,
        dest: &mut [EntryState],
        capacity: usize,
    ) {
        self.count = 0;
        let old_entries = std::mem::take(src);
        for old_entry_state in old_entries {
            if let EntryState::Occupied(old_entry) = old_entry_state {
                match Self::find_index(dest, &old_entry.k, capacity) {
                    Ok(_) => {
                        // Impossible to find a existing key in new table.
                        unreachable!()
                    }
                    Err(idx) => {
                        dest[idx] = EntryState::Occupied(old_entry);
                        self.count += 1;
                    }
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
        assert_eq!(table.get(&k1).unwrap().v, v1);
        assert_eq!(table.count, 1);

        // Test multiple insertions
        table.set(make_string("world"), Value::Number(2.0));
        assert_eq!(table.count, 2);

        // Test nonexistent key
        assert!(table.get(&make_string("nonexistent")).is_none());
    }

    #[test]
    fn test_table_delete_and_tombstones() {
        let mut table = Table::new();
        let k1 = make_string("key1");
        let k2 = make_string("key2");

        table.set(k1.clone(), Value::Number(1.0));
        table.set(k2.clone(), Value::Number(2.0));
        assert_eq!(table.count, 2);

        // Delete leaves tombstone, count unchanged
        table.del(&k1);
        assert_eq!(table.count, 2);
        assert!(table.get(&k1).is_none());

        // Delete nonexistent key
        let initial_count = table.count;
        table.del(&make_string("nonexistent"));
        assert_eq!(table.count, initial_count);
    }

    #[test]
    fn test_table_transfer_with_tombstones() {
        let mut table = Table::new();

        // Insert entries
        for i in 0..5 {
            let key = format!("key{}", i);
            table.set(make_string(&key), Value::Number(i as f64));
        }
        assert_eq!(table.count, 5);

        // Delete some entries
        table.del(&make_string("key1"));
        table.del(&make_string("key3"));

        // Tombstones don't reduce count in current implementation
        assert_eq!(table.count, 5);

        // Trigger resize - tombstones should be discarded
        let old_capacity = table.capacity;
        table.adjust_capacity(old_capacity * 2);

        // After transfer, count should reflect only occupied entries (5 - 2 deleted)
        assert_eq!(table.count, 3);

        // Verify we still can get the non-deleted entries
        assert!(table.get(&make_string("key0")).is_some());
        assert!(table.get(&make_string("key1")).is_none());
        assert!(table.get(&make_string("key2")).is_some());
        assert!(table.get(&make_string("key3")).is_none());
        assert!(table.get(&make_string("key4")).is_some());
    }

    #[test]
    fn test_string_interning_same_content() {
        let mut table = Table::new();

        // Insert the same string content with different ObjString instances
        let s1 = make_string("interned");
        let v1 = Value::Number(1.0);
        table.set(s1.clone(), v1);

        // Create another ObjString with the same content
        let s2 = make_string("interned");

        // The hashes should be the same (both map to same slot)
        assert_eq!(s1.hash, s2.hash);

        // We should be able to retrieve using the second string
        let result = table.get(&s2);
        assert!(result.is_some());
        assert_eq!(result.unwrap().v, v1);
    }

    #[test]
    fn test_string_interning_multiple_same_strings() {
        let mut table = Table::new();

        // Insert multiple entries with same-content keys
        table.set(make_string("key"), Value::Number(1.0));
        table.set(make_string("key"), Value::Number(2.0));
        table.set(make_string("key"), Value::Number(3.0));

        // Should only have 1 entry (updates, not new insertions)
        assert_eq!(table.count, 1);

        // The value should be the last one inserted
        let result = table.get(&make_string("key"));
        assert!(result.is_some());
        assert_eq!(result.unwrap().v, Value::Number(3.0));
    }
}
