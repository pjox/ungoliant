use std::collections::HashMap;
use std::collections::HashSet;
use std::convert::TryFrom;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;

use flate2::Compression;
use serial_test::serial;
use ungoliant::error;
use ungoliant::lang::LANG;
use ungoliant::pipeline::Metadata;
use ungoliant::pipeline::OscarMetadata;
use ungoliant::shard;
use ungoliant::shard::wet;
use warc::header::WarcHeader;
use warc::RawRecord;

#[test]
//todo assert error type
fn pipeline_no_folders() {
    let src = PathBuf::from("svdkjljlkmjlmdsfljkf");
    let dst = PathBuf::from("fzjoijzoecijzoiej");
    let lid_path = PathBuf::from("lid.176.bin");

    let p = OscarMetadata::new(src, dst, lid_path, 500_000_000);
    assert!(p.run().is_err());
}

fn gen_test_shards(src: &Path, dst: &Path) -> Result<(), error::Error> {
    for shard in std::fs::read_dir(src)? {
        let shard = shard?;
        let records = shard::wet::Wet::from_path_gzip(shard.path())?;
        let dst_path: PathBuf = [
            dst.to_str().unwrap(),
            &shard.file_name().into_string().unwrap(),
        ]
        .iter()
        .collect();
        let dst = File::create(dst_path)?;

        let buf = flate2::write::GzEncoder::new(dst, Compression::default());
        let mut writer = warc::WarcWriter::new(buf);

        for (_, record) in records.skip(0).take(30).enumerate() {
            // println!("writing record {}", idx);
            let record = record.unwrap();
            writer.write_raw(&record)?;
        }
    }
    Ok(())
}

fn get_lang_data(lang: &'static str, dst: &PathBuf) -> (Vec<String>, Vec<Metadata>) {
    // generate lang file paths
    let mut langfile = dst.clone();
    let mut metafile = dst.clone();
    langfile.push(format!("{}.txt", lang));
    metafile.push(format!("{}_meta.json", lang));

    // open sentence/metadata files
    let mut langfile = File::open(langfile).unwrap();
    let metafile = File::open(metafile).unwrap();

    // read sentences
    let mut sentences = String::new();
    langfile.read_to_string(&mut sentences).unwrap();

    // put sentences and metadata into vectors
    let sentences: Vec<String> = sentences.lines().map(|x| x.to_string()).collect();

    let metadata = serde_json::from_reader(metafile).unwrap();

    (sentences, metadata)
}

#[test]
#[serial]
#[ignore]
fn assert_meta_final_offset() {
    // generate test shards
    // and run pipeline on them
    let mut src_gen = PathBuf::from("result_1");
    let src = PathBuf::from("src_intg_final_offset_single");
    let dst = PathBuf::from("dst_intg_final_offset_single");
    std::fs::create_dir(&src).unwrap();
    std::fs::create_dir(&dst).unwrap();
    gen_test_shards(&src_gen, &src)
        .expect("ensure to have a folder named result_1 containing 0.txt.gz as test shard.");
    let lid_path = PathBuf::from("lid.176.bin");

    let p = OscarMetadata::new(src.clone(), dst.clone(), lid_path, 500_000_000);
    p.run().unwrap();

    for lang in LANG.iter() {
        // generate lang file paths
        let mut langfile = dst.clone();
        let mut metafile = dst.clone();
        langfile.push(format!("{}.txt", lang));
        metafile.push(format!("{}_meta.json", lang));

        // open sentence/metadata files
        let mut langfile = File::open(langfile).unwrap();
        let metafile = File::open(metafile).unwrap();

        // get number of sentences from corpus
        let mut sentences = String::new();
        langfile.read_to_string(&mut sentences).unwrap();
        let nb_sentences_corpus = sentences.lines().count();

        let mut metadata: Vec<Metadata> = serde_json::from_reader(metafile).unwrap();
        metadata.sort_by(|a, b| a.offset.cmp(&b.offset));

        // get final offset + nb_sentences
        // add 1 for last space
        let nb_sentences_metadata = match metadata.last() {
            Some(meta) => meta.offset + meta.nb_sentences + 1,
            None => 0,
        };

        assert_eq!(nb_sentences_corpus, nb_sentences_metadata);
    }

    std::fs::remove_dir_all(&src).unwrap();
    std::fs::remove_dir_all(&dst).unwrap();
}

