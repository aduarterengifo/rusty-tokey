use crate::TokenInterner;

#[derive(Debug, Clone)]
struct LinkedArrayNode<T> {
    elem: T,
    idx: usize,
    next: Option<usize>,
    prev: Option<usize>
}


struct LinkedArray<T> {
    vec: Vec<Option<LinkedArrayNode<T>>>,
    interner: TokenInterner
}

pub enum MergeError {
    InvalidIndex, 
    LastElement, 
    ElementNotFound, 
    NextElementNotFound
}

impl LinkedArray<u32> {
    fn new(vec: Vec<u32>, interner: TokenInterner) -> LinkedArray<u32> {
        // create vec that will hold linked-array-nodes. 
        let mut new_vec: Vec<Option<LinkedArrayNode<u32>>> = Vec::with_capacity(vec.len());

        // loop over vec arg, and push linked-array-nodes 
        for (idx, elem) in vec.iter().enumerate() {
            let cur = LinkedArrayNode {
              elem: *elem,
              idx,
              prev: if idx > 0 { Some(idx - 1) } else { None },
              next: if idx + 1 < vec.len() { Some(idx + 1) } else { None }
            };
            new_vec.push(Some(cur));
        }

        LinkedArray {
            vec: new_vec,
            interner
        }
    }

    // morally, I can just act conditionally, and do nothing otherwise, 
    // but then what if my callee, wants to do something conditionally on my actions?
    // in that case I am truly and fully fucked.
    fn replace_pair(&mut self, idx: usize, token_id: u32) -> Result<(), MergeError> {
        if idx >= self.vec.len() {
            return Err(MergeError::InvalidIndex)
        }

        // get data without holding borrows
        let (fst_elem, snd_idx, snd_elem, snd_next_idx_opt) = {
            let fst = self.vec[idx].as_ref().ok_or(MergeError::ElementNotFound)?;
            let snd_idx = fst.next.ok_or(MergeError::NextElementNotFound)?;
            let snd = self.vec[snd_idx].as_ref().ok_or(MergeError::NextElementNotFound)?;
            
            (fst.elem, snd_idx, snd.elem, snd.next)
        };

        // get new token id 
        let fst_bytes = self.interner.get_bytes(fst_elem);
        let snd_bytes = self.interner.get_bytes(snd_elem);

        let mut merged_bytes = Vec::with_capacity(fst_bytes.len() + snd_bytes.len());

        merged_bytes.extend_from_slice(fst_bytes);
        merged_bytes.extend_from_slice(snd_bytes);

        let new_token_id = self.interner.intern(merged_bytes);

        // mutate first element.
        if let Some(fst) = self.vec[idx].as_mut() {
            fst.elem = new_token_id;
            fst.next = snd_next_idx_opt; // Skip over the second element
        }

        // if snd_next exists, update its prev pointer.
        if let Some(snd_next_idx) = snd_next_idx_opt {
            if let Some(snd_next_node) = self.vec[snd_next_idx].as_mut() {
                snd_next_node.prev = Some(idx);
            }
        }

        // remove snd.
        self.vec[snd_idx] = None;

        Ok(())
    }   

    fn prev(&self, idx: usize) -> Option<&LinkedArrayNode<u32>> {   
        // Monadic bind style - chain the operations
        self.vec.get(idx)
            .and_then(|node| node.as_ref())
            .and_then(|node| node.prev)
            .and_then(|prev_idx| self.vec.get(prev_idx))
            .and_then(|node| node.as_ref())
    }

    fn next(&self, idx: usize) -> Option<&LinkedArrayNode<u32>> {
        // Monadic bind style - chain the operations
        self.vec.get(idx)
            .and_then(|node| node.as_ref())
            .and_then(|node| node.next)
            .and_then(|next_idx| self.vec.get(next_idx))
            .and_then(|node| node.as_ref())
    }
}