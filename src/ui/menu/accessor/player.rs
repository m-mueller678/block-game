use std::ops::{Deref, DerefMut};
use item::SlotStorage;
use std::sync::{Arc, Mutex, MutexGuard};
use player::Player;
use super::BoundedAccessor;

pub struct PlayerInventoryAccessor(Arc<Mutex<Player>>);

impl<'a> BoundedAccessor<'a, SlotStorage> for PlayerInventoryAccessor {
    type Ref = PlayerInventoryGuard<'a>;
    type RefMut = PlayerInventoryGuard<'a>;

    fn get(&'a mut self) -> Option<Self::Ref> {
        Some(PlayerInventoryGuard(self.0.lock().unwrap()))
    }

    fn get_mut(&'a mut self) -> Option<Self::RefMut> {
        Some(PlayerInventoryGuard(self.0.lock().unwrap()))
    }
}

impl PlayerInventoryAccessor {
    pub fn new(player: Arc<Mutex<Player>>) -> Self {
        PlayerInventoryAccessor(player)
    }
}

pub struct PlayerInventoryGuard<'a>(MutexGuard<'a, Player>);

impl<'a> Deref for PlayerInventoryGuard<'a> {
    type Target = SlotStorage;

    fn deref(&self) -> &Self::Target {
        self.0.inventory()
    }
}

impl<'a> DerefMut for PlayerInventoryGuard<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.inventory_mut()
    }
}
