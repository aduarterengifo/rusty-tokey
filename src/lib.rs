use pyo3::prelude::*;
use std::collections::{HashMap, HashSet, BinaryHeap};
use std::cmp::Ordering;
use std::hash::Hash;
use std::collections::hash_map::Entry;
use fancy_regex::Regex;
use once_cell::sync::Lazy;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use rayon::prelude::*;

const PAT: &str = r"'(?:[sdmt]|ll|ve|re)| ?\p{L}+| ?\p{N}+| ?[^\s\p{L}\p{N}]+|\s+(?!\S)|\s+";

struct TokenInterner {
    tokens: Vec<Vec<u8>>,
    token_to_id: HashMap<Vec<u8>, u32>
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

    fn get_bytes(&self, id:TokenId) -> &[u8] {
        &self.tokens[id as usize]
    }
 }

static RE: Lazy<Regex> = Lazy::new(|| Regex::new(PAT).unwrap());

 #[derive(Eq, PartialEq)]
 #[derive(PartialOrd)]
struct PairHeapEntry {
    count: usize,
    pair: (Vec<u8>, Vec<u8>),
}

impl Ord for PairHeapEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        // Max heap: higher count = higher priority
        self.count.cmp(&other.count)
            .then(self.pair.cmp(&other.pair))  // tie-breaking
    }
}

fn decrement_or_remove<T: std::cmp::Eq + Hash>(map: &mut HashMap<T, usize>, key: T, amount: usize) -> () {
    match map.entry(key) {
        Entry::Occupied(mut entry) => {
            // add
            *entry.get_mut() = entry.get_mut().saturating_sub(amount);
            
            // if the value becomes 0 as a result remove.
            if *entry.get() == 0 {
                entry.remove();
            }
        }
        Entry::Vacant(_) => {}
    }
}

// borrows: 
//   tok_to_count: hashmap
fn get_pairs(
    tok_to_count: &HashMap<Vec<TokenId>, usize>,
    interner: &mut TokenInterner
) -> (
    HashMap<(TokenId, TokenId), usize>,
    HashMap<(TokenId, TokenId), HashSet<Vec<TokenId>>>,
    BinaryHeap<PairHeapEntry>
) {
    let estimated_pairs = tok_to_count.len() * 2;
    let mut pair_to_count: HashMap<(TokenId, TokenId), usize> = HashMap::with_capacity(estimated_pairs);
    let mut pair_to_toks: HashMap<(TokenId, TokenId), HashSet<Vec<TokenId>>> = HashMap::with_capacity(estimated_pairs);
    let mut heap = BinaryHeap::new();

    for (tok, count) in tok_to_count {
        // not big enough for a pair.
        if tok.len() < 2 {
            continue;
        }
        for i in 0..(tok.len() - 1) {
            let pair = (tok[i].clone(), tok[i+1].clone());

            let new_count = {
                let entry = pair_to_count.entry(pair.clone()).or_insert(0);
                *entry += count;
                *entry
            };

            heap.push(PairHeapEntry {
                count: new_count, 
                pair: (interner.get_bytes(pair.0).to_vec(), interner.get_bytes(pair.1).to_vec()),
            });

            pair_to_toks
                .entry(pair)
                .or_insert(HashSet::new())
                .insert(tok.clone());
        }
    }

    (pair_to_count, pair_to_toks, heap)
}

fn rusty_get_chunk_pre_toks(filepath: &str, start: u64, end: u64, special_tokens:Vec<String>) ->  io::Result<HashMap<Vec<u8>, usize>> {
    let mut tok_to_count: HashMap<Vec<u8>, usize> = HashMap::new();
    let pat_special_toks = special_tokens.iter().map(|x: &String| regex::escape(x)).collect::<Vec<String>>().join("|");
    let re_special_toks: Regex = Regex::new(&pat_special_toks).unwrap();
    let mut file = File::open(filepath)?;
    file.seek(SeekFrom::Start(start))?;
    let mut buffer = vec![0u8; (end - start) as usize];
    let bytes_read = file.read(&mut buffer)?;
    let chunk = String::from_utf8_lossy(&buffer[..bytes_read]);
    for regex_match in re_special_toks.split(&chunk).flat_map(|subchunk| RE.find_iter(subchunk.unwrap())) {
        let key: Vec<u8> = regex_match.unwrap().as_str().as_bytes().to_vec();
        // += 1
        *tok_to_count.entry(key).or_default() += 1;
    }
    Ok(tok_to_count)   
}

fn rusty_get_pre_toks(filepath: &str, boundaries: Vec<u64>, special_tokens:Vec<String>) -> io::Result<(HashMap<Vec<TokenId>, usize>, TokenInterner)> {
    let token_to_id = HashMap::new();
    let tokens = Vec::new();
    let mut interner = TokenInterner {tokens, token_to_id};
    // let pat_special_toks = special_tokens.iter().map(|x: &String| regex::escape(x)).collect::<Vec<String>>().join("|");
    let r: Vec<(u64, u64)> = boundaries.windows(2).map(|x| (x[0],x[1])).collect();

    let intermediate = r
        .par_iter()
        .map(|(start,end)| {
            rusty_get_chunk_pre_toks(filepath,*start,*end, special_tokens.clone()).unwrap()
        })
        .collect::<Vec<_>>();

    let mut toks :HashSet<Vec<u8>> = HashSet::new(); 

    for chunkmap in &intermediate {
        for raw in chunkmap.keys() {
            toks.insert(raw.clone());
             for byte in raw {
                  toks.insert(vec![*byte]);
              }
        }
    }

    let mut sorted_toks: Vec<Vec<u8>> = toks.into_iter().collect();
    sorted_toks.sort();

    for tok in sorted_toks {
        interner.intern(tok);
    }

    let mut result = HashMap::new();

    for chunk_map in intermediate {
        for (raw, count) in chunk_map {
            let token_ids: Vec<TokenId> = raw
            .chunks(1)
            .map(|byte| interner.intern(byte.to_vec()))
            .collect();

            *result.entry(token_ids).or_default() += count;
        }
    }

    Ok((result, interner))
}

