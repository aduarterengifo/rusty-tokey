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

// (NVM) TODO: maybe start accepting &[u8] instead? keep in mind all hell might break loose.

// TODO: actually use Cow<u8> instead of Vec<u8>

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
    tok_to_count: &HashMap<Vec<Vec<u8>>, usize>,
) -> (
    HashMap<(Vec<u8>, Vec<u8>), usize>,
    HashMap<(Vec<u8>, Vec<u8>), HashSet<Vec<Vec<u8>>>>,
    BinaryHeap<PairHeapEntry>
) {
    let mut pair_to_count: HashMap<(Vec<u8>, Vec<u8>), usize> = HashMap::new();
    let mut pair_to_toks: HashMap<(Vec<u8>, Vec<u8>), HashSet<Vec<Vec<u8>>>> = HashMap::new();
    let mut heap = BinaryHeap::new();

    for (tok, count) in tok_to_count {
        // not big enough for a pair.
        if tok.len() < 2 {
            continue;
        }
        for i in 0..(tok.len() - 1) {
            let pair = (tok[i].clone(), tok[i+1].clone());

            let new_count = match pair_to_count.entry(pair.clone()) {
                std::collections::hash_map::Entry::Occupied(mut e) => {
                    *e.get_mut() += count; // Increment by counter if exists
                    *e.get()
                }
                std::collections::hash_map::Entry::Vacant(e) => {
                    e.insert(*count); // Insert counter if doesn't exist
                    *count
                }
            };

            heap.push(PairHeapEntry {
                count: new_count, 
                pair: pair.clone(),
            });

            pair_to_toks
                .entry(pair.clone())
                .or_insert(HashSet::new())
                .insert(tok.clone());
        }
    }

    (pair_to_count, pair_to_toks, heap)
}


/// we take a map from vector of bytes and a max vocab size.
/// and we return?
#[pyfunction]
fn rusty_merge(mut tok_to_count: HashMap<Vec<Vec<u8>>, usize>, max: usize) -> PyResult<Vec<(Vec<u8>, Vec<u8>)>> {
    println!("we are inside rusty_merge");
    // println!("{:?}", tok_to_count);
    let mut max_pairs = vec![(vec![0; 0], vec![0; 0]); 0];
    let (mut pair_to_count, mut pair_to_toks, mut heap) = get_pairs(&tok_to_count);

    // let max_count_2 =  pair_to_count[&("w".as_bytes().to_vec(),"w".as_bytes().to_vec())];
    // println!("max_count {}", max_count_2);
    // TODO: maintain heap of max-pairs, instead of finding the max_pair on every loop.

    while max_pairs.len() < max {
        // pop from heap.
        if let Some (heap_entry) = heap.pop() {
            if pair_to_count.get(&heap_entry.pair) == Some(&heap_entry.count) {
                let max_pair = heap_entry.pair;

                max_pairs.push(max_pair.clone());

                let max_pair_to_toks = &pair_to_toks[&max_pair];
                
                // for every tok that contains max_pair
                for tok in max_pair_to_toks.clone() {
                    match tok_to_count.entry(tok.clone()) {
                        Entry::Occupied(_) => (),
                        Entry::Vacant(_) => continue, // Skip if doesn't exist
                    };

                    let tok_count = tok_to_count[&tok.clone()];

                    // for every pair in tok
                    for i in 0..tok.len() - 1 {
                        let pair = (tok[i].clone(), tok[i+1].clone());
                        
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
                                *e.get_mut() = e.get_mut().saturating_sub(tok_count); // remove tok_count from pair count. 
                                
                                if *e.get() > 0 {
                                    heap.push(PairHeapEntry { count: *e.get(), pair  });
                                } else {
                                    e.remove();
                                }
                            }
                            std::collections::hash_map::Entry::Vacant(_) => {}
                        };

                    }
                    
                    // construct new_tok
                    let mut new_tok: Vec<Vec<u8>>= Vec::new();
                    
                    let mut i = 0; 

                    while i < tok.len() {
                        if i < tok.len() - 1 && (tok[i].clone(), tok[i+1].clone()) == max_pair {
                            let new_vocab = [&tok[i][..], &tok[i+1][..]].concat();
                            // println!("new_vocab {:?}", String::from_utf8(new_vocab.clone()));
                            new_tok.push(new_vocab);
                            i += 2;
                        } else {
                            new_tok.push(tok[i].clone());
                            i += 1
                        }
                    }

                    // println!("new_tok {:?}", new_tok.clone().iter().map(|x| String::from_utf8(x.clone()).unwrap()).collect::<Vec<String>>().join("") );

                    // increment new_tok count by tok_count
                    *tok_to_count.entry(new_tok.clone()).or_default() += tok_count;
                    
                    // decrement tok count by tok_count
                    // if tok_count is zero -> remove tok entry all together.
                    decrement_or_remove(&mut tok_to_count, tok.clone(), tok_count);

                    // for every pair in new_tok
                    for i in 0..new_tok.clone().len() - 1 {
                        let pair = (new_tok[i].clone(), new_tok[i+1].clone());
                        
                        match pair_to_toks.entry(pair.clone()) {
                            // old_pair
                            std::collections::hash_map::Entry::Occupied(mut e) => {
                                // insert new_tok
                                e.get_mut().insert(new_tok.clone());
                            }
                            // new pair
                            std::collections::hash_map::Entry::Vacant(e) => {
                                e.insert(HashSet::from_iter([new_tok.clone()]));
                            }
                        }

                        *pair_to_count.entry(pair.clone()).or_default() += tok_count;

                        heap.push(PairHeapEntry {
                            count: pair_to_count[&pair],
                            pair: pair.clone(),
                        });

                    }
                }
            }           
        }
    };
    // println!("{:?}", max_pairs);
    Ok(max_pairs)
}

