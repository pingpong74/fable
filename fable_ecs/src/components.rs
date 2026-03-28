use std::{cell::UnsafeCell, usize};

use fable_ecs_macros::component;
use linkme::distributed_slice;

pub struct ComponentId(pub(crate) UnsafeCell<usize>);

impl ComponentId {
    pub const fn invalid() -> ComponentId {
        return ComponentId(UnsafeCell::new(usize::MAX));
    }

    pub const fn get_id(&self) -> usize {
        return unsafe { *self.0.get() };
    }
}

unsafe impl Send for ComponentId {}
unsafe impl Sync for ComponentId {}

pub trait Component: Send + Sync + 'static {
    const INFO: &'static ComponentInfo;
    const ID: &'static ComponentId;

    fn get_id() -> usize {
        return Self::ID.get_id();
    }
}

#[derive(Clone, Copy)]
pub struct ComponentInfo {
    pub(crate) layout: std::alloc::Layout,
    pub(crate) drop_fn: unsafe fn(*mut u8),
    pub(crate) id_ptr: &'static ComponentId,
}

impl ComponentInfo {
    pub const fn of<T: 'static>(id_ptr: &'static ComponentId) -> Self {
        unsafe fn drop_ptr<T>(x: *mut u8) {
            unsafe {
                x.cast::<T>().drop_in_place();
            };
        }

        Self {
            layout: std::alloc::Layout::new::<T>(),
            drop_fn: drop_ptr::<T>,
            id_ptr: id_ptr,
        }
    }
}

#[distributed_slice]
pub static COMPONENTS_POOL: [ComponentInfo];
