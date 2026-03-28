use std::{
    alloc::{Layout, dealloc},
    collections::HashMap,
    ops::Deref,
};

use crate::{
    COMPONENTS_POOL, ComponentInfo,
    bitset::*,
    entity::{Entities, EntityId, EntityLocation},
    fetch::{Bundle, Fetch},
};

// TODO:
// FIX ARCHETYPE WRITE FUNC!!!!

type ArchetypeId = usize;

pub(crate) struct ArchetypeSet {
    archetypes: Vec<Archetype>,
    from_components: HashMap<BitSet, ArchetypeId, BuildBitSetHasher>,
}

impl ArchetypeSet {
    pub(crate) fn new() -> ArchetypeSet {
        return ArchetypeSet {
            archetypes: vec![],
            from_components: HashMap::with_hasher(BuildBitSetHasher::default()),
        };
    }

    pub(crate) fn create_entity<B: Bundle>(&mut self, components: B, entity_id: EntityId) -> EntityLocation {
        let bits = B::get_bit_set();

        let arch_id = match self.from_components.get(&bits) {
            Some(&id) => id,
            None => {
                let dst_arch = Archetype::new(bits);
                self.archetypes.push(dst_arch);
                self.archetypes.len() - 1
            }
        };

        let arch = &mut self.archetypes[arch_id];
        let row = arch.reserve();

        arch.write_bundle(components, row);
        arch.entities[row] = entity_id;

        return EntityLocation {
            archetype_id: arch_id as u32,
            row: row as u32,
        };
    }

    pub(crate) fn remove_entity(&mut self, id: EntityId, entities: &mut Entities) {
        let loc = entities.get_location(id).unwrap();
        let arch = &mut self.archetypes[loc.archetype_id as usize];
        let last_id = arch.entities[arch.len - 1];
        unsafe {
            arch.remove(loc.row as usize);
        }

        if last_id != id {
            entities.set_location(last_id, loc);
        }

        entities.destroy(id);
    }

    pub(crate) fn add_component<B: Bundle>(&mut self, entity_id: EntityId, entities: &mut Entities, components: B) {
        let src_location = entities.get_location(entity_id).expect("Wrong id id");

        let dst_archetype_id = {
            let src_archetype = &self.archetypes[src_location.archetype_id as usize];
            let required = src_archetype.component_bits | B::get_bit_set();

            match self.from_components.get(&required) {
                Some(&id) => id,
                None => {
                    let dst_arch = Archetype::new(required);
                    self.archetypes.push(dst_arch);
                    self.archetypes.len() - 1
                }
            }
        };

        let dst_location;
        let mut moved_last_entity = None;

        unsafe {
            let [src_arch, dst_arch] = self.archetypes.get_disjoint_unchecked_mut([src_location.archetype_id as usize, dst_archetype_id as usize]);

            let last_row = src_arch.len - 1;
            let dst_row = dst_arch.reserve();

            Archetype::swap(src_arch, src_location.row as usize, dst_arch, dst_row);

            if src_location.row as usize != last_row {
                let last_id = src_arch.entities[last_row];
                src_arch.entities[src_location.row as usize] = last_id;
                moved_last_entity = Some(last_id);
            }

            dst_arch.entities[dst_row] = entity_id;

            dst_arch.write_bundle(components, dst_row);

            dst_location = EntityLocation {
                archetype_id: dst_archetype_id as u32,
                row: dst_row as u32,
            };
        }

        entities.set_location(entity_id, dst_location);
        if let Some(last_id) = moved_last_entity {
            entities.set_location(last_id, src_location);
        }
    }

