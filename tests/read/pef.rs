use object::Object;
use object::ObjectSection;
use chrono::Datelike;

#[cfg(feature = "pef")]
#[test]
fn pef_test_section() {
    let pef_testfiles = std::path::Path::new("testfiles/pef");

    let file_name = "test1";

    let path = pef_testfiles.join(file_name);
    let file = std::fs::File::open(&path).expect(format!("Could not open {:?}", &path).as_str());
    let reader = object::read::ReadCache::new(file);
    let object = object::read::File::parse(&reader);
    assert!(object.is_ok());
    let object = object.unwrap();

    assert!(object.sections().count() == 3);

    let sect_0 = object.section_by_index(object::SectionIndex(1));
    assert!(sect_0.is_ok());
}

#[cfg(feature = "pef")]
#[test]
fn pef_test_section_by_name() {
    use object::ObjectSection;

    let pef_testfiles = std::path::Path::new("testfiles/pef");

    let file_name = "Microsoft_Component_Library";

    let path = pef_testfiles.join(file_name);
    let file = std::fs::File::open(&path).expect(format!("Could not open {:?}", &path).as_str());
    let reader = object::read::ReadCache::new(file);
    let object = object::read::File::parse(&reader);
    assert!(object.is_ok());
    let object = object.unwrap();

    let sect_text = object.section_by_name(".text");
    assert!(sect_text.is_some());
    let file_range = sect_text.unwrap().file_range();
    assert!(file_range.is_some());
    assert_eq!(file_range.unwrap().0, 144);
    assert_eq!(file_range.unwrap().1, 64);

    let sect_text = object.section_by_name(".data");
    assert!(sect_text.is_some());

    let sect_text = object.section_by_name(".ppcldr");
    assert!(sect_text.is_some());
}

#[cfg(feature = "pef")]
#[test]
fn pef_test_timestamp() {
    let pef_testfiles = std::path::Path::new("testfiles/pef");

    let file_name = "test1";

    let path = pef_testfiles.join(file_name);
    let file = std::fs::File::open(&path).expect(format!("Could not open {:?}", &path).as_str());
    let reader = object::read::ReadCache::new(file);
    
    let pef_file = object::read::pef::PefFile::parse(&reader);
    assert!(pef_file.is_ok());
    let pef_file = pef_file.unwrap();

    let date = pef_file.timestamp();
    assert!(date.is_some());
    assert_eq!(date.unwrap().year(), 2024);
    assert_eq!(date.unwrap().month(), 10);
    assert_eq!(date.unwrap().day(), 27);   
}

#[cfg(feature = "pef")]
#[test]
fn pef_test_pattern_initialized_data() {
    use std::io::Read;

    let pef_testfiles = std::path::Path::new("testfiles/pef");

    let file_name = "test1";

    let path = pef_testfiles.join(file_name);
    let file = std::fs::File::open(&path).expect(format!("Could not open {:?}", &path).as_str());
    let reader = object::read::ReadCache::new(file);
    
    let pef_file = object::read::pef::PefFile::parse(&reader);
    assert!(pef_file.is_ok());
    let pef_file = pef_file.unwrap();

    let unpacked_pattern_seg_data_fname = "test1_pattern_unpacked_data.bin";
    let pattern_data_path = pef_testfiles.join(unpacked_pattern_seg_data_fname);
    let mut pattern_data_file = std::fs::File::open(&pattern_data_path)
        .expect(format!("Could not open pattern initialized data to compare against {:?}", &pattern_data_path).as_str());

    let mut read_data_buf = Vec::new();
    pattern_data_file
        .read_to_end(&mut read_data_buf)
        .expect(format!("Could not read pattern initialized data to compare against {:?}", &pattern_data_path).as_str());

    let uncompressed_data = pef_file
        .section_by_index(object::SectionIndex(2))
        .expect("Cannot get pattern initialized data section")
        .uncompressed_data()
        .expect("Decompression failed");

    assert!(uncompressed_data == read_data_buf);
}