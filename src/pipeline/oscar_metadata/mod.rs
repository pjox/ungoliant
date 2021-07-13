/*! Generates OSCAR 2018-compatible corpora augmented with metadata.

OSCAR v1.1 holds identified sentences in `<lang>.txt` files
just like OSCAR 2018, along with metadata that are in a separate `<lang>_meta.json`.

# Code structure and nomenclature

CommonCrawl dumps are distributed in gzipped text files named `Shards`.

Each `Shard` contains `Records`. A `Record` can be seen as the content of a single page crawl.
The pipeline operates on record-level, filters lines deemed too short and tries to identify the language of each line.
At the end of this process, the record is a [document::Document].

The [document::Document] is then split into numerous [document::Piece], each holding consecutive lines of a given language.
Each [document::Piece] is transformed into [document::MergedPiece], which holds a newline-separated concatenation of sentences.

Note that we can have a `n -> 1` relation between pieces and merged pieces,
since there is a way of merging n identical language pieces into a single merged one.

OSCAR is comprised of language folders, holding content and metadata files of a given size.
Within each folder are laid out files following this naming scheme:

- <lang>_part_<n>.txt.gz
- <lang>_meta_part_<n>.json.gz

These files are named Parts and aren't written in one time.
The pipeline names [document::PartChunk] content and metadata that will be written in one time.
# Metadata

Metadata maps to sentences in the content file.
Relevant lines are at `[offset, offset+nb_sentences]`, with the assumption that lines are 0-indexed.
## Example

```json
{
    "headers": {
      "warc-identified-content-language": "fra",
      "warc-type": "conversion",
      "warc-record-id": "<urn:uuid:00000000-0000-0000-0000-000000000000>",
      "warc-block-digest": "sha1:7X6XVXEBXADSGELSDQP4P2U5XLAAA5P6",
      "warc-target-uri": "https://foo.bar",
      "warc-date": "2021-02-24T17:11:25Z",
      "content-length": "4463",
      "warc-refers-to": "<urn:uuid:00000000-0000-0000-0000-000000000000>",
      "content-type": "text/plain"
    },
    "offset": 34124,
    "nb_sentences": 3
}
``

This particular record begins at offset `34124+1` and ends at `34124+3`.

// # Chunks
// When processing a record that holds sentences in multiple languages,
// There is the need to extract each contiguous sequence of sentences that share the same language.
// Chunks are these contiguous sequences, and the [chunks] module deals with them.
!*/
mod chunks;
pub mod document;
pub mod metadata;
#[allow(clippy::module_inception)]
pub mod oscar_metadata;