#[test]
#[serial]
#[ignore]
fn assert_meta_successive_offsets() {
    // generate test shards
    // and run pipeline on them
    let mut src_gen = PathBuf::from("result_1");
    let src = PathBuf::from("src_intg_successive_offsets_single");
    let dst = PathBuf::from("dst_intg_successive_offsets_single");
    std::fs::create_dir(&src).unwrap();
    std::fs::create_dir(&dst).unwrap();
    gen_test_shards(&src_gen, &src)
        .expect("ensure to have a folder named result_1 containing 0.txt.gz as test shard.");
    let lid_path = PathBuf::from("lid.176.bin");

    let p = OscarMetadata::new(src.clone(), dst.clone(), lid_path, 500_000_000);
    p.run().unwrap();

    for lang in LANG.iter() {
        // generate lang file paths
        let mut langfile = dst.clone();
        let mut metafile = dst.clone();
        langfile.push(format!("{}.txt", lang));
        metafile.push(format!("{}_meta.json", lang));

        // open sentence/metadata files
        let mut langfile = File::open(langfile).unwrap();
        let metafile = File::open(metafile).unwrap();

        // get number of sentences from corpus
        let mut sentences = String::new();
        langfile.read_to_string(&mut sentences).unwrap();
        let nb_sentences_corpus = sentences.lines().count();

        println!("{}: {} sentences", lang, nb_sentences_corpus);
        let metadata: Vec<Metadata> = serde_json::from_reader(metafile).unwrap();
        let mut nb_sentences_metadata = metadata.iter().fold(0, |acc, x| {
            assert_eq!(acc, x.offset);
            acc + x.nb_sentences + 1 // account for newline
        });

        println!(
            "{}: C:{} M:{}",
            lang, nb_sentences_corpus, nb_sentences_metadata
        );
        assert_eq!(nb_sentences_corpus, nb_sentences_metadata);
    }
    std::fs::remove_dir_all(&src);
    std::fs::remove_dir_all(&dst);
}

#[test]
#[serial]
#[ignore]
fn assert_meta_validity() {
    // generate test shards
    // and run pipeline on them
    let mut src_gen = PathBuf::from("result_1");
    let src = PathBuf::from("src_intg_meta_validity_single");
    let dst = PathBuf::from("dst_intg_meta_validity_single");
    std::fs::create_dir(&src).unwrap();
    std::fs::create_dir(&dst).unwrap();
    gen_test_shards(&src_gen, &src)
        .expect("ensure to have a folder named result_1 containing 0.txt.gz as test shard.");
    let lid_path = PathBuf::from("lid.176.bin");

    let p = OscarMetadata::new(src.clone(), dst.clone(), lid_path, 500_000_000);
    p.run().unwrap();

    // get data and metadata from shard
    let mut source = src.clone();
    source.push("0.txt.gz");
    let shard = wet::Wet::from_path_gzip(&source).unwrap();

    // unwrap and ignore errors
    let shard_records: Vec<RawRecord> = shard
        .filter_map(|x| {
            if x.is_ok() {
                x.ok()
            } else {
                println!("{:?}", x);
                None
            }
        })
        .collect();

    let shard_metadata: Vec<Metadata> = shard_records
        .iter()
        .map(|record| Metadata::try_from(record.headers.clone()).unwrap())
        .collect();

    for lang in LANG.iter() {
        // generate lang file paths
        let mut langfile = dst.clone();
        let mut metafile = dst.clone();
        langfile.push(format!("{}.txt", lang));
        metafile.push(format!("{}_meta.json", lang));

        // open sentence/metadata files
        let mut langfile = File::open(langfile).unwrap();
        let metafile = File::open(metafile).unwrap();

        // read sentences
        let mut sentences = String::new();
        langfile.read_to_string(&mut sentences).unwrap();

        // put sentences and metadata into vectors
        let sentences: Vec<&str> = sentences.lines().collect();
        let metadata: Vec<Metadata> = serde_json::from_reader(metafile).unwrap();

        for meta in metadata {
            let meta_match = shard_metadata
                .iter()
                .enumerate()
                // find matching record in shard
                .find(|(_, x)| {
                    x.headers.get(&WarcHeader::RecordID).unwrap()
                        == meta.headers.get(&WarcHeader::RecordID).unwrap()
                })
                // ensure that is has only one language
                .and_then(|(idx, x)| {
                    if !x
                        .headers
                        .get(&WarcHeader::Unknown(
                            "warc-identified-content-language".to_string(),
                        ))
                        // silently fail condition
                        // not ideal
                        .unwrap_or(&",".to_string())
                        .contains(",")
                    {
                        return Some((idx, x));
                    }
                    None
                });

            // if there's a match
            if let Some(m) = meta_match {
                // take nb_sentences sentences from offset
                let corpus_lines: HashSet<&str> = sentences
                    .iter()
                    .skip(meta.offset)
                    .take(meta.nb_sentences)
                    // deref to &str
                    .map(|x| *x)
                    .collect();

                // get lines from shard
                // in a vec
                let shard_string = String::from_utf8_lossy(&shard_records[m.0].body);
                let shard_lines: HashSet<&str> = shard_string.lines().collect();

                // ensure that corpus is into shard
                assert!(corpus_lines.is_subset(&shard_lines));
            }
        }
    }
    std::fs::remove_dir_all(&src).expect(&format!("could not delete test src folder: {:?}", &dst));
    std::fs::remove_dir_all(&dst).expect(&format!("could not delete test dst folder: {:?}", &dst));
}

