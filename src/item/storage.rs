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

impl Slot {
    pub fn new() -> Self {
        Slot(Mutex::new(None))
    }
    pub fn move_from(&self, game_data: &GameData, from_slot: &Slot) {
        let (mut to_lock, mut from_lock) = loop {
            if let Some((l1, l2)) = match self.0.try_lock() {
                Ok(l1) => match from_slot.0.try_lock() {
                    Ok(l2) => Some((l1, l2)),
                    Err(TryLockError::WouldBlock) => None,
                    Err(e) => Err(e).unwrap(),
                },
                Err(TryLockError::WouldBlock) => None,
                Err(e) => Err(e).unwrap(),
            } {
                break (l1, l2);
            } else {
                thread::yield_now();
            }
        };
        if let Some(from) = from_lock.take() {
            match &mut *to_lock {
                &mut Some(ref mut to) => {
                    *from_lock = to.stack_from(game_data, from);
                }
                to => {
                    *to = Some(from)
                }
            }
        }
    }
    pub fn lock(&self) -> SlotLock {
        SlotLock(self.0.lock().unwrap())
    }
    pub fn is_empty(&self) -> bool {
        self.0.lock().unwrap().is_none()
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
