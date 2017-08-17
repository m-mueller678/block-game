use super::*;
use std::ops::{Index, IndexMut, Deref, DerefMut};
use std::convert::From;

pub struct Slot(Option<Box<ItemStack>>);

impl Slot {
    pub fn new()->Self{
        Slot(None)
    }
    pub fn move_from(&mut self, game_data: &GameData, from_stack: &mut Option<Box<ItemStack>>) {
        if let Some(from) = from_stack.take() {
            if let Some(ref mut to) = self.0 {
                *from_stack = to.stack_from(game_data, from);
            } else {
                self.0 = Some(from);
            }
        }
    }
    pub fn stack(&self)->&Option<Box<ItemStack>>{
        &self.0
    }
}

impl From<Box<ItemStack>> for Slot{
    fn from(stack: Box<ItemStack>) -> Self {
        Slot(Some(stack))
    }
}

impl Deref for Slot {
    type Target = Option<Box<ItemStack>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Slot {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub struct SlotStorage {
    slots: Vec<Slot>
}

impl SlotStorage {
    pub fn new(size:usize)->Self{
        SlotStorage{
            slots:(0..size).map(|_|Slot::new()).collect()
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

impl IndexMut<usize> for SlotStorage {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.slots[index]
    }
}