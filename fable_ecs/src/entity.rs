pub type EntityId = u32;

const INDEX_BITS: u32 = 20;
const INDEX_MASK: u32 = (1 << INDEX_BITS) - 1;

#[inline(always)]
fn make_entity(index: u32, generation: u32) -> EntityId {
    (generation << INDEX_BITS) | index
}

#[inline(always)]
fn entity_index(id: EntityId) -> usize {
    (id & INDEX_MASK) as usize
}

#[inline(always)]
fn entity_version(id: EntityId) -> u32 {
    id >> INDEX_BITS
}

#[derive(Clone, Copy)]
pub(crate) struct EntityLocation {
    pub(crate) archetype_id: u32,
    pub(crate) row: u32,
}

struct EntityEntry {
    location: EntityLocation,
    version: u32,
}

pub(crate) struct Entities {
    entries: Vec<EntityEntry>,
    free: Vec<u32>,
}

impl Entities {
    pub(crate) fn new() -> Entities {
        return Entities {
            entries: Vec::new(),
            free: Vec::new(),
        };
    }

    pub(crate) fn alloc(&mut self) -> EntityId {
        if let Some(index) = self.free.pop() {
            let version = self.entries[index as usize].version;
            make_entity(index, version)
        } else {
            let index = self.entries.len() as u32;
            self.entries.push(EntityEntry {
                // Temporary dummy location; will be overwritten by set_location
                location: EntityLocation { archetype_id: 0, row: 0 },
                version: 0,
            });
            make_entity(index, 0)
        }
    }

    pub(crate) fn destroy(&mut self, id: EntityId) {
        let idx = entity_index(id);
        let version = entity_version(id);

        let entry = &mut self.entries[idx];
        debug_assert!(entry.version == version, "Stale EntityId");

        entry.version = entry.version.wrapping_add(1);
        return self.free.push(idx as u32);
    }

    #[inline(always)]
    pub(crate) fn set_location(&mut self, id: EntityId, loc: EntityLocation) {
        let idx = entity_index(id);
        let version = entity_version(id);

        let entry = &mut self.entries[idx];
        debug_assert!(entry.version == version, "Stale EntityId");
        entry.location = loc;
    }

    #[inline(always)]
    pub(crate) fn get_location(&self, id: EntityId) -> Option<EntityLocation> {
        let idx = entity_index(id);
        let version = entity_version(id);

        let entry = self.entries.get(idx).expect("Wrong wrong wa");
        return (entry.version == version).then_some(entry.location);
    }
}
