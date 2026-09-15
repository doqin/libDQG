/// A generational handle to an entity in a [`crate::world::World`].
///
/// The generation guards against a stale `Entity` (held after despawn) silently referring to
/// whatever new entity was later allocated at the same index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Entity {
    pub(crate) index: u32,
    pub(crate) generation: u32,
}

/// Allocates and recycles [`Entity`] indices, bumping an index's generation on despawn so any
/// `Entity` handle still held to it goes stale rather than aliasing whatever is spawned next.
#[derive(Clone)]
pub(crate) struct EntityAllocator {
    generations: Vec<u32>,
    free_list: Vec<u32>,
}

impl EntityAllocator {
    pub fn new() -> Self {
        Self { generations: Vec::new(), free_list: Vec::new() }
    }

    pub fn spawn(&mut self) -> Entity {
        if let Some(index) = self.free_list.pop() {
            Entity { index, generation: self.generations[index as usize] }
        } else {
            let index = self.generations.len() as u32;
            self.generations.push(0);
            Entity { index, generation: 0 }
        }
    }

    pub fn despawn(&mut self, entity: Entity) -> bool {
        if !self.is_alive(entity) {
            return false;
        }
        self.generations[entity.index as usize] = self.generations[entity.index as usize].wrapping_add(1);
        self.free_list.push(entity.index);
        true
    }

    pub fn is_alive(&self, entity: Entity) -> bool {
        self.generations.get(entity.index as usize).copied() == Some(entity.generation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recycled_index_gets_a_new_generation() {
        let mut allocator = EntityAllocator::new();
        let a = allocator.spawn();
        allocator.despawn(a);
        let b = allocator.spawn();

        assert_eq!(a.index, b.index);
        assert_ne!(a.generation, b.generation);
        assert!(!allocator.is_alive(a));
        assert!(allocator.is_alive(b));
    }

    #[test]
    fn distinct_spawns_get_distinct_indices() {
        let mut allocator = EntityAllocator::new();
        let a = allocator.spawn();
        let b = allocator.spawn();
        assert_ne!(a.index, b.index);
    }
}
