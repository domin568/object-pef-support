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

#[cfg(feature = "pef")]
#[test]
fn pef_test_imports() {
    use std::str;

    let pef_testfiles = std::path::Path::new("testfiles/pef");

    let file_name = "test1";

    let path = pef_testfiles.join(file_name);
    let file = std::fs::File::open(&path).expect(format!("Could not open {:?}", &path).as_str());
    let reader = object::read::ReadCache::new(file);
    
    let pef_file = object::read::pef::PefFile::parse(&reader);
    assert!(pef_file.is_ok());
    let pef_file = pef_file.unwrap();

    let expected: &[(&[u8], &[u8])] = &[
        (b"InterfaceLib", b"NewPtr"),
        (b"InterfaceLib", b"Gestalt"),
        (b"InterfaceLib", b"ExitToShell"),
        (b"InterfaceLib", b"DisposePtr"),
        (b"InterfaceLib", b"NewPtrClear"),
        (b"MathLib",      b"fabs"),
        (b"MathLib",      b"num2dec"),
    ];

    let imports = pef_file.imports();
    assert!(imports.is_ok());
    let imports = imports.unwrap();
    assert_eq!(
        imports.len(),
        expected.len(),
        "expected {} imports, got {}",
        expected.len(),
        imports.len(),
    );

    for (idx, import) in imports.iter().enumerate() {
        let lib_bytes  = import.library();
        let name_bytes = import.name();

        let lib_str  = str::from_utf8(lib_bytes)
            .expect(&format!("library bytes not UTF-8 at index {}", idx));
        let name_str = str::from_utf8(name_bytes)
            .expect(&format!("name bytes not UTF-8 at index {}", idx));

        let (exp_lib, exp_name) = expected[idx];
        let exp_lib_str  = str::from_utf8(exp_lib)
            .expect("expected library literal is not UTF-8");
        let exp_name_str = str::from_utf8(exp_name)
            .expect("expected name literal is not UTF-8");

        assert_eq!(
            lib_str, exp_lib_str,
            "library mismatch at index {}: expected `{}`, got `{}`",
            idx, exp_lib_str, lib_str
        );
        assert_eq!(
            name_str, exp_name_str,
            "name mismatch at index {}: expected `{}`, got `{}`",
            idx, exp_name_str, name_str
        );
    }
}

