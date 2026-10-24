use crate::error::RuntimeError;
use crate::value::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HeapHandle {
    pub slot: usize,
    pub generation: u64,
}

#[derive(Debug, Clone)]
struct HeapSlot {
    generation: u64,
    value: Option<Value>,
}

#[derive(Debug, Clone, Default)]
pub struct HeapManager {
    slots: Vec<HeapSlot>,
}

impl HeapManager {
    pub fn new() -> Self {
        Self { slots: Vec::new() }
    }

    pub fn alloc(&mut self, value: Value) -> HeapHandle {
        // Find existing freed slot if any
        for (i, slot) in self.slots.iter_mut().enumerate() {
            if slot.value.is_none() {
                slot.value = Some(value);
                return HeapHandle {
                    slot: i,
                    generation: slot.generation,
                };
            }
        }

        // Otherwise push a new slot
        let slot_idx = self.slots.len();
        self.slots.push(HeapSlot {
            generation: 1,
            value: Some(value),
        });
        HeapHandle {
            slot: slot_idx,
            generation: 1,
        }
    }

    pub fn get(&self, handle: HeapHandle) -> Result<&Value, RuntimeError> {
        if let Some(slot) = self.slots.get(handle.slot) {
            if slot.generation == handle.generation {
                if let Some(ref val) = slot.value {
                    return Ok(val);
                }
            }
        }
        Err(RuntimeError::new(format!(
            "invalid or stale heap handle (slot={}, gen={})",
            handle.slot, handle.generation
        )))
    }

    pub fn get_mut(&mut self, handle: HeapHandle) -> Result<&mut Value, RuntimeError> {
        if let Some(slot) = self.slots.get_mut(handle.slot) {
            if slot.generation == handle.generation {
                if let Some(ref mut val) = slot.value {
                    return Ok(val);
                }
            }
        }
        Err(RuntimeError::new(format!(
            "invalid or stale heap handle (slot={}, gen={})",
            handle.slot, handle.generation
        )))
    }

    pub fn free(&mut self, handle: HeapHandle) -> Result<(), RuntimeError> {
        if let Some(slot) = self.slots.get_mut(handle.slot) {
            if slot.generation == handle.generation && slot.value.is_some() {
                slot.value = None;
                slot.generation += 1;
                return Ok(());
            }
        }
        Err(RuntimeError::new(format!(
            "double-free or invalid heap handle (slot={}, gen={})",
            handle.slot, handle.generation
        )))
    }
}
