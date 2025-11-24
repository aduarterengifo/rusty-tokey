use fancy_regex::Regex;
use interner::TokenIdsId;
use interner::VocabInterner;
use interner::TokSeqInterner;
use linked_array::LinkedArray;
use linked_array::LinkedArrayNode;
use once_cell::sync::Lazy;
use pyo3::prelude::*;
use rayon::prelude::*;
use token_interner::TokenId;
use std::cmp::Ordering;
use std::collections::hash_map::Entry;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::fs::File;
use std::hash::Hash;
use std::io::{self, Read, Seek, SeekFrom};

pub mod linked_array;
pub mod token_interner;
pub mod interner;

const PAT: &str = r"'(?:[sdmt]|ll|ve|re)| ?\p{L}+| ?\p{N}+| ?[^\s\p{L}\p{N}]+|\s+(?!\S)|\s+";

static RE: Lazy<Regex> = Lazy::new(|| Regex::new(PAT).unwrap());

#[derive(Eq, PartialEq, PartialOrd)]
struct PairHeapEntry {
    count: usize,
    pair: (Vec<u8>, Vec<u8>),
}

impl Ord for PairHeapEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        // Max heap: higher count = higher priority
        self.count
            .cmp(&other.count)
            .then(self.pair.cmp(&other.pair)) // tie-breaking
    }
}

fn decrement_or_remove<T: std::cmp::Eq + Hash>(
    map: &mut HashMap<T, usize>,
    key: T,
    amount: usize,
) -> () {
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

fn get_pairs(
    tok_to_count: &HashMap<TokenIdsId, usize>,
    vocab_interner: &mut VocabInterner,
    tok_seq_interner: &mut TokSeqInterner
) -> (
    HashMap<(TokenId, TokenId), usize>,
    HashMap<(TokenId, TokenId), HashMap<TokenIdsId, HashSet<usize>>>,
    BinaryHeap<PairHeapEntry>,
) {
    let estimated_pairs = tok_to_count.len() * 2;
    let mut pair_to_count: HashMap<(TokenId, TokenId), usize> =
        HashMap::with_capacity(estimated_pairs);
    let mut pair_to_toks: HashMap<(TokenId, TokenId), HashMap<TokenIdsId, HashSet<usize>>> =
        HashMap::with_capacity(estimated_pairs);
    let mut heap = BinaryHeap::new();

    for (tok_id, count) in tok_to_count {
        // not big enough for a pair.
        let tok = tok_seq_interner.get(*tok_id);
        if tok.len() < 2 {
            continue;
        }
        for i in 0..(tok.len() - 1) {
            let pair = (tok[i].clone(), tok[i + 1].clone());

            let new_count = {
                let entry = pair_to_count.entry(pair.clone()).or_insert(0);
                *entry += count;
                *entry
            };

            heap.push(PairHeapEntry {
                count: new_count,
                pair: (
                    vocab_interner.get(pair.0).to_vec(),
                    vocab_interner.get(pair.1).to_vec(),
                ),
            });

            pair_to_toks
                .entry(pair)
                .or_insert_with(HashMap::new)
                .entry(*tok_id)
                .or_insert_with(HashSet::new)
                .insert(i);
        }
    }

    (pair_to_count, pair_to_toks, heap)
}

fn rusty_get_chunk_pre_toks(
    filepath: &str,
    start: u64,
    end: u64,
    special_tokens: Vec<String>,
) -> io::Result<HashMap<Vec<u8>, usize>> {
    let mut tok_to_count: HashMap<Vec<u8>, usize> = HashMap::new();
    let pat_special_toks = special_tokens
        .iter()
        .map(|x: &String| regex::escape(x))
        .collect::<Vec<String>>()
        .join("|");
    let re_special_toks: Regex = Regex::new(&pat_special_toks).unwrap();
    let mut file = File::open(filepath)?;
    file.seek(SeekFrom::Start(start))?;
    let mut buffer = vec![0u8; (end - start) as usize];
    let bytes_read = file.read(&mut buffer)?;
    let chunk = String::from_utf8_lossy(&buffer[..bytes_read]);
    for regex_match in re_special_toks
        .split(&chunk)
        .flat_map(|subchunk| RE.find_iter(subchunk.unwrap()))
    {
        let key: Vec<u8> = regex_match.unwrap().as_str().as_bytes().to_vec();
        // += 1
        *tok_to_count.entry(key).or_default() += 1;
    }
    Ok(tok_to_count)
}

fn rusty_get_pre_toks(
    filepath: &str,
    boundaries: Vec<u64>,
    special_tokens: Vec<String>,
) -> io::Result<(HashMap<TokenIdsId, usize>, VocabInterner, TokSeqInterner)> {
    let item_to_id = HashMap::new();
    let items = Vec::new();
    let mut vocab_interner = VocabInterner {
        items,
        item_to_id,
    };

    let toq_seq_item_to_id = HashMap::new();
    let toq_seq_items = Vec::new();

    let mut tok_seq_interner = TokSeqInterner {
        items: toq_seq_items, 
        item_to_id: toq_seq_item_to_id
    };

    // let pat_special_toks = special_tokens.iter().map(|x: &String| regex::escape(x)).collect::<Vec<String>>().join("|");
    let r: Vec<(u64, u64)> = boundaries.windows(2).map(|x| (x[0], x[1])).collect();

    let intermediate = r
        .par_iter()
        .map(|(start, end)| {
            rusty_get_chunk_pre_toks(filepath, *start, *end, special_tokens.clone()).unwrap()
        })
        .collect::<Vec<_>>();

    let mut toks: HashSet<Vec<u8>> = HashSet::new();

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
        vocab_interner.intern(tok);
    }

    let mut result = HashMap::new();

    for chunk_map in intermediate {
        for (raw, count) in chunk_map {
            let tok_ids: Vec<TokenId> = raw
                .chunks(1)
                .map(|byte| vocab_interner.intern(byte.to_vec()))
                .collect();

            let toq_ids_id = tok_seq_interner.intern(tok_ids);

            *result.entry(toq_ids_id).or_default() += count;
        }
    }

    Ok((result, vocab_interner, tok_seq_interner))
}

