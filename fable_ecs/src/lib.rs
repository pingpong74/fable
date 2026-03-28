mod archetypes;
mod bitset;
#[allow(unused)]
mod components;
mod entity;
mod fetch;

pub use components::{COMPONENTS_POOL, Component, ComponentId, ComponentInfo};
pub use entity::EntityId;
pub use fable_ecs_macros::component;
pub use linkme;

use crate::{
    archetypes::ArchetypeSet,
    entity::Entities,
    fetch::{Bundle, Fetch},
};

/// this function collects all the components using link me crate ans assigns them ids
pub fn ecs_init() {
    for (i, component) in COMPONENTS_POOL.iter().enumerate() {
        unsafe {
            *component.id_ptr.0.get() = i;
        };
    }
}

pub struct Ecs {
    entities: Entities,
    archetype_set: ArchetypeSet,
}

impl Ecs {
    pub fn new() -> Ecs {
        return Ecs {
            entities: Entities::new(),
            archetype_set: ArchetypeSet::new(),
        };
    }

    pub fn create_entity<B: Bundle>(&mut self, components: B) -> EntityId {
        let id = self.entities.alloc();
        self.entities.set_location(id, self.archetype_set.create_entity(components, id));
        return id;
    }

    pub fn remove_entity(&mut self, id: EntityId) {
        self.archetype_set.remove_entity(id, &mut self.entities);
    }

    pub fn add_components<B: Bundle>(&mut self, id: EntityId, components: B) {
        self.archetype_set.add_component(id, &mut self.entities, components);
    }

    pub fn remove_components<B: Bundle>(&mut self, id: EntityId) {
        self.archetype_set.remove_components::<B>(id, &mut self.entities);
    }

    pub fn query<'a, T, F>(&mut self, f: F)
    where
        T: Fetch,
        F: for<'b> FnMut(T::Item<'b>),
    {
        self.archetype_set.query_raw(f);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[component]
    struct A;
}
