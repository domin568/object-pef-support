use object::Object;

#[cfg(feature = "pef")]
#[test]
fn pef_test_sections() {
    let pef_testfiles = std::path::Path::new("testfiles/pef");

    let file_name = "test1";

    let path = pef_testfiles.join(file_name);
    let file = std::fs::File::open(&path).expect(format!("Could not open {:?}", &path).as_str());
    let reader = object::read::ReadCache::new(file);
    let object = object::read::File::parse(&reader);
    assert!(object.is_ok());
    let object = object.unwrap();

    let sect_0 = object.section_by_index(object::SectionIndex(1));
    assert!(sect_0.is_ok());
}