fn get_adjacent_pairs(linked_tok: &LinkedArray<u32>, position: usize) -> Vec<(&LinkedArrayNode<u32>, &LinkedArrayNode<u32>)> {
    let mut pairs = Vec::new();
    
    if let Some(curr) = linked_tok.get(position) {
        // Previous pair
        if let Some(prev) = linked_tok.prev(position) {
            pairs.push((prev, curr));
        }
        
        // Next pair
        if let Some(next) = linked_tok.next(position) {
            if let Some(next_next) = linked_tok.next(next.idx) {
                pairs.push((next, next_next));
            }
        }
    }
    
    pairs
}

fn old_pair_accounting(pair: (u32,u32), tok_id: u32, tok_count: usize, heap: &mut BinaryHeap<PairHeapEntry>, pair_to_toks : &mut HashMap<(u32, u32), HashMap<u32, HashSet<usize>>>,pair_to_count: &mut HashMap<(u32, u32), usize>, vocab_interner: &mut VocabInterner) -> () {
    // if there is a map associated with prev_pair
    if let Some(map) = pair_to_toks.get_mut(&pair) {
        // remote it.
        map.remove(&tok_id);
    }
    
    // if there is a count associated with prev_pair
    if let Some(count) = pair_to_count.get_mut(&pair) {
        // substract tok_count from it.
        *count = count.saturating_sub(tok_count);
        // if its bigger than 0 add it to the heap.
        if *count > 0 {
            heap.push(PairHeapEntry {
                count: *count,
                pair: (
                    vocab_interner.get(pair.0).to_vec(),
                    vocab_interner.get(pair.1).to_vec(),
                ),
            });
        } else {
        // if the count associated with prev_pair is 0 remove entry.
            pair_to_count.remove(&pair);
        }
    }
}

