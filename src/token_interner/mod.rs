use std::collections::HashMap;

#[derive(Debug)]
struct TokenInterner {
    tokens: Vec<Vec<u8>>,
    token_to_id: HashMap<Vec<u8>, u32>,
}

type TokenId = u32;

impl TokenInterner {
    fn intern(&mut self, token: Vec<u8>) -> TokenId {
        if let Some(&id) = self.token_to_id.get(&token) {
            id
        } else {
            let id = self.tokens.len() as TokenId;
            self.token_to_id.insert(token.clone(), id);
            self.tokens.push(token);
            id
        }
    }

    fn get_bytes(&self, id: TokenId) -> &[u8] {
        &self.tokens[id as usize]
    }
}