    pub(crate) fn remove_components<B: Bundle>(&mut self, entity_id: EntityId, entities: &mut Entities) {
        let src_location = entities.get_location(entity_id).expect("Wrong id id");

        let to_drop;

        let dst_archetype_id = {
            let src_archetype = &self.archetypes[src_location.archetype_id as usize];
            let required = src_archetype.component_bits & !B::get_bit_set();
            to_drop = src_archetype.component_bits & B::get_bit_set();
            match self.from_components.get(&required) {
                Some(&id) => id,
                None => {
                    let dst_arch = Archetype::new(required);
                    self.archetypes.push(dst_arch);
                    self.archetypes.len() - 1
                }
            }
        };

        {
            let arch = &mut self.archetypes[src_location.archetype_id as usize];

            for id in to_drop.iter() {
                let col_index = *arch.id_to_index.get(&id).expect("Hmm");
                let comp_info = arch.component_info[col_index];
                unsafe {
                    let ptr = arch.data[col_index].add(src_location.row as usize * comp_info.layout.size());

                    (comp_info.drop_fn)(ptr);
                }
            }
        }

        let dst_location;
        let mut moved_last_entity = None;

        unsafe {
            let [src_arch, dst_arch] = self.archetypes.get_disjoint_unchecked_mut([src_location.archetype_id as usize, dst_archetype_id as usize]);

            let last_row = src_arch.len - 1;
            let dst_row = dst_arch.reserve();

            Archetype::swap(src_arch, src_location.row as usize, dst_arch, dst_row);

            if src_location.row as usize != last_row {
                let last_id = src_arch.entities[last_row];
                src_arch.entities[src_location.row as usize] = last_id;
                moved_last_entity = Some(last_id);
            }

            dst_arch.entities[dst_row] = entity_id;

            dst_location = EntityLocation {
                archetype_id: dst_archetype_id as u32,
                row: dst_row as u32,
            };
        }

        entities.set_location(entity_id, dst_location);
        if let Some(last_id) = moved_last_entity {
            entities.set_location(last_id, src_location);
        }
    }

    pub(crate) fn query_raw<'a, T, F>(&mut self, mut f: F)
    where
        T: Fetch,
        F: for<'b> FnMut(T::Item<'b>),
    {
        debug_assert!(T::LEN <= 32);
        let required = T::get_bit_set();

        for arch in &self.archetypes {
            if arch.component_bits & required == required {
                let mut target_ptrs = [std::ptr::null_mut::<u8>(); 32];
                let mut stride = [0; 32];

                for (i, comp_id) in T::COMPONENT_IDS.iter().enumerate() {
                    let id = comp_id.get_id();

                    let col_idx = *arch.id_to_index.get(&id).unwrap();
                    target_ptrs[i] = arch.data[col_idx];
                    stride[i] = arch.component_info[col_idx].layout.size();
                }

                for _ in 0..arch.len {
                    f(T::fetch(&target_ptrs));

                    unsafe {
                        target_ptrs.iter_mut().enumerate().for_each(|(i, ptr)| {
                            *ptr = ptr.add(stride[i]);
                        });
                    }
                }
            }
        }
    }
}

const INITIAL_CAPACITY: usize = 25;

struct Archetype {
    id_to_index: OrderedIdMap<usize>,
    component_bits: BitSet,
    component_info: Box<[ComponentInfo]>,

    // data storage
    data: Box<[*mut u8]>,
    entities: Box<[u32]>,
    capacity: usize,
    len: usize,
}

impl Archetype {
    /// Component ids MUST be sorted
    fn new(mask: BitSet) -> Archetype {
        let count = mask.count();
        let mut data = Vec::with_capacity(count);
        let mut component_ids = Vec::with_capacity(count);
        let mut component_info = Vec::with_capacity(count);

        // Iterate through set bits to gather component metadata
        for (i, comp_id) in mask.iter().enumerate() {
            let info = COMPONENTS_POOL[comp_id];

            // Calculate allocation size: capacity * size_of::<T>
            let size = INITIAL_CAPACITY * info.layout.size();

            let data_ptr = if size > 0 {
                unsafe {
                    // Use the layout from the pool to maintain correct alignment
                    std::alloc::alloc(std::alloc::Layout::from_size_align_unchecked(size, info.layout.align()))
                }
            } else {
                // Handle ZSTs (Zero Sized Types) or INITIAL_CAPACITY 0
                std::ptr::NonNull::dangling().as_ptr()
            };

            data.push(data_ptr);
            component_ids.push((comp_id, i));
            component_info.push(info);
        }

        let entities = unsafe {
            let layout = std::alloc::Layout::array::<u32>(INITIAL_CAPACITY).unwrap();
            let ptr = std::alloc::alloc(layout) as *mut u32;
            Box::from_raw(std::slice::from_raw_parts_mut(ptr, INITIAL_CAPACITY))
        };

        Self {
            component_bits: mask,
            id_to_index: OrderedIdMap::new(component_ids.into_iter()),
            component_info: component_info.into_boxed_slice(),
            data: data.into_boxed_slice(),
            entities,
            capacity: INITIAL_CAPACITY,
            len: 0,
        }
    }

