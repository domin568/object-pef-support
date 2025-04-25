//use object::{pe, read, Object, ObjectSection};
use std::fs;
use std::path::PathBuf;

#[cfg(feature = "pef")]
#[test]
fn pef_test() {
    let path_to_obj: PathBuf = ["testfiles", "pef", "test1"].iter().collect();
    let contents = fs::read(path_to_obj).expect("Could not read test1");
    assert!(contents.len() > 0);
}