fn rusty_merge(mut tok_to_count: HashMap<Vec<TokenId>, usize>, max: usize, interner: &mut TokenInterner) -> PyResult<Vec<(Vec<u8>, Vec<u8>)>> {
    let mut max_pairs = vec![(vec![0; 0], vec![0; 0]); 0];
    let (mut pair_to_count, mut pair_to_toks, mut heap) = get_pairs(&tok_to_count, interner);

    while max_pairs.len() < max {
        // pop from heap.
        if let Some (heap_entry) = heap.pop() {
            let max_pair = (interner.intern(heap_entry.pair.0), interner.intern(heap_entry.pair.1));
            if pair_to_count.get(&max_pair) == Some(&heap_entry.count) {

                max_pairs.push((interner.get_bytes(max_pair.0).to_vec(),interner.get_bytes(max_pair.1).to_vec()));
                
                 let tokens_to_process: Vec<Vec<TokenId>> = pair_to_toks[&max_pair].iter().cloned().collect();
                // for every tok that contains max_pair
                for tok in &tokens_to_process {
                    match tok_to_count.entry(tok.to_vec()) {
                        Entry::Occupied(_) => (),
                        Entry::Vacant(_) => continue, // Skip if doesn't exist
                    };

                    let tok_count = if let Some(&count) = tok_to_count.get(tok) {
                        count
                    } else {
                        continue;
                    };

                    // for every pair in tok
                    for i in 0..tok.len() - 1 {
                        let pair = (tok[i], tok[i+1]);
                        
                        match pair_to_toks.entry(pair) {
                            std::collections::hash_map::Entry::Occupied(mut e) => {
                                let set = e.get_mut();

                                // remove from pair's toks.
                                set.remove(tok);
                            }
                            std::collections::hash_map::Entry::Vacant(_) => {}
                        }

                        match pair_to_count.entry(pair) {
                            std::collections::hash_map::Entry::Occupied(mut e) => {
                                *e.get_mut() = e.get_mut().saturating_sub(tok_count); // remove tok_count from pair count. 
                                
                                if *e.get() > 0 {
                                    heap.push(PairHeapEntry { count: *e.get(), pair:(interner.get_bytes(pair.0).to_vec(), interner.get_bytes(pair.1).to_vec())  });
                                } else {
                                    e.remove();
                                }
                            }
                            std::collections::hash_map::Entry::Vacant(_) => {}
                        };

                    }
                    
                    // construct new_tok
                    let mut new_tok: Vec<TokenId>= Vec::new();
                    
                    let mut i = 0; 

                    while i < tok.len() {
                        if i < tok.len() - 1 && (tok[i], tok[i+1]) == max_pair {
        
                            let bytes1 = interner.get_bytes(tok[i]);
                            let bytes2 = interner.get_bytes(tok[i+1]);

                            let merged_bytes = [bytes1, bytes2].concat();

                            let new_token_id = interner.intern(merged_bytes);

                            new_tok.push(new_token_id);
                            i += 2;
                        } else {
                            new_tok.push(tok[i]);
                            i += 1
                        }
                    }

                    // increment new_tok count by tok_count
                    *tok_to_count.entry(new_tok.clone()).or_default() += tok_count;
                    
                    // decrement tok count by tok_count
                    // if tok_count is zero -> remove tok entry all together.
                    decrement_or_remove(&mut tok_to_count, tok.to_vec(), tok_count);

                    // for every pair in new_tok
                    for i in 0..new_tok.len() - 1 {
                        let pair = (new_tok[i], new_tok[i+1]);

                        pair_to_toks
                            .entry(pair)
                            .or_insert_with(HashSet::new)
                            .insert(new_tok.clone());
                        

                        *pair_to_count.entry(pair).or_default() += tok_count;

                        heap.push(PairHeapEntry {
                            count: pair_to_count[&pair],
                            pair: (interner.get_bytes(pair.0).to_vec(), interner.get_bytes(pair.1).to_vec()),
                        });

                    }
                }
            }           
        }
    };
    Ok(max_pairs)
}


#[pyfunction]
fn rusty_full_merge(filepath: &str, boundaries: Vec<u64>, special_tokens:Vec<String>, max: usize) -> PyResult<Vec<(Vec<u8>, Vec<u8>)>> {
    let (tok_to_count,mut interner) = rusty_get_pre_toks(filepath, boundaries, special_tokens).unwrap();
    return rusty_merge(tok_to_count, max, &mut interner);
}

/// A Python module implemented in Rust.
#[pymodule]
fn rusty_tokey(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(rusty_full_merge, m)?)?;
    Ok(())
}
