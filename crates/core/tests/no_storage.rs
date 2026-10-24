//! D17, changed for this game (BUILDER-SPEC.md change table, SECOND-ORDER
//! R3.6): the page keeps one thing between visits, the core's save of trial
//! bests, achievements and settings, and only `web/store.js` may touch
//! browser storage, and only `localStorage`. Every other file under web/
//! (vendor/ aside) still fails if it names a storage API. After Galactic
//! Survivors. The core reads the save and treats an unreadable one as new
//! (crates/core/src/save.rs).

use std::fs;
use std::path::{Path, PathBuf};

const STORAGE_NAMES: &[&str] = &[
    "localStorage",
    "sessionStorage",
    "indexedDB",
    "IDBFactory",
    "document.cookie",
    "caches",
    "CacheStorage",
    "serviceWorker",
    "navigator.storage",
    "openDatabase",
];

fn storage_names_in(text: &str) -> Vec<&'static str> {
    STORAGE_NAMES
        .iter()
        .copied()
        .filter(|name| text.contains(name))
        .collect()
}

fn files_under(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(read) = fs::read_dir(&d) else {
            continue;
        };
        for entry in read {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                if path.file_name().map(|n| n != "vendor").unwrap_or(true) {
                    stack.push(path);
                }
            } else {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

#[test]
fn the_page_names_no_browser_storage() {
    let web = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../web");
    let mut found = Vec::new();
    for path in files_under(&web) {
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let store = path.ends_with("web/store.js");
        for name in storage_names_in(&text) {
            if store && name == "localStorage" {
                continue;
            }
            found.push(format!("{}: {name}", path.display()));
        }
    }
    assert!(
        found.is_empty(),
        "browser storage outside web/store.js, or other than localStorage there (D17 as changed):\n{}",
        found.join("\n")
    );
}

#[test]
fn the_scanner_sees_storage() {
    assert_eq!(
        storage_names_in("window.localStorage.setItem('n', name)"),
        vec!["localStorage"]
    );
    assert_eq!(storage_names_in("indexedDB.open('x')"), vec!["indexedDB"]);
    assert!(storage_names_in("el(\"note\").value").is_empty());
}

#[test]
fn the_store_is_the_one_file_that_names_storage() {
    let web = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../web");
    let store = fs::read_to_string(web.join("store.js")).expect("web/store.js");
    assert_eq!(storage_names_in(&store), vec!["localStorage"]);
}
