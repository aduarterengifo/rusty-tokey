import os
import regex as re
from typing import BinaryIO
import cProfile
import pstats
from rusty_tokey import rusty_full_merge


PAT = r"""'(?:[sdmt]|ll|ve|re)| ?\p{L}+| ?\p{N}+| ?[^\s\p{L}\p{N}]+|\s+(?!\S)|\s+"""
PAT_RE = re.compile(PAT)


def find_chunk_boundaries(file: BinaryIO, desired_num_chunks: int, split_special_token: bytes) -> list[int]:
    """
    Chunk the file into parts that can be counted independently.
    May return fewer chunks if the boundaries end up overlapping.
    """
    assert isinstance(split_special_token, bytes), "Must represent special token as a bytestring"

    # Get total file size in bytes
    file.seek(0, os.SEEK_END)
    file_size = file.tell()
    file.seek(0)

    chunk_size = file_size // desired_num_chunks

    # Initial guesses for chunk boundary locations, uniformly spaced
    # Chunks start on previous index, don't include last index
    chunk_boundaries = [i * chunk_size for i in range(desired_num_chunks + 1)]
    chunk_boundaries[-1] = file_size

    mini_chunk_size = 4096  # Read ahead by 4k bytes at a time

    for bi in range(1, len(chunk_boundaries) - 1):
        initial_position = chunk_boundaries[bi]
        file.seek(initial_position)  # Start at boundary guess
        while True:
            mini_chunk = file.read(mini_chunk_size)  # Read a mini chunk

            # If EOF, this boundary should be at the end of the file
            if mini_chunk == b"":
                chunk_boundaries[bi] = file_size
                break

            # Find the special token in the mini chunk
            found_at = mini_chunk.find(split_special_token)
            if found_at != -1:
                chunk_boundaries[bi] = initial_position + found_at
                break
            initial_position += mini_chunk_size

    # Make sure all boundaries are unique, but might be fewer than desired_num_chunks
    return sorted(set(chunk_boundaries))

def rusty_train_bpe(
    input_path: str, vocab_size: int, special_tokens: list[str]
) -> tuple[dict[int, bytes], list[tuple[bytes, bytes]]]:
    special_tokens_len = len(special_tokens)
    stopping_condition = vocab_size - 256 - special_tokens_len
    # open the input path
    with open(input_path, "rb") as f:
        # find the chunk boundaries
        boundaries = find_chunk_boundaries(f, 16, "<|endoftext|>".encode("utf-8"))

        max_pairs = rusty_full_merge(input_path, boundaries, special_tokens, stopping_condition)
        # print('max_pairs', max_pairs)
        vocab: dict[int, bytes] = {}
        # single-bytes
        for i in range(0, 256):
            vocab[i] = bytes([i])
        # special_tokens
        for i, token in enumerate(special_tokens):
            vocab[i + 256] = token.encode("utf-8")

        # vocab
        for i, (a, b) in enumerate(max_pairs):
            vocab[i + 256 + special_tokens_len] = a + b
        return (vocab, max_pairs)


if __name__ == "__main__":
    profiler = cProfile.Profile()
    profiler.enable()
    (vocab, max_pairs) = rusty_train_bpe("./python/rusty_tokey/data/tinystories_sample_5M.txt", 400, ["<|endoftext|>"])
    print("max_pairs", max_pairs)
    print("vocab", vocab)
    profiler.disable()
    stats = pstats.Stats(profiler).sort_stats("cumtime")
    stats.print_stats(20)  # Show top 20 slowest functions
