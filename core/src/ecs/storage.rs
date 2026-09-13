use super::entity::Entity;

/// Sparse component storage keyed by [`Entity`]. Indexed directly by the entity's index, with
/// the stored generation checked on every access so a stale `Entity` (held past a despawn) never
/// reads or writes whatever new component ends up in the recycled slot.
pub struct ComponentStore<T> {
    slots: Vec<Option<(u32, T)>>,
}

impl<T> ComponentStore<T> {
    pub fn new() -> Self {
        Self { slots: Vec::new() }
    }

    pub fn insert(&mut self, entity: Entity, value: T) {
        let index = entity.index as usize;
        if index >= self.slots.len() {
            self.slots.resize_with(index + 1, || None);
        }
        self.slots[index] = Some((entity.generation, value));
    }

    pub fn remove(&mut self, entity: Entity) -> Option<T> {
        let slot = self.slots.get_mut(entity.index as usize)?;
        if slot.as_ref().map(|(generation, _)| *generation) == Some(entity.generation) {
            slot.take().map(|(_, value)| value)
        } else {
            None
        }
    }

    pub fn get(&self, entity: Entity) -> Option<&T> {
        self.slots.get(entity.index as usize)?.as_ref().and_then(|(generation, value)| {
            (*generation == entity.generation).then_some(value)
        })
    }

    pub fn get_mut(&mut self, entity: Entity) -> Option<&mut T> {
        self.slots.get_mut(entity.index as usize)?.as_mut().and_then(|(generation, value)| {
            (*generation == entity.generation).then_some(value)
        })
    }

    pub fn iter(&self) -> impl Iterator<Item = (Entity, &T)> {
        self.slots.iter().enumerate().filter_map(|(i, slot)| {
            slot.as_ref().map(|(generation, value)| (Entity { index: i as u32, generation: *generation }, value))
        })
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Entity, &mut T)> {
        self.slots.iter_mut().enumerate().filter_map(|(i, slot)| {
            slot.as_mut().map(|(generation, value)| (Entity { index: i as u32, generation: *generation }, value))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::entity::EntityAllocator;

    #[test]
    fn stale_entity_cannot_read_recycled_slot() {
        let mut allocator = EntityAllocator::new();
        let mut store = ComponentStore::new();

        let a = allocator.spawn();
        store.insert(a, "a");
        allocator.despawn(a);

        let b = allocator.spawn();
        store.insert(b, "b");

        assert_eq!(store.get(a), None);
        assert_eq!(store.get(b), Some(&"b"));
    }

    #[test]
    fn insert_then_get_round_trips() {
        let mut allocator = EntityAllocator::new();
        let mut store = ComponentStore::new();
        let a = allocator.spawn();
        store.insert(a, 42);
        assert_eq!(store.get(a), Some(&42));
    }
}
