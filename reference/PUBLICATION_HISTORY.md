# Published source history

The public branch is a separate copy of the local milestone history, prepared
with the requested identity `ValentinHirschi <valentin.hirschi@gmail.com>`.
The original local refs remain intact. Commit messages, dates, parent
relationships and empty milestones are retained; signatures are not copied.
External references, generated corpora, proofs, build directories and local
environment files are excluded from every published tree. App-local refs and
tags are not pushed.

[PUBLICATION_COMMIT_MAP.tsv](PUBLICATION_COMMIT_MAP.tsv) maps recorded local
source commits to their public equivalents. Its four tab-separated columns
are original commit, published commit, original tree and published tree.
Historical benchmark records continue to contain their original source
identifiers and immutable binary/input/output hashes. Use the mapping to
inspect the corresponding code after cloning the public repository. Mapping
metadata itself is added after the recorded source commits, so the map does
not need to contain its own commit.

Publication uses an explicit branch push without force. Neither reference
checkouts nor large benchmark artifacts are distributed through Git; pinned
dependency and corpus setup is documented in [SETUP.md](../docs/SETUP.md).
