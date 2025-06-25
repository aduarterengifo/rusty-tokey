use std::collections::HashMap;
use std::hash::Hash;

use crate::token_interner::TokenId;

// TODO: make private
pub struct Interner<T: Clone + Hash + Eq> {
    pub items: Vec<T>,
    pub item_to_id: HashMap<T, u32>,
}

impl<T: Clone + Hash + Eq> Interner<T> {
      pub fn intern(&mut self, item: T) -> u32 { 
        if let Some(&id) = self.item_to_id.get(&item) {
            id
        } else {
            let id = self.items.len() as TokenId;
            self.item_to_id.insert(item.clone(), id);
            self.items.push(item);
            id
        }
      }
      pub fn get(&self, id: u32) -> &T { 
            &self.items[id as usize]
       }
  }

pub type VocabInterner = Interner<Vec<u8>>;

pub type TokenIdsId = u32;
pub type TokSeqInterner = Interner<Vec<TokenId>>;