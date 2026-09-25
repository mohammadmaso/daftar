//! §12 performance targets on a generated 5,000-page library: wiki search under 50 ms.
//! Timings are asserted strictly in release builds (`cargo test --release --test perf`); debug
//! builds use 1,500 pages and a looser bound so the regular test run stays quick and not flaky.

use std::time::{Duration, Instant};

use daftar_core::fsutil::atomic_write;
use daftar_core::search::SearchIndex;
use daftar_core::testutil::lib_in;

const PEOPLE: &[(&str, &str)] = &[
    ("Sara", "سارا"),
    ("Ali", "علی"),
    ("Maryam", "مریم"),
    ("Reza", "رضا"),
];
const TOPICS: &[(&str, &str)] = &[
    ("sleep", "خواب"),
    ("vitamin D", "ویتامین دی"),
    ("coffee", "قهوه"),
    ("career", "کار"),
    ("headache", "سردرد"),
];

#[test]
fn search_at_scale_is_fast() {
    let (_d, lib) = lib_in();
    let n = if cfg!(debug_assertions) { 1500 } else { 5000 };
    for i in 0..n {
        let (pe, pf) = PEOPLE[i % PEOPLE.len()];
        let (te, tf) = TOPICS[i % TOPICS.len()];
        let vault = ["life", "health", "mind", "work"][i % 4];
        let doc = format!(
            "---\ntype: topic\ntitle: {{ en: \"{te} note {i}\", fa: \"یادداشت {tf} {i}\" }}\naliases: [\"{tf} {i}\"]\nsummary: \"{pe} and {te}\"\n---\n\n\
             ## ۱ مهر\n\n{pf} درباره‌ی {tf} صحبت کرد. Talked with [[{}|{pe}]] about {te} on day {i}.\n\n\
             - Sleep was poor after late coffee (status:: proposed) ^c-{i}\n",
            pe.to_lowercase()
        );
        atomic_write(
            &lib.path(&format!("vaults/{vault}/topics/note-{i:04}.md")),
            doc.as_bytes(),
        )
        .unwrap();
    }
    let t = Instant::now();
    let mut idx = SearchIndex::open(&lib).unwrap();
    assert_eq!(idx.refresh(&lib).unwrap(), n);
    eprintln!("indexed {n} pages in {:?}", t.elapsed());
    let t = Instant::now();
    assert_eq!(idx.refresh(&lib).unwrap(), 0);
    eprintln!("no-op refresh in {:?}", t.elapsed());

    let queries = [
        "سردرد",
        "coffee",
        "ويتامين",
        "سارا قهوه",
        "career note 42",
        "خوا",
        "یادداشت کار",
        "maryam sleep",
    ];
    let mut worst = Duration::ZERO;
    for q in queries {
        let t = Instant::now();
        let hits = idx.search(q, &[], &[], 20).unwrap();
        let el = t.elapsed();
        assert!(!hits.is_empty(), "no hits for {q}");
        worst = worst.max(el);
        eprintln!("{q:>16}: {:>3} hits in {el:?}", hits.len());
    }
    let t = Instant::now();
    let back = idx.backlinks("vaults/life/topics/note-0000.md").unwrap();
    eprintln!("backlinks in {:?} ({} pages)", t.elapsed(), back.len());
    let bound = if cfg!(debug_assertions) { 250 } else { 50 };
    assert!(
        worst < Duration::from_millis(bound),
        "worst search {worst:?} over {bound} ms"
    );
}
