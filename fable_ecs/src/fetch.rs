use crate::{Component, ComponentId, bitset::BitSet};

// the fetch core trait ensures that we cannot do smt like (C1, (C2, C3), C4....)
pub trait FetchCore {
    type Item<'a>;
    const COMPONENT_ID: &'static ComponentId;

    fn fetch<'a>(data: *mut u8) -> Self::Item<'a>;
}

impl<T: Component> FetchCore for &T {
    type Item<'a> = &'a T;
    const COMPONENT_ID: &'static ComponentId = &T::ID;

    fn fetch<'a>(data: *mut u8) -> Self::Item<'a> {
        return unsafe { &*(data as *const T) };
    }
}

impl<T: Component> FetchCore for &mut T {
    type Item<'a> = &'a mut T;
    const COMPONENT_ID: &'static ComponentId = &T::ID;

    fn fetch<'a>(data: *mut u8) -> Self::Item<'a> {
        return unsafe { &mut *(data as *mut T) };
    }
}

// actual fetch trait
pub trait Fetch {
    type Item<'a>;
    const LEN: usize;
    const COMPONENT_IDS: &'static [&'static ComponentId];

    fn get_bit_set() -> BitSet {
        return BitSet::from_component_ids(Self::COMPONENT_IDS);
    }

    fn fetch<'a>(data: &[*mut u8]) -> Self::Item<'a>;
}

impl<A: FetchCore> Fetch for A {
    type Item<'a> = A::Item<'a>;
    const LEN: usize = 1;
    const COMPONENT_IDS: &'static [&'static ComponentId] = &[A::COMPONENT_ID];

    fn fetch<'a>(data: &[*mut u8]) -> Self::Item<'a> {
        return A::fetch(data[0]);
    }
}

impl<A: FetchCore, B: FetchCore> Fetch for (A, B) {
    type Item<'a> = (A::Item<'a>, B::Item<'a>);
    const LEN: usize = 2;
    const COMPONENT_IDS: &'static [&'static ComponentId] = &[A::COMPONENT_ID, B::COMPONENT_ID];

    fn fetch<'a>(data: &[*mut u8]) -> Self::Item<'a> {
        return (A::fetch(data[0]), B::fetch(data[1]));
    }
}

// the bundle trait is used for adding an entity, or adding
pub trait Bundle: Sized {
    const LEN: usize;
    const COMPONENT_IDS: &'static [&'static ComponentId];

    fn get_bit_set() -> BitSet {
        return BitSet::from_component_ids(Self::COMPONENT_IDS);
    }
    // TODO: impl ts
    fn write<'a>(self, data: &[*mut u8]);
}

impl<T: Component> Bundle for T {
    const LEN: usize = 1;
    const COMPONENT_IDS: &'static [&'static ComponentId] = &[T::ID];

    fn write<'a>(self, data: &[*mut u8]) {
        unsafe {
            (data[0] as *mut T).write(self);
        }
    }
}

impl<A: Component, B: Component> Bundle for (A, B) {
    const LEN: usize = 2;
    const COMPONENT_IDS: &'static [&'static ComponentId] = &[A::ID, B::ID];

    fn write<'a>(self, data: &[*mut u8]) {
        unsafe {
            (data[0] as *mut A).write(self.0);
            (data[1] as *mut B).write(self.1);
        }
    }
}
