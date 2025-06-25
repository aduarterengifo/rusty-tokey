use crate::{token_interner::TokenId, VocabInterner};

#[derive(Debug, Clone)]
pub struct LinkedArrayNode<T> {
    pub elem: T,
    pub idx: usize,
    next: Option<usize>,
    prev: Option<usize>
}


pub struct LinkedArray<T> {
    vec: Vec<Option<LinkedArrayNode<T>>>,
}

#[derive(Debug)]
pub enum MergeError {
    InvalidIndex, 
    LastElement, 
    ElementNotFound, 
    NextElementNotFound
}

impl LinkedArray<u32> {
    pub fn new(vec: Vec<u32>) -> LinkedArray<u32> {
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
        }
    }

    // morally, I can just act conditionally, and do nothing otherwise, 
    // but then what if my callee, wants to do something conditionally on my actions?
    // in that case I am truly and fully fucked.
    pub fn replace_pair(&mut self, idx: usize, interner: &mut VocabInterner) -> Result<Vec<u32>, MergeError> {
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
        let fst_bytes = interner.get(fst_elem);
        let snd_bytes = interner.get(snd_elem);

        let mut merged_bytes = Vec::with_capacity(fst_bytes.len() + snd_bytes.len());

        merged_bytes.extend_from_slice(fst_bytes);
        merged_bytes.extend_from_slice(snd_bytes);

        let new_token_id = interner.intern(merged_bytes);

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

        let res: Vec<TokenId> = self.vec.iter().filter_map(|x| x.clone().map(|y| y.elem)).collect();

        Ok(res)
    } 

    pub fn get(&self, idx: usize) -> Option<&LinkedArrayNode<u32>> {   
        // Monadic bind style - chain the operations
        self.vec.get(idx)
            .and_then(|node| node.as_ref())
    }  

    pub fn prev(&self, idx: usize) -> Option<&LinkedArrayNode<u32>> {   
        // Monadic bind style - chain the operations
        self.vec.get(idx)
            .and_then(|node| node.as_ref())
            .and_then(|node| node.prev)
            .and_then(|prev_idx| self.vec.get(prev_idx))
            .and_then(|node| node.as_ref())
    }

    pub fn next(&self, idx: usize) -> Option<&LinkedArrayNode<u32>> {
        // Monadic bind style - chain the operations
        self.vec.get(idx)
            .and_then(|node| node.as_ref())
            .and_then(|node| node.next)
            .and_then(|next_idx| self.vec.get(next_idx))
            .and_then(|node| node.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interner::{VocabInterner, Interner};
    use std::collections::HashMap;

    fn create_test_interner() -> VocabInterner {
        VocabInterner {
            items: vec![b"a".to_vec(), b"b".to_vec(), b"c".to_vec(), b"d".to_vec()],
            item_to_id: [
                (b"a".to_vec(), 0),
                (b"b".to_vec(), 1), 
                (b"c".to_vec(), 2),
                (b"d".to_vec(), 3),
            ].into_iter().collect(),
        }
    }

    #[test]
    fn test_linked_array_new() {
        let arr = LinkedArray::new(vec![10, 20, 30]);
        
        assert_eq!(arr.vec.len(), 3);
        
        // Check first element
        let first = arr.get(0).unwrap();
        assert_eq!(first.elem, 10);
        assert_eq!(first.idx, 0);
        assert_eq!(first.prev, None);
        assert_eq!(first.next, Some(1));
        
        // Check middle element
        let middle = arr.get(1).unwrap();
        assert_eq!(middle.elem, 20);
        assert_eq!(middle.idx, 1);
        assert_eq!(middle.prev, Some(0));
        assert_eq!(middle.next, Some(2));
        
        // Check last element
        let last = arr.get(2).unwrap();
        assert_eq!(last.elem, 30);
        assert_eq!(last.idx, 2);
        assert_eq!(last.prev, Some(1));
        assert_eq!(last.next, None);
    }

    #[test]
    fn test_linked_array_single_element() {
        let arr = LinkedArray::new(vec![42]);
        
        let node = arr.get(0).unwrap();
        assert_eq!(node.elem, 42);
        assert_eq!(node.idx, 0);
        assert_eq!(node.prev, None);
        assert_eq!(node.next, None);
    }

    #[test]
    fn test_linked_array_empty() {
        let arr: LinkedArray<u32> = LinkedArray::new(vec![]);
        assert_eq!(arr.vec.len(), 0);
        assert!(arr.get(0).is_none());
    }

    #[test]
    fn test_get_invalid_index() {
        let arr = LinkedArray::new(vec![1, 2, 3]);
        assert!(arr.get(3).is_none());
        assert!(arr.get(100).is_none());
    }

    #[test]
    fn test_prev_navigation() {
        let arr = LinkedArray::new(vec![10, 20, 30, 40]);
        
        // Test valid prev navigation
        let prev = arr.prev(2).unwrap();
        assert_eq!(prev.elem, 20);
        assert_eq!(prev.idx, 1);
        
        // Test first element has no prev
        assert!(arr.prev(0).is_none());
        
        // Test invalid index
        assert!(arr.prev(10).is_none());
    }

    #[test]
    fn test_next_navigation() {
        let arr = LinkedArray::new(vec![10, 20, 30, 40]);
        
        // Test valid next navigation
        let next = arr.next(1).unwrap();
        assert_eq!(next.elem, 30);
        assert_eq!(next.idx, 2);
        
        // Test last element has no next
        assert!(arr.next(3).is_none());
        
        // Test invalid index
        assert!(arr.next(10).is_none());
    }

    #[test]
    fn test_replace_pair_basic() {
        let mut arr = LinkedArray::new(vec![0, 1, 2, 3]);
        let mut interner = create_test_interner();
        
        // Replace pair at position 1 (merge elements 1 and 2)
        let result = arr.replace_pair(1, &mut interner).unwrap();
        
        // Should have 3 elements now (0, merged(1,2), 3)
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], 0); // 'a'
        assert_eq!(result[2], 3); // 'd'
        
        // Check the structure after merge
        let first = arr.get(0).unwrap();
        assert_eq!(first.elem, 0);
        assert_eq!(first.next, Some(1));
        
        let merged = arr.get(1).unwrap();
        assert_eq!(merged.next, Some(3)); // Should skip over removed element
        
        let last = arr.get(3).unwrap();
        assert_eq!(last.elem, 3);
        assert_eq!(last.prev, Some(1)); // Should point back to merged element
        
        // Element at index 2 should be None (removed)
        assert!(arr.get(2).is_none());
    }

    #[test]
    fn test_replace_pair_first_elements() {
        let mut arr = LinkedArray::new(vec![0, 1, 2]);
        let mut interner = create_test_interner();
        
        let result = arr.replace_pair(0, &mut interner).unwrap();
        
        assert_eq!(result.len(), 2);
        assert_eq!(result[1], 2); // 'c' should be preserved
        
        // Check linkage
        let merged = arr.get(0).unwrap();
        assert_eq!(merged.next, Some(2));
        assert_eq!(merged.prev, None);
        
        let last = arr.get(2).unwrap();
        assert_eq!(last.prev, Some(0));
    }

    #[test]
    fn test_replace_pair_last_elements() {
        let mut arr = LinkedArray::new(vec![0, 1, 2]);
        let mut interner = create_test_interner();
        
        let result = arr.replace_pair(1, &mut interner).unwrap();
        
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], 0); // 'a' should be preserved
        
        // Check linkage
        let first = arr.get(0).unwrap();
        assert_eq!(first.next, Some(1));
        
        let merged = arr.get(1).unwrap();
        assert_eq!(merged.prev, Some(0));
        assert_eq!(merged.next, None);
    }

    #[test]
    fn test_replace_pair_index_preservation() {
        let mut arr = LinkedArray::new(vec![0, 1, 2, 3, 0, 1]);
        let mut interner = create_test_interner();
        
        // Merge at position 1 (elements 1,2)
        let result1 = arr.replace_pair(1, &mut interner).unwrap();
        assert_eq!(result1.len(), 5);
        
        // Check that indices are preserved correctly
        let elem_0 = arr.get(0).unwrap();
        assert_eq!(elem_0.idx, 0);
        assert_eq!(elem_0.next, Some(1));
        
        let merged_elem = arr.get(1).unwrap();
        assert_eq!(merged_elem.idx, 1); // Original index preserved
        assert_eq!(merged_elem.next, Some(3)); // Skips removed element at idx 2
        
        let elem_3 = arr.get(3).unwrap();
        assert_eq!(elem_3.idx, 3);
        assert_eq!(elem_3.prev, Some(1)); // Points to merged element
        
        // Verify element at index 2 was removed
        assert!(arr.get(2).is_none());
        
        // Now merge at position 4 (elements at indices 4,5)
        let result2 = arr.replace_pair(4, &mut interner).unwrap();
        assert_eq!(result2.len(), 4);
        
        // Check final structure maintains correct indices
        let final_elem = arr.get(4).unwrap();
        assert_eq!(final_elem.idx, 4); // Original index preserved
        assert!(final_elem.next.is_none()); // Should be last element
        
        // Element at index 5 should be removed
        assert!(arr.get(5).is_none());
    }

    #[test]
    fn test_replace_pair_errors() {
        let mut arr = LinkedArray::new(vec![0, 1]);
        let mut interner = create_test_interner();
        
        // Test invalid index
        match arr.replace_pair(5, &mut interner) {
            Err(MergeError::InvalidIndex) => {},
            _ => panic!("Expected InvalidIndex error"),
        }
        
        // Test last element (no next to merge with)
        match arr.replace_pair(1, &mut interner) {
            Err(MergeError::NextElementNotFound) => {},
            _ => panic!("Expected NextElementNotFound error"),
        }
    }

    #[test]
    fn test_multiple_merges_preserve_indices() {
        let mut arr = LinkedArray::new(vec![0, 1, 2, 3, 0, 1, 2]);
        let mut interner = create_test_interner();
        
        // First merge at position 0
        let result1 = arr.replace_pair(0, &mut interner).unwrap();
        assert_eq!(result1.len(), 6);
        
        // Verify structure after first merge
        let merged_0 = arr.get(0).unwrap();
        assert_eq!(merged_0.idx, 0);
        assert_eq!(merged_0.next, Some(2));
        
        let elem_2 = arr.get(2).unwrap();
        assert_eq!(elem_2.prev, Some(0));
        
        // Second merge at position 4 
        let result2 = arr.replace_pair(4, &mut interner).unwrap();
        assert_eq!(result2.len(), 5);
        
        // Verify indices are still correct after multiple merges
        let merged_4 = arr.get(4).unwrap();
        assert_eq!(merged_4.idx, 4);
        assert_eq!(merged_4.next, Some(6));
        
        let elem_6 = arr.get(6).unwrap();
        assert_eq!(elem_6.prev, Some(4));
        assert_eq!(elem_6.idx, 6);
    }

    #[test]
    fn test_chain_navigation_after_merges() {
        let mut arr = LinkedArray::new(vec![0, 1, 2, 3, 0]);
        let mut interner = create_test_interner();
        
        // Merge middle elements
        arr.replace_pair(1, &mut interner).unwrap();
        
        // Test forward navigation
        let start = arr.get(0).unwrap();
        let next1 = arr.next(start.idx).unwrap();
        let next2 = arr.next(next1.idx).unwrap();
        let next3 = arr.next(next2.idx);
        
        assert_eq!(next1.idx, 1); // Merged element
        assert_eq!(next2.idx, 3); // Skipped over removed element 2
        assert_eq!(next3.unwrap().idx, 4); // Last element
        
        // Test backward navigation
        let end = arr.get(4).unwrap();
        let prev1 = arr.prev(end.idx).unwrap();
        let prev2 = arr.prev(prev1.idx).unwrap();
        let prev3 = arr.prev(prev2.idx).unwrap();
        let prev4 = arr.prev(prev3.idx);
        
        assert_eq!(prev1.idx, 3);
        assert_eq!(prev2.idx, 1); // Merged element
        assert_eq!(prev3.idx, 0);
        assert!(prev4.is_none()); // Beginning of list
    }
}