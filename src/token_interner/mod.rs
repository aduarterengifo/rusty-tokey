use std::collections::HashMap;

#[derive(Debug)]
pub struct TokenInterner {
    pub tokens: Vec<Vec<u8>>,
    pub token_to_id: HashMap<Vec<u8>, u32>,
}

pub type TokenId = u32;

impl TokenInterner {
    pub fn intern(&mut self, token: Vec<u8>) -> TokenId {
        if let Some(&id) = self.token_to_id.get(&token) {
            id
        } else {
            let id = self.tokens.len() as TokenId;
            self.token_to_id.insert(token.clone(), id);
            self.tokens.push(token);
            id
        }
    }

    pub fn get_bytes(&self, id: TokenId) -> &[u8] {
        &self.tokens[id as usize]
    }
}