#[test]
#[serial]
#[ignore]
fn assert_meta_final_offset_multishard() {
    // generate test shards
    // and run pipeline on them
    let mut src_gen = PathBuf::from("result_5");
    let src = PathBuf::from("src_intg_final_offset_multi");
    let dst = PathBuf::from("dst_intg_final_offset_multi");
    std::fs::create_dir(&src).unwrap();
    std::fs::create_dir(&dst).unwrap();
    gen_test_shards(&src_gen, &src)
        .expect("ensure to have a folder named result_1 containing 0.txt.gz as test shard.");
    let lid_path = PathBuf::from("lid.176.bin");
    let p = OscarMetadata::new(src.clone(), dst.clone(), lid_path, 500_000_000);
    p.run().unwrap();

    for lang in LANG.iter() {
        // generate lang file paths
        let mut langfile = dst.clone();
        let mut metafile = dst.clone();
        langfile.push(format!("{}.txt", lang));
        metafile.push(format!("{}_meta.json", lang));

        // open sentence/metadata files
        let mut langfile = File::open(langfile).unwrap();
        let metafile = File::open(metafile).unwrap();

        // get number of sentences from corpus
        let mut sentences = String::new();
        langfile.read_to_string(&mut sentences).unwrap();
        let lines = sentences.lines();
        let nb_sentences_corpus = lines.count();

        let mut metadata: Vec<Metadata> = serde_json::from_reader(metafile).unwrap();

        //sort metadata
        //to get the one that has the last offset
        metadata.sort_by(|a, b| a.offset.cmp(&b.offset));

        for m in &metadata {
            println!("{:#?}", m);
            println!("{}+{}", m.offset, m.nb_sentences);
        }
        // get final offset + nb_sentences
        let nb_sentences_metadata = match metadata.last() {
            Some(meta) => meta.offset + meta.nb_sentences + 1,
            None => 0,
        };

        assert_eq!(nb_sentences_corpus, nb_sentences_metadata);
    }

    std::fs::remove_dir_all(&src).unwrap();
    std::fs::remove_dir_all(&dst).unwrap();
}

