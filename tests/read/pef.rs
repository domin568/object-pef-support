use object::{Object, ObjectSection};
use std::collections::HashMap;

#[cfg(feature = "pef")]
#[test]
fn pef_test_sections() {
    let pef_testfiles = std::path::Path::new("testfiles/pef");

    let files_to_sections :HashMap<&str, Vec<&str>> = HashMap::from([
        ("test1", vec![""]),
        ("test2", vec![""]),
    ]);

    for (file_name, actual_section_names) in files_to_sections {
        let path = pef_testfiles.join(file_name);
        let file = std::fs::File::open(&path).expect(format!("Could not open {:?}", &path).as_str());
        let reader = object::read::ReadCache::new(file);
        let object = object::read::File::parse(&reader).expect(format!("Could not parse {:?}", &path).as_str());

        for actual_section_name in actual_section_names {
            let section = object.section_by_name(actual_section_name)
            .expect(format!("Could not get section {} for file{:?}", actual_section_name, path).as_str());
            assert_eq!(section.name(), Ok(actual_section_name));
        }
    }
}