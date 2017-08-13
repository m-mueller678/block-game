use std::ops::{Deref, DerefMut};

pub use self::player::PlayerInventoryAccessor;

mod player;

pub trait Accessor<T> where
        for<'a> Self: BoundedAccessor<'a, T> {}

impl<T, A> Accessor<T> for A where
        for<'a> Self: BoundedAccessor<'a, T> {}

pub trait BoundedAccessor<'a, T> where
    Self::Ref: Deref<Target=T> + 'a,
    Self::RefMut: DerefMut<Target=T> + 'a
{
    type Ref;
    type RefMut;
    fn get(&'a mut self) -> Option<Self::Ref>;
    fn get_mut(&'a mut self) -> Option<Self::RefMut>;
}