#[test]
#[serial]
#[ignore]
fn assert_meta_successive_offsets_multishard() {
    // generate test shards
    // and run pipeline on them
    let mut src_gen = PathBuf::from("result_5");
    let src = PathBuf::from("src_intg_successive_offsets_multi");
    let dst = PathBuf::from("dst_intg_successive_offsets_multi");
    std::fs::create_dir(&src).unwrap();
    std::fs::create_dir(&dst).unwrap();
    gen_test_shards(&src_gen, &src)
        .expect("ensure to have a folder named result_1 containing 0.txt.gz as test shard.");
    let lid_path = PathBuf::from("lid.176.bin");
    let p = OscarMetadata::new(src.clone(), dst.clone(), lid_path, 500_000_000);
    p.run().unwrap();

    for lang in LANG.iter() {
        // generate lang file paths
        let mut langfile = dst.clone();
        let mut metafile = dst.clone();
        langfile.push(format!("{}.txt", lang));
        metafile.push(format!("{}_meta.json", lang));

        // open sentence/metadata files
        let mut langfile = File::open(langfile).unwrap();
        let metafile = File::open(metafile).unwrap();

        // get number of sentences from corpus
        let mut sentences = String::new();
        langfile.read_to_string(&mut sentences).unwrap();
        let nb_sentences_corpus = sentences.lines().count();

        let mut metadata: Vec<Metadata> = serde_json::from_reader(metafile).unwrap();
        metadata.sort_by(|a, b| a.offset.cmp(&b.offset));
        let nb_sentences_metadata = metadata.iter().fold(0, |acc, x| {
            assert_eq!(acc, x.offset, "failed at lang {}", lang);
            acc + x.nb_sentences + 1
        });

        println!(
            "{}: C:{} M:{}",
            lang, nb_sentences_corpus, nb_sentences_metadata
        );
        assert_eq!(nb_sentences_corpus, nb_sentences_metadata);
    }
    std::fs::remove_dir_all(&src);
    std::fs::remove_dir_all(&dst);
}

#[test]
#[serial]
#[ignore]
fn assert_meta_validity_multishard() {
    // gen test shards and run pipeline
    let mut src_gen = PathBuf::from("result_5");
    let src = PathBuf::from("src_intg_meta_validity_multi");
    let dst = PathBuf::from("dst_intg_meta_validity_multi");
    std::fs::create_dir(&src);
    std::fs::create_dir(&dst);
    gen_test_shards(&src_gen, &src)
        .expect("ensure to have a folder named result_5 containing 0.txt.gz as test shard.");
    let lid_path = PathBuf::from("lid.176.bin");
    let p = OscarMetadata::new(src.clone(), dst.clone(), lid_path, 500_000_000);
    p.run().unwrap();

    let mut record_index = HashMap::new();
    //Read all 5 shards
    for shard_idx in 0..5 {
        println!("processing shard {}", shard_idx);
        let mut shard_path = src.clone();
        shard_path.push(format!("{}.txt.gz", shard_idx));
        let shard = wet::Wet::from_path_gzip(&shard_path).unwrap();

        let records = shard.filter_map(|record| match record {
            Ok(record) => {
                // get record_id and body
                // parse to strings
                let record_id = record.headers.get(&WarcHeader::RecordID).unwrap().clone();
                let body = record.body.clone();
                let record_id = String::from_utf8_lossy(&record_id).to_string();

                //transform body into a vector of sentences
                let body = String::from_utf8_lossy(&body)
                    .lines()
                    .map(|line| line.to_string())
                    .collect::<Vec<String>>();

                Some((record_id, body))
            }
            Err(e) => {
                println!("{:?}", e);
                return None;
            }
        });

        record_index.extend(records);
    }

    for lang in LANG.iter() {
        // get from corpus
        let (sentences, metadata) = get_lang_data(lang, &dst);
        for meta in metadata {
            let corpus_body: Vec<&String> = sentences
                .iter()
                .skip(meta.offset)
                .take(meta.nb_sentences)
                .collect();

            let record_id = meta.headers.get(&WarcHeader::RecordID).unwrap();
            let shard_body = record_index.get(record_id).unwrap();
            let shard_body_hs: HashSet<&String> = shard_body.iter().collect();
            for sentence in shard_body {
                assert!(shard_body_hs.contains(sentence));
            }
        }
    }

    std::fs::remove_dir_all(&src);
    std::fs::remove_dir_all(&dst);
}
#[test]
#[ignore]
fn pipeline_single_shard() {
    let src = PathBuf::from("debug_1/");
    let dst = PathBuf::from("temp_1/");

    let lid_path = PathBuf::from("lid.176.bin");
    let p = OscarMetadata::new(src.clone(), dst.clone(), lid_path, 500_000_000);
    let res = p.run();
    assert!(res.is_ok());

    std::fs::remove_dir_all(dst);
}
