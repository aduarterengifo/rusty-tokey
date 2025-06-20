# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Rust/Python hybrid project that implements a Byte Pair Encoding (BPE) tokenizer. The core tokenization algorithms are implemented in Rust for performance, with Python bindings provided via PyO3. The project focuses on efficient text tokenization with support for special tokens and multiprocessing.

## Key Components Architecture

- **Rust Core (`src/lib.rs`)**: Contains the main BPE merge algorithm (`rusty_merge`) and helper functions. Uses PyO3 to expose functions to Python. The merge function implements an optimized version of the BPE algorithm with efficient pair counting and token updating.

- **Python Interface (`test.py`)**: Main training script that orchestrates the BPE training process. Handles file chunking, multiprocessing for pre-tokenization, and calls into Rust for the merge operations. Uses regex for tokenization patterns and implements heap-based optimization.

- **Build System**: Uses maturin for building the Rust-Python extension, configured for stable ABI (abi3-py38) for compatibility across Python versions.

## Development Commands

### Building the Extension
```bash
# Build the Rust extension and install in development mode
maturin develop
```

### Running Tests/Training
```bash
# Run the main BPE training script
python test.py
```

### Dependencies Management
```bash
# Install Python dependencies
uv sync

# Add new dependencies
uv add <package>
```

### Development Environment
```bash
# Enter Nix development shell (if using Nix)
nix develop
```

## Key Implementation Details

- The Rust implementation maintains two key data structures: `pair_to_count` (frequency of byte pairs) and `pair_to_toks` (which tokens contain each pair)
- The Python side handles file I/O, chunking for parallel processing, and pre-tokenization using regex patterns
- Special tokens like `<|endoftext|>` are handled separately in the tokenization pipeline
- The merge algorithm efficiently updates pair counts when tokens are merged, avoiding full recomputation