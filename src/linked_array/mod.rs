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

    fn get(&self) -> Vec<u32> {
        vec![0]
    }

    fn replace_pair(&mut self, idx: usize, token_id: u32) -> Result<(), MergeError> {
        if idx >= self.vec.len() {
            return Err(MergeError::InvalidIndex)
        }

        let left_ref = self.vec[idx].as_ref();
        
        Ok(())
    }   

    fn prev(&self, idx: usize,) -> Option<&LinkedArrayNode<u32>> {   
        // let node = self.vec.get(idx)?;
        // let prev_idx = node.prev?;
        // self.vec.get(prev_idx)
        //
        None
    }

    fn next(&self, idx: usize) -> Option<&LinkedArrayNode<u32>> {
        // let node = self.vec.get(idx)?;
        // let prev_idx = node.next?;
        // self.vec.get(prev_idx)
        None
    }
}