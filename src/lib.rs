use pyo3::prelude::*;
use std::collections::{HashMap, HashSet};

/// Formats the sum of two numbers as string.
#[pyfunction]
fn sum_as_string(a: usize, b: usize) -> PyResult<String> {
    Ok((a + b).to_string())
}

// borrows: 
//   tok_to_count: hashmap
fn get_pairs(
    tok_to_count: &HashMap<Vec<Vec<u8>>, usize>,
) -> (
    HashMap<(Vec<u8>, Vec<u8>), usize>,
    HashMap<(Vec<u8>, Vec<u8>), HashSet<Vec<u8>>>,
) {
    let mut pair_to_count: HashMap<(Vec<u8>, Vec<u8>), usize> = HashMap::new();
    let mut pair_to_toks: HashMap<(Vec<u8>, Vec<u8>), HashSet<Vec<u8>>> = HashMap::new();

    for (tok, count) in tok_to_count {
        // not big enough for a pair.
        if tok.len() < 2 {
            continue;
        }
        for i in 0..(tok.len() - 1) {
            let pair = (tok[i], tok[i+1]);

            match pair_to_count.entry(pair.clone()) {
                std::collections::hash_map::Entry::Occupied(mut e) => {
                    *e.get_mut() += count; // Increment by counter if exists
                }
                std::collections::hash_map::Entry::Vacant(e) => {
                    e.insert(*count); // Insert counter if doesn't exist
                }
            }

            pair_to_toks
                .entry(pair.clone())
                .or_insert(HashSet::new())
                .insert(tok);
        }
    }

    (pair_to_count, pair_to_toks)
}

/// we take a map from vector of bytes and a max vocab size.
/// and we return?
#[pyfunction]
fn merge(tok_to_count: HashMap<Vec<Vec<u8>>, usize>, max: usize) -> PyResult<Vec<(Vec<u8>, Vec<u8>)>> {
    let mut max_pairs = vec![(vec![0; 0], vec![0; 0]); max];
    let (mut pair_to_count, mut pair_to_toks) = get_pairs(&tok_to_count);
    let max_heap = 

    while max_pairs.len() < max {
        let max_pair_opt = max_pairs.iter()
        .max_by_key(|(_, count)| count)
        .map(|pair| pair.clone());

        // if there is a max_pair 
        if let Some(max_pair) = max_pair_opt {
            max_pairs.push(max_pair.clone());
            // ---------------- EFFICIENTLY UPDATE ----------------
            let max_pair_to_toks = &pair_to_toks[&max_pair];
            // for every tok that contains max_pair
            for tok in max_pair_to_toks.clone() {
                let tok_count = tok_to_count[&tok.clone()];

                // for every pair
                for i in 0..tok.len() - 1 {
                    let pair = (vec![tok[i]], vec![tok[i+1]]);
                    
                    match pair_to_toks.entry(pair.clone()) {
                        std::collections::hash_map::Entry::Occupied(mut e) => {
                            let set = e.get_mut();

                            // remove from pair's toks.
                            set.remove(&tok);
                        }
                        std::collections::hash_map::Entry::Vacant(_) => {}
                    }

                    match pair_to_count.entry(pair.clone()) {
                        std::collections::hash_map::Entry::Occupied(mut e) => {
                            *e.get_mut() -= tok_count; // remove tok_count from pair count. 
                            
                            // if pair's count is zero.
                            if *e.get() == 0 {
                                // remove pair's pair_to_count entry
                                e.remove();

                                // TODO: also remove the pair's pair_to_toks entry
                            }
                        }
                        std::collections::hash_map::Entry::Vacant(_) => {}
                    }
                }
                let new_tok: Vec<u8>= Vec::new();
                let mut i = 0; 
                while i < tok.len() {
                    if i < tok.len() - 1 && (tok[i], tok[i+1]) == max_pair {
                        new_tok.append(tok[i] + tok[i+1]);
                    }
                }
            }
            
            // ---------------- EFFICIENTLY UPDATE ----------------
        }


    };

    Ok(max_pairs)
}

/// A Python module implemented in Rust.
#[pymodule]
fn rusty_tokey(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(sum_as_string, m)?)?;
    m.add_function(wrap_pyfunction!(merge, m)?)?;
    Ok(())
}