    fn reserve(&mut self) -> usize {
        assert!(self.len < self.capacity);

        let vacant = self.len;
        self.len += 1;

        return vacant;
    }

    // need of heavy af fixing
    // bundles > 32 are not allowed
    pub(crate) fn write_bundle<B: Bundle>(&mut self, components: B, row: usize) {
        debug_assert!(B::LEN <= 32);

        let mut target_ptrs = [std::ptr::null_mut::<u8>(); 32];

        for (i, comp_id) in B::COMPONENT_IDS.iter().enumerate() {
            let id = comp_id.get_id();

            let col_idx = *self.id_to_index.get(&id).unwrap();
            let base_ptr = self.data[col_idx];
            let size = self.component_info[col_idx].layout.size();

            target_ptrs[i] = unsafe { base_ptr.add(row * size) };
        }

        components.write(&target_ptrs);
    }

    /// This functions just puts the entity at the last index at the given index.
    /// The element at the given index is dropped, and the last , element is moved to the location of the old element
    /// Safety: Called must manage the change of Id of the old id
    unsafe fn remove(&mut self, row: usize) {
        let last = self.len - 1;

        for (col, comp_info) in self.data.iter_mut().zip(&self.component_info) {
            let size = comp_info.layout.size();
            unsafe {
                let row_ptr = col.add(row * size);
                let last_ptr = col.add(last * size);

                (comp_info.drop_fn)(row_ptr);

                if row != last {
                    std::ptr::copy_nonoverlapping(last_ptr, row_ptr, size);
                }
            }
        }

        if row != last {
            self.entities[row] = self.entities[last];
        }

        self.len = last;
    }

    /// This functions moves the entity at src_row to dst_row and the last entity in src archetype to src_row.
    /// changes the len (basically says the last entity id is invalid)
    /// Safety: Called must manage the change of Id of the old id
    unsafe fn swap(src: &mut Archetype, src_row: usize, dst: &mut Archetype, dst_row: usize) {
        // This part copies data from src to dst archetype
        let mut si = 0;
        let mut di = 0;

        while si < src.data.len() && di < dst.data.len() {
            let src_component_id = src.id_to_index[si].0;
            let dst_component_id = dst.id_to_index[di].0;

            if src_component_id > dst_component_id {
                di += 1;
            } else if dst_component_id > src_component_id {
                si += 1;
            } else {
                let size = src.component_info[si].layout.size();

                unsafe {
                    let src_row_ptr = src.data[si].add(src_row * size);
                    let dst_row_ptr = dst.data[di].add(dst_row * size);

                    debug_assert!(size == dst.component_info[di].layout.size());

                    std::ptr::copy_nonoverlapping(src_row_ptr, dst_row_ptr, size);
                }

                si += 1;
                di += 1;
            }
        }

        // Now copy the data at the end of the src archetype to the hole created at src_row

        let last = src.len - 1;

        if src_row != last {
            for (col, comp_info) in src.data.iter_mut().zip(&src.component_info) {
                let size = comp_info.layout.size();
                unsafe {
                    let row_ptr = col.add(src_row * size);
                    let last_ptr = col.add(last * size);

                    std::ptr::copy_nonoverlapping(last_ptr, row_ptr, size);
                }
            }
        }

        src.len = last;
    }
}

impl Drop for Archetype {
    fn drop(&mut self) {
        unsafe {
            self.data
                .iter()
                .zip(&self.component_info)
                .for_each(|(&col, comp_info)| dealloc(col, Layout::from_size_align_unchecked(comp_info.layout.size() * self.capacity, comp_info.layout.align())));
        }
    }
}

/// For mapping the type id to the archetype local index
struct OrderedIdMap<V>(Box<[(usize, V)]>);

impl<V> OrderedIdMap<V> {
    fn new(iter: impl Iterator<Item = (usize, V)>) -> Self {
        let mut vals = iter.collect::<Box<[_]>>();
        vals.sort_unstable_by_key(|(id, _)| *id);
        Self(vals)
    }

    fn search(&self, id: &usize) -> Option<usize> {
        self.0.binary_search_by_key(id, |(id, _)| *id).ok()
    }

    fn get(&self, id: &usize) -> Option<&V> {
        self.search(id).map(move |idx| &self.0[idx].1)
    }
}

impl<V> Deref for OrderedIdMap<V> {
    type Target = [(usize, V)];

    fn deref(&self) -> &Self::Target {
        return &self.0;
    }
}
