//! PEF definitions.
//!
//! This module is based heavily on "Mac OS Runtime Architectures for System 7 Through Mac OS 9" January 31, 1997

#![allow(missing_docs)]

use crate::endian::{ BigEndian as BE, I32, U16, U32};
use crate::pod::Pod;

/// Joy!
pub const TAG1 : u32 = 0x4A6F_7921;
/// peff
pub const TAG2 : u32= 0x7065_6666;

/// pwpc
pub const ARCHITECTURE_PPC : u32 = 0x7077_7063;
/// m68k
pub const ARCHITECTURE_68K : u32 = 0x6D36_386B;

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct PEFContainerHeader
{
    /// Designates that the container uses an Apple-defined format. This field must be set to "Joy!" in ASCII
    pub tag1 : U32<BE>, 
    /// Identifies the type of container (currently set to peff in ASCII")
    pub tag2 : U32<BE>,
    /// Indicates the architecture type that the container was generated for. This field holds the ASCII value pwpc for the PowerPC CFM implementation or m68k for CFM-68K.
    pub architecture : U32<BE>,
    /// Indicates the version of PEF used in the container. The current version is 1.
    pub format_version : U32<BE>,
    /// Indicates when the PEF container was created. The stamp follows the Macintosh time-measurement scheme (that is, the number of seconds measured from January 1, 1904)
    pub date_time_stamp : U32<BE>,
    /// Contain version information that the Code Fragment Manager uses to check shared library compatibility
    pub old_def_version : U32<BE>,
    /// Contain version information that the Code Fragment Manager uses to check shared library compatibility
    pub old_imp_version : U32<BE>,
    /// Contain version information that the Code Fragment Manager uses to check shared library compatibility
    pub current_version : U32<BE>,
    /// Indicates the total number of sections contained in the container
    pub section_count : U16<BE>,
    /// Indicates the number of instantiated sections. Instantiated sections contain code or data that are required for execution
    pub inst_section_count : U16<BE>,
    /// Reserved for future use
    pub reserved_a : U32<BE>,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectionKind 
{
    /// Contains read-only executable code in an uncompressed binary format.
    /// A container can have any number of code sections. Code sections are always shared.
    Code = 0,
    /// Contains uncompressed, initialized, read/write data followed by zero-initialized read/write data
    UnpackedData = 1,
    /// Contains read/write data initialized by a pattern specification contained in the section’s contents. 
    /// The contents essentially contain a small program that tells the Code Fragment Manager how to initialize the raw data in memory.
    PatternInitializedData = 2,
    /// Contains uncompressed, initialized, read-only data. 
    /// A container can have any number of constant sections, and they are implicitly shared.
    Constant = 3,
    /// Contains information about imports, exports, and entry points. 
    /// A container can have only one loader section.
    Loader = 4,
    /// Reserved for future use
    Debug = 5,
    /// Contains information that is both executable and modifiable. 
    /// For example, this section can store code that contains embedded data. 
    /// A container can have any number of executable data sections, each with a different sharing option.
    ExecutableData = 6,
    /// Reserved for future use
    Exception = 7,
    /// Reserved for future use
    Traceback = 8,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShareKind 
{
    /// Indicates that the section is shared within a process, but a fresh copy is created for different processes.
    ProcessShare = 1, 
    /// Indicates that the section is shared between all processes in the system.
    GlobalShare = 4, 
    /// Indicates that the section is shared between all processes, but is protected. 
    /// Protected sections are read/write in privileged mode and read-only in user mode. This option is not available in System 7.
    ProtectedShare = 5,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct PEFSectionHeader
{
    /// Holds the offset from the start of the section name table to the location of the section name. 
    /// The name of the section is stored as a C-style null-terminated character string.
    pub name_offset : I32<BE>,
    /// Indicates the preferred address (as designated by the linker) at which to place the section’s instance. 
    /// If the Code Fragment Manager can place the instance in the preferred memory location, 
    /// the load-time and link-time addresses are identical and no internal relocations need to be performed.
    pub default_address : U32<BE>,
    /// Indicates the size, in bytes, required by the section’s contents at execution time. 
    /// For a code section, this size is merely the size of the executable code. 
    /// For a data section, this size indicates the sum of the size of the initialized data plus the size of any zero-initialized data. 
    /// Zero-initialized data appears at the end of a section’s contents and its length is exactly
    /// the difference of the totalSize and unpackedSize values. For noninstantiated sections, this field is ignored.
    pub total_size : U32<BE>,
    /// Indicates the size of the section’s contents that is explicitly initialized from the container. 
    /// For code sections, this field is the size of the executable code. 
    /// For an unpacked data section, this field indicates only the size of the initialized data. 
    /// For packed data this is the size to which the compressed contents expand. 
    /// The unpackedSize value also defines the boundary between the explicitly initialized portion and the zero-initialized portion.
    pub unpacked_size : U32<BE>,
    /// Indicates the size, in bytes, of a section’s contents in the container. 
    /// For code sections, this field is the size of the executable code. 
    /// For an unpacked data section, this field indicates only the size of the initialized data. 
    /// For a packed data section this field is the size of the pattern description contained in the section.
    pub packed_size : U32<BE>,
    /// Contains the offset from the beginning of the container to the start of the section’s contents. 
    /// Packed data sections and the loader section should be 4-byte aligned.
    /// Code sections and data sections that are not packed should be at least 16-byte aligned.
    pub container_offset : U32<BE>,
    /// Indicates the type of section as well as any special attributes. 
    /// Note that instantiated read-only sections cannot have zero-initialized extensions.
    pub section_kind : SectionKind, 
    /// Controls how the section information is shared among processes by the Code Fragment Manager.
    pub share_kind : ShareKind,
    /// Indicates the desired alignment for instantiated sections in memory as a power of 2. 
    /// A value of 0 indicates 1-byte alignment, 1 indicates 2-byte (halfword) alignment, 2 indicates 4-byte (word) alignment, and so on. 
    /// Note that this field does not indicate the alignment of raw data relative to a container. 
    /// The Code Fragment Manager does not support this field under System 7.
    pub alignment : u8, 
    /// Reserved for future use
    pub reserved_a : u8,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct PEFLoaderInfoHeader 
{
    /// Specifies the number of the section in this container that contains the main symbol. 
    /// If the fragment does not have a main symbol, this field is set to -1.
    pub main_section : I32<BE>,
    /// Indicates the offset (in bytes) from the beginning of the section to the main symbol.
    pub main_offset : U32<BE>,
    /// Contains the number of the section containing the initialization function’s transition vector. 
    /// If no initialization function exists, this field is set to -1.
    pub init_section : I32<BE>,
    /// Indicates the offset (in bytes) from the beginning of the section to the initialization function’s transition vector.
    pub init_offset : U32<BE>,
    /// Contains the number of the section containing the termination routine’s transition vector. 
    /// If no termination routine exists, this field is set to -1.
    pub term_section : I32<BE>,
    /// Indicates the offset (in bytes) from the beginning of the section to the termination routine’s transition vector.
    pub term_offset : U32<BE>,
    /// The importedLibraryCount field (4 bytes) indicates the number of imported libraries.
    pub imported_library_count : U32<BE>,
    /// Indicates the total number of imported symbols.
    pub total_imported_symbol_count : U32<BE>,
    /// Indicates the number of sections containing load-time relocations.
    pub reloc_section_count : U32<BE>,
    /// Indicates the offset (in bytes) from the beginning of the loader section to the start of the relocations area.
    pub reloc_instr_offset : U32<BE>,
    /// Indicates the offset (in bytes) from the beginning of the loader section to the start of the loader string table.
    pub loader_strings_offset : U32<BE>,
    /// Indicates the offset (in bytes) from the beginning of the loader section to the start of the export hash table. 
    /// The hash table should be 4-byte aligned with padding added if necessary.
    pub export_hash_offset : U32<BE>,
    /// Indicates the number of hash index values (that is, the number of entries in the hash table). 
    /// The number of entries is specified as a power of two. 
    /// For example, a value of 0 indicates one entry, while a value of 2 indicates four entries. 
    /// If no exports exist, the hash table still contains one entry, and the value of this field is 0.
    pub export_hash_table_power : U32<BE>,
    /// Indicates the number of symbols exported from this container.
    pub exported_symbol_count : U32<BE>,
}

unsafe_impl_pod!(
    PEFContainerHeader,
    PEFSectionHeader
);