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

    fn replace_pair(&mut self, idx: usize, token_id: u32) -> () {
        // Extract all the indices and values we need first
        // let (curr_elem, next_elem, next_next_idx) = {
        //     let curr = self.vec.get(idx)?;
        //     if let Some(curr) = curr {
        //         let next_idx = curr.next?;
        //         let next = self.vec.get(next_idx)?;
        //         if let Some(next) = next {
        //             let next_next_idx = next.next;
        //             return (curr.elem, next.elem, next_next_idx)
        //         };
        //     }
        // };

        // what do i actually need to write?

        let (curr_elem, new_token_id,next_next_idx) = match self.vec.get(idx) {
           Some(Some(curr)) => {
            match curr.next.and_then(|x| self.vec.get(x)) {
                Some(Some(next)) => {
                    // byte representation 
                    // string interning 
                    let curr_elem_bytes = self.interner.get_bytes(curr.elem);
                    let next_elem_bytes = self.interner.get_bytes(next.elem);
                    let merged_bytes = [curr_elem_bytes, next_elem_bytes].concat();
                    let new_token_id = self.interner.intern(merged_bytes);

                    // update curr elem.
                    //curr.elem = new_token_id;

                    let next_next_idx = next.next;
                    match next.next.and_then(|x| self.vec.get(x)) {
                        Some(Some(next_next)) => {
                            // set curr's next to next-next.
                            // curr.next = next_next_idx;
                            // next_next.prev = Some(idx);
                            (Some(curr.elem), Some(new_token_id),next_next_idx)
                        },
                        _ => (Some(curr.elem), Some(new_token_id),None)
                    }
                },
                _ => (None,None,None)
            }
           },
           _ => (None,None,None)
        };

        // if curr exists.
        if let Some(Some(curr_mut)) = self.vec.get_mut(idx) {

            // curr.elem <- new_token_id
            curr_mut.elem = new_token_id.unwrap();

            // if next_next exists
            // if let Some(Some(next_next_mut)) = self.vec.get_mut(next_next_idx.unwrap()) {
            //     // curr.next <- next_next_idx
            //     curr_mut.next = next_next_idx;  
            // } else {
            //     curr_mut.next = None;
            // }


            self.vec[next_next_idx.unwrap()].unwrap().prev = Some(idx);
 
        }



        // Get the byte representations for merging
        // let curr_elem_bytes = self.interner.get_bytes(curr_elem);
        // let next_elem_bytes = self.interner.get_bytes(next_elem);
        // let merged_bytes = [curr_elem_bytes, next_elem_bytes].concat();
        // let new_token_id = self.interner.intern(merged_bytes);

        // // mutations

        // // Update current node with merged token
        // self.vec[idx].elem = new_token_id;
        // self.vec[idx].next = next_next_idx;

        // // Update next-next node's prev pointer (if it exists)
        // if let Some(next_next_idx) = next_next_idx {
        //     if next_next_idx < self.vec.len() {
        //         self.vec[next_next_idx].prev = Some(idx);
        //     }
        // }

        // Mark the next node as deleted or handle it as needed
        // You might want to mark it as inactive rather than actually removing it
        
        ();
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