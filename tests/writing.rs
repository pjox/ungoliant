use std::collections::HashMap;
use std::path::Path;

use rayon::iter::{IntoParallelIterator, ParallelIterator};
use ungoliant::pipeline::oscar_metadata::document::MergedPiece;
use ungoliant::writing::LangFiles;
use warc::header::WarcHeader;

fn english_sentences(nb: i32) -> Vec<String> {
    (0..nb)
        .into_iter()
        .map(|x| format!("english sentence number {}", x + 1).to_string())
        .collect()
}
fn french_sentences(nb: i32) -> Vec<String> {
    (0..nb)
        .into_iter()
        .map(|x| format!("phrase française numéro {}", x + 1).to_string())
        .collect()
}

fn english_mergedparts(nb: i32) -> Vec<MergedPiece> {
    (0..nb)
        .into_iter()
        .map(|x| {
            let sentences = english_sentences(x + 1);
            let headers = vec![(
                WarcHeader::ContentType,
                Vec::from(format!("blogpost{}", x + 1).as_bytes()),
            )]
            .into_iter()
            .collect();
            let identification = "en";
            MergedPiece::new(headers, sentences, identification)
        })
        .collect()
}

fn french_mergedparts(nb: i32) -> Vec<MergedPiece> {
    (0..nb)
        .into_iter()
        .map(|x| {
            let sentences = french_sentences(x + 1);
            let headers = vec![(
                WarcHeader::ContentType,
                Vec::from(format!("article francais {}", x + 1).as_bytes()),
            )]
            .into_iter()
            .collect();
            let identification = "fr";
            MergedPiece::new(headers, sentences, identification)
        })
        .collect()
}
#[test]
fn single_lang() {
    let dst = Path::new("intg_single_lang_monothread");
    std::fs::create_dir(dst).unwrap();
    let langfiles = LangFiles::new(dst, 1000).unwrap();

    let parts = english_mergedparts(10).into_par_iter();
    println!("{:#?}", parts);
    parts.for_each(|part| {
        let part_lang = part.identification;
        let mut langfile = langfiles.writers().get(part_lang).unwrap().lock().unwrap();
        langfile.write(vec![part]).unwrap();
    });
    std::fs::remove_dir_all(dst).unwrap();
}

#[test]
fn multiple_langs() {
    let dst = Path::new("intg_multiple_langs");
    std::fs::create_dir(dst).unwrap();
    let langfiles = LangFiles::new(dst, 1000).unwrap();

    // assume they are shuffled
    let mut parts = english_mergedparts(10);
    parts.append(&mut french_mergedparts(10));

    let mut parts_by_lang = HashMap::new();

    for part in parts {
        let e = parts_by_lang
            .entry(part.identification())
            .or_insert_with(Vec::new);
        e.push(part)
    }

    println!("{:#?}", parts_by_lang);
    let parts_by_lang = parts_by_lang.into_par_iter();
    parts_by_lang.for_each(|(lang, lparts)| {
        let mut langfile = langfiles.writers().get(lang).unwrap().lock().unwrap();
        langfile.write(lparts).unwrap();
    });

    langfiles.close_meta().unwrap();
    std::fs::remove_dir_all(dst).unwrap();
}
