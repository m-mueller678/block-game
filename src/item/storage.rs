use super::*;
use std::ops::Index;
use std::convert::From;
use std::sync::{Mutex, MutexGuard, TryLockError};
use std::thread;

type Inner = Option<Box<ItemStack>>;

pub struct Slot(Mutex<Inner>);

pub struct SlotLock<'a>(MutexGuard<'a, Inner>);

impl<'a> SlotLock<'a> {
    pub fn stack(&mut self) -> Option<&mut ItemStack> {
        match *self.0 {
            Some(ref mut b) => Some(&mut **b),
            None => None,
        }
    }
}

fn double_lock<'a, 'b>(s1: &'a Slot, s2: &'b Slot) -> (SlotLock<'a>, SlotLock<'b>) {
    assert_ne!(s1 as *const Slot, s2 as *const Slot);
    loop {
        match s1.0.try_lock() {
            Ok(l1) => match s2.0.try_lock() {
                Ok(l2) => return (SlotLock(l1), SlotLock(l2)),
                Err(TryLockError::WouldBlock) => {}
                Err(e) => Err(e).unwrap(),
            },
            Err(TryLockError::WouldBlock) => {}
            Err(e) => Err(e).unwrap(),
        }
        thread::yield_now();
    };
}

impl Slot {
    pub fn from_itemstack(stack: Box<ItemStack>) -> Self {
        Slot(Mutex::new(Some(stack)))
    }
    pub fn new() -> Self {
        Slot(Mutex::new(None))
    }
    pub fn move_all_from(&self, game_data: &GameData, from_slot: &Slot) {
        let (SlotLock(mut to_lock), SlotLock(mut from_lock)) = double_lock(self, from_slot);
        if let Some(from) = from_lock.take() {
            match &mut *to_lock {
                &mut Some(ref mut to) => {
                    *from_lock = to.stack_from(game_data, from, 1);
                }
                to => {
                    *to = Some(from)
                }
            }
        }
    }
    pub fn move_some_from(&self, game_data: &GameData, from_slot: &Slot, max: u32) {
        if max == 0 { return; }
        let (SlotLock(mut to_lock), SlotLock(mut from_lock)) = double_lock(self, from_slot);
        if let &mut Some(ref mut to) = &mut *to_lock {
            if let Some(mut from) = from_lock.take() {
                *from_lock = to.stack_some_from(game_data, from, 1, max);
            }
            return;
        }
        if let Some(mut from) = from_lock.take() {
            let count = from.count();
            if max >= count {
                *to_lock = Some(from);
            } else {
                *to_lock = Some(from.take(game_data, max));
                *from_lock = Some(from);
            }
        }
    }
    pub fn lock(&self) -> SlotLock {
        SlotLock(self.0.lock().unwrap())
    }
    pub fn is_empty(&self) -> bool {
        self.0.lock().unwrap().is_none()
    }
    pub fn count(&self) -> u32 {
        self.0.lock().unwrap().as_ref().map_or(0, |x| x.count())
    }
}

impl From<Box<ItemStack>> for Slot {
    fn from(stack: Box<ItemStack>) -> Self {
        Slot(Mutex::new(Some(stack)))
    }
}

pub struct SlotStorage {
    slots: Vec<Slot>
}

impl SlotStorage {
    pub fn new(size: usize) -> Self {
        SlotStorage {
            slots: (0..size).map(|_| Slot::new()).collect()
        }
    }
    pub fn len(&self) -> usize {
        self.slots.len()
    }
}

impl Index<usize> for SlotStorage {
    type Output = Slot;

    fn index(&self, index: usize) -> &Self::Output {
        &self.slots[index]
    }
}