fn rusty_merge(
    mut tok_to_count: HashMap<TokenIdsId, usize>,
    max: usize,
    vocab_interner: &mut VocabInterner,
    tok_seq_interner: &mut TokSeqInterner
) -> PyResult<Vec<(Vec<u8>, Vec<u8>)>> {
    let mut max_pairs = vec![(vec![0; 0], vec![0; 0]); 0];
    let (mut pair_to_count, mut pair_to_toks, mut heap) = get_pairs(&tok_to_count, vocab_interner, tok_seq_interner);

    let mut tok_to_linked: HashMap<TokenIdsId, LinkedArray<TokenId>> = HashMap::new();

    while max_pairs.len() < max {
        // pop from heap.
        if let Some(heap_entry) = heap.pop() {
            let max_pair = (
                vocab_interner.intern(heap_entry.pair.0),
                vocab_interner.intern(heap_entry.pair.1),
            );
            if pair_to_count.get(&max_pair) == Some(&heap_entry.count) {
                max_pairs.push((
                    vocab_interner.get(max_pair.0).to_vec(),
                    vocab_interner.get(max_pair.1).to_vec(),
                ));

                // for every tok that contains max_pair
                for (tok_id, positions) in pair_to_toks[&max_pair].clone().into_iter() {
                    // for every position 
                    for position in positions {
                        let tok = tok_seq_interner.get(tok_id).clone();

                        match tok_to_count.entry(tok_id) { // for idx in
                            Entry::Occupied(_) => (),
                            Entry::Vacant(_) => continue, // Skip if doesn't exist
                        };

                        let tok_count = if let Some(&count) = tok_to_count.get(&tok_id) {
                            count
                        } else {
                            continue;
                        };
                        let linked_tok = tok_to_linked.entry(tok_id)
                            .or_insert_with(|| {
                                let tok_vec = tok_seq_interner.get(tok_id).clone();
                                LinkedArray::new(tok_vec)
                            });

                        for node_pair in get_adjacent_pairs(&linked_tok, position) {
                            let pair = (node_pair.0.elem, node_pair.1.elem);
                            old_pair_accounting(pair, tok_id, tok_count, &mut heap, &mut pair_to_toks, &mut pair_to_count, vocab_interner);
                        }

                        let new_tok = match linked_tok.replace_pair(position, vocab_interner) {
                            Ok(tok) => tok,
                            Err(e) => {
                                println!("Error merging at position {}: {:?}", position, e);
                                continue; // Skip this position and continue with next
                            }
                        };
                        
                        let new_interned_tok = tok_seq_interner.intern(new_tok.to_vec());

                        let interned_tok = tok_seq_interner.intern(tok.to_vec());

                        // increment new_tok count by tok_count
                        *tok_to_count.entry(new_interned_tok).or_default() += tok_count;

                        if let Some(count) = tok_to_count.get_mut(&interned_tok) {
                            *count = count.saturating_sub(tok_count);
                            if *count == 0 {
                                tok_to_count.remove(&interned_tok);
                            }
                        }


                        for node_pair in get_adjacent_pairs(&linked_tok, position) {
                            let pair = (node_pair.0.elem, node_pair.1.elem);

                            pair_to_toks
                                    .entry(pair)
                                    .or_insert_with(HashMap::new)
                                    .entry(new_interned_tok)
                                    .or_insert_with(HashSet::new)
                                    .insert(node_pair.0.idx);

                            *pair_to_count.entry(pair).or_default() += tok_count;
                        }
                    }
                }
            }
        }
    }
    Ok(max_pairs)
}

#[pyfunction]
fn rusty_full_merge(
    filepath: &str,
    boundaries: Vec<u64>,
    special_tokens: Vec<String>,
    max: usize,
) -> PyResult<Vec<(Vec<u8>, Vec<u8>)>> {
    let (tok_to_count, mut vocab_interner, mut toq_seq_interner) =
        rusty_get_pre_toks(filepath, boundaries, special_tokens).unwrap();
    return rusty_merge(tok_to_count, max, &mut vocab_interner, &mut toq_seq_interner);
}

/// A Python module implemented in Rust.
#[pymodule]
fn rusty_tokey(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(rusty_full_merge, m)?)?;
    Ok(())
}
