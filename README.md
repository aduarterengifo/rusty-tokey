# rusty-tokey

```
                 _         _        _                
 _ __ _   _ ___ | |_ _   _| |_ ___ | | _____ _   _ 
| '__| | | / __|| __| | | | __/ _ \| |/ / _ \ | | |
| |  | |_| \__ \| |_| |_| | || (_) |   <  __/ |_| |
|_|   \__,_|___/ \__|\__, |\__\___/|_|\_\___|\__, |
                     |___/                   |___/ 
```

Byte Pair Encoding tokenizer. Rust core with Python bindings.

## Build

```bash
maturin develop
```

## Run

```bash
python test.py
```

## What it does

- Pre-tokenizes text using regex patterns
- Counts byte pair frequencies 
- Merges most frequent pairs iteratively
- Outputs vocabulary and merge operations

Core algorithm in Rust for speed. Python handles I/O and orchestration.