#[pyfunction]
fn rusty_full_merge(filepath: &str, boundaries: Vec<u64>, special_tokens:Vec<String>, max: usize) -> PyResult<Vec<(Vec<u8>, Vec<u8>)>> {
    let tok_to_count = rusty_get_pre_toks(filepath, boundaries, special_tokens);
    // println!("tok_to_count {:?}", tok_to_count);
    return rusty_merge(tok_to_count.unwrap(), max)
}


#[pyfunction]
fn rusty_pre_tok(chunk:&str, special_tokens:Vec<String>) -> HashMap<Vec<Vec<u8>>, usize> {
    let mut tok_to_count: HashMap<Vec<Vec<u8>>, usize> = HashMap::new();
    let pat_special_toks = special_tokens.iter().map(|x: &String| regex::escape(x)).collect::<Vec<String>>().join("|");
    let re_special_toks: Regex = Regex::new(&pat_special_toks).unwrap();

    for regex_match in re_special_toks.split(chunk).flat_map(|subchunk| RE.find_iter(subchunk.unwrap())) {
        let key: Vec<Vec<u8>> = regex_match.unwrap().as_str().as_bytes().iter().map(|&byte| vec![byte]).collect();

        // += 1
        *tok_to_count.entry(key).or_default() += 1;
        
    }

    tok_to_count
}

fn rusty_get_chunk_pre_toks(filepath: &str, start: u64, end: u64, special_tokens:Vec<String>) ->  io::Result<HashMap<Vec<Vec<u8>>, usize>> {
    let mut tok_to_count: HashMap<Vec<Vec<u8>>, usize> = HashMap::new();
    let pat_special_toks = special_tokens.iter().map(|x: &String| regex::escape(x)).collect::<Vec<String>>().join("|");
    let re_special_toks: Regex = Regex::new(&pat_special_toks).unwrap();
    let mut file = File::open(filepath)?;
    file.seek(SeekFrom::Start(start))?;
    let mut buffer = vec![0u8; (end - start) as usize];
    let bytes_read = file.read(&mut buffer)?;
    let chunk = String::from_utf8_lossy(&buffer[..bytes_read]);
    for regex_match in re_special_toks.split(&chunk).flat_map(|subchunk| RE.find_iter(subchunk.unwrap())) {
        let key: Vec<Vec<u8>> = regex_match.unwrap().as_str().as_bytes().iter().map(|&byte| vec![byte]).collect();
        // += 1
        *tok_to_count.entry(key).or_default() += 1;
    }
    Ok(tok_to_count)   
}

#[pyfunction]
fn rusty_get_pre_toks(filepath: &str, boundaries: Vec<u64>, special_tokens:Vec<String>) -> io::Result<HashMap<Vec<Vec<u8>>, usize>> {

    // let pat_special_toks = special_tokens.iter().map(|x: &String| regex::escape(x)).collect::<Vec<String>>().join("|");
    let r: Vec<(u64, u64)> = boundaries.windows(2).map(|x| (x[0],x[1])).collect();
    let map = r
        .par_iter()
        .map(|(start,end)| {
            rusty_get_chunk_pre_toks(filepath,*start,*end, special_tokens.clone())
        })
        .collect::<Vec<_>>()
        .into_iter()
        .fold(HashMap::new(), |mut acc, chunk_map| {
            for (key, value) in chunk_map.unwrap() {
                *acc.entry(key).or_default() += value;
            }
            acc
        });
    Ok(map)
}

/// A Python module implemented in Rust.
#[pymodule]
fn rusty_tokey(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(rusty_merge, m)?)?;
    m.add_function(wrap_pyfunction!(rusty_full_merge, m)?)?;
    m.add_function(wrap_pyfunction!(rusty_pre_tok, m)?)?;
    m.add_function(wrap_pyfunction!(rusty_get_pre_toks, m)?)?;
    Ok(())
}