#[cfg(feature = "pef")]
#[test]
fn pef_test_exports() {
    use std::str;

    let pef_testfiles = std::path::Path::new("testfiles/pef");

    let file_name = "Internet_Config_Extension";

    let path = pef_testfiles.join(file_name);
    let file = std::fs::File::open(&path).expect(format!("Could not open {:?}", &path).as_str());
    let reader = object::read::ReadCache::new(file);
    
    let pef_file = object::read::pef::PefFile::parse(&reader);
    assert!(pef_file.is_ok());
    let pef_file = pef_file.unwrap();

    let exports = pef_file.exports();
    assert!(exports.is_ok());
    let exports = exports.unwrap();
    
    let expected: &[(&[u8], u64)] = &[
        (b"ICCGetIndMapEntry", 680),
        (b"ICRequiresInterruptSafe", 480),
        (b"ICSetCurrentProfile", 592),
        (b"ICDeletePref", 848),
        (b"ICGetPref", 960),
        (b"ICCStop", 1208),
        (b"ICEditPreferences", 816),
        (b"ICChooseNewConfig", 1120),
        (b"ICMapTypeCreator", 752),
        (b"ICDeleteMapEntry", 640),
        (b"ICCMapTypeCreator", 744),
        (b"ICGetSeed", 1024),
        (b"ICCRefreshCaches", 1032),
        (b"ICCChooseNewConfig", 1112),
        (b"ICGetSeedInterruptSafe", 448),
        (b"ICCChooseConfig", 1128),
        (b"ICGetCurrentProfile", 608),
        (b"ICCSetConfigReference", 1064),
        (b"ICFindConfigFile", 1184),
        (b"ICCSetProfileName", 520),
        (b"ICCStart", 1224),
        (b"ICCGetProfileName", 536),
        (b"ICSetMapEntry", 656),
        (b"ICGetMapEntry", 672),
        (b"ICGetComponentInstance", 1200),
        (b"ICCDeletePref", 840),
        (b"ICStop", 1216),
        (b"ICParseURL", 800),
        (b"ICFindPrefHandle", 928),
        (b"ICCParseURL", 792),
        (b"ICCGetConfigName", 1096),
        (b"ICCMapFilename", 760),
        (b"ICCDefaultFileName", 984),
        (b"ICGetIndMapEntry", 688),
        (b"ICStart", 1232),
        (b"ICCGetIndPref", 856),
        (b"ICGetConfigName", 1104),
        (b"ICSetPref", 944),
        (b"ICCLaunchURL", 776),
        (b"ICGetIndPref", 864),
        (b"ICCGetCurrentProfile", 600),
        (b"ICCGetPerm", 1000),
        (b"ICMapEntriesFilename", 736),
        (b"ICChooseConfig", 1136),
        (b"ICCSetPrefHandle", 888),
        (b"ICCMapEntriesFilename", 728),
        (b"ICCGetPrefHandle", 904),
        (b"ICCSetMapEntry", 648),
        (b"ICCGetMapEntry", 664),
        (b"ICCEnd", 824),
        (b"ICSetConfigReference", 1072),
        (b"ICCFindUserConfigFile", 1160),
        (b"ICGetPrefHandle", 912),
        (b"ICCBegin", 968),
        (b"ICGetConfigReference", 1088),
        (b"ICSetPrefHandle", 896),
        (b"ICCGetPref", 952),
        (b"ICCMapEntriesTypeCreator", 712),
        (b"ICDefaultFileName", 992),
        (b"ICCGetMappingInterruptSafe", 456),
        (b"ICAddMapEntry", 624),
        (b"ICCGetComponentInstance", 1192),
        (b"ICDeleteProfile", 496),
        (b"ICLaunchURL", 784),
        (b"ICCGetSeed", 1016),
        (b"ICAddProfile", 512),
        (b"ICCGeneralFindConfigFile", 1144),
        (b"ICCCountProfiles", 568),
        (b"ICFindUserConfigFile", 1168),
        (b"ICCDeleteMapEntry", 632),
        (b"ICMapFilename", 768),
        (b"ICGetIndProfile", 560),
        (b"ICCCountMapEntries", 696),
        (b"ICCCountPref", 872),
        (b"ICSpecifyConfigFile", 1056),
        (b"ICCAddProfile", 504),
        (b"ICMapEntriesTypeCreator", 720),
        (b"ICCountPref", 880),
        (b"ICCFindPrefHandle", 920),
        (b"ICGetProfileName", 544),
        (b"ICCountProfiles", 576),
        (b"ICSetProfileName", 528),
        (b"ICCSetCurrentProfile", 584),
        (b"ICCountMapEntries", 704),
        (b"ICCSetPref", 936),
        (b"ICGetMappingInterruptSafe", 464),
        (b"ICCDeleteProfile", 488),
        (b"ICCAddMapEntry", 616),
        (b"ICCSpecifyConfigFile", 1048),
        (b"ICCGetSeedInterruptSafe", 440),
        (b"ICCRequiresInterruptSafe", 472),
        (b"ICCGetIndProfile", 552),
        (b"ICCEditPreferences", 808),
        (b"ICEnd", 832),
        (b"ICBegin", 976),
        (b"ICGetPerm", 1008),
        (b"ICRefreshCaches", 1040),
        (b"ICCGetConfigReference", 1080),
        (b"ICGeneralFindConfigFile", 1152),
        (b"ICCFindConfigFile", 1176),
    ];

    assert_eq!(
        exports.len(),
        expected.len(),
        "expected {} exports, got {}",
        expected.len(),
        exports.len(),
    );

    for (idx, export) in exports.iter().enumerate() {
        let name  = export.name();
        let offset = export.address();

        let name_str = str::from_utf8(name)
            .expect(&format!("name bytes not UTF-8 at index {}", idx));

        let (exp_name, exp_offset) = expected[idx];
        let exp_name_str  = str::from_utf8(exp_name)
            .expect("expected library literal is not UTF-8");

        assert_eq!(
            name_str, exp_name_str,
            "name mismatch at index {}: expected `{}`, got `{}`",
            idx, exp_name_str, name_str
        );
        assert_eq!(
            offset, exp_offset,
            "offset mismatch at index {}: expected `{}`, got `{}`",
            idx, exp_offset, offset
        );
    }
}