use core::fmt::Debug;
use crate::pef;
use crate::endian::{ BigEndian as BE, I32, U16, U32};
use core::{slice, str, mem};
use alloc::vec::Vec;
use crate::read::{
    self, Architecture, ComdatKind, Error, Export, FileFlags,
    Import, NoDynamicRelocationIterator, Object, ObjectComdat, ObjectKind, ObjectSection,
    ObjectSymbol, ObjectSymbolTable, ReadError, ReadRef,
    Result, SectionIndex, SymbolFlags, SymbolIndex,
    SymbolKind, SymbolScope, SymbolSection, StringTable
};
use chrono::{DateTime, Utc};

use super::{
    PefSection, PefSectionIterator,
    PefSegment, PefSegmentIterator, SectionTable
};

/// A PEF image file.
///
/// Most functionality is provided by the [`Object`] trait implementation.
#[derive(Debug)]
pub struct PefFile<'data, R = &'data [u8]>
where
    R: ReadRef<'data>
{
    pub(super) header: &'data pef::PEFContainerHeader,
    pub(super) sections: SectionTable<'data, R>,
    pub(super) data: R,
    pub(super) loader: PefLoaderParser<'data, R>
}

impl<'data, R> PefFile<'data, R>
where
    R: ReadRef<'data>,
{
    /// Parse the raw PEF file data.
    pub fn parse(data: R) -> Result<Self> {
        let header = pef::PEFContainerHeader::parse(data)?;
        let sections = header.sections(data)?;
        let loader_section = sections
            .iter()
            .find(|&sect| sect.section_kind == pef::SectionKind::Loader).expect("Missing loader section");

        let loader = PefLoaderParser::parse(data, loader_section.container_offset.get(BE)).expect("Could not parse PEF loader section");
        
        Ok(PefFile {
            header,
            sections,
            data,
            loader,
        })
    }
}

impl pef::PEFContainerHeader {
    /// Read the PEF container header.
    ///
    /// Also checks that the `tag1` field in the header is valid.
    pub fn parse<'data, R: ReadRef<'data>>(data: R) -> read::Result<&'data Self> {
        let container_header = data
            .read_at::<pef::PEFContainerHeader>(0)
            .read_error("Invalid PEFContainerHeader header size or alignment")?;
        if container_header.tag1.get(BE) != pef::TAG1 {
            return Err(Error("Invalid PEF magic"));
        }
        Ok(container_header)
    }

        /// Return the slice of section headers.
    ///
    /// Returns `Ok(&[])` if there are no section headers.
    /// Returns `Err` for invalid values.
    fn section_headers<'data, R: ReadRef<'data>>(
        &self,
        data: R,
    ) -> read::Result<&'data [pef::PEFSectionHeader]> {
        if self.section_count.get(BE) == 0 {
            return Err(Error("Missing sections"));
        }
        let sections_off = mem::size_of::<pef::PEFContainerHeader>() as u64;
        let section_size = mem::size_of::<pef::PEFSectionHeader>() as u64;
        let section_count = self.section_count.get(BE) as u64;
        if data.len().expect("Cannot get data len") <= sections_off + section_size * section_count {
            return Err(Error("Cut file"));
        }
        data.read_slice_at(sections_off, section_count as usize)
            .read_error("Invalid PEF section header offset/size/alignment")
    }

    fn section_strings<'data, R: ReadRef<'data>>(
        &self,
        data: R,
        sections: &[pef::PEFSectionHeader],
    ) -> read::Result<StringTable<'data, R>> {
        if sections.is_empty() {
            return Ok(StringTable::default());
        };
        let offset = mem::size_of::<pef::PEFContainerHeader>()
                            + (self.section_count.get(BE) as usize * mem::size_of::<pef::PEFSectionHeader>());
        Ok(StringTable::new(data, offset as u64, sections[0].container_offset.get(BE) as u64))
    }

    /// Return the section table.
    fn sections<'data, R: ReadRef<'data>>(
        &self,
        data: R,
    ) -> read::Result<SectionTable<'data, R>> {
        let sections = self.section_headers(data)?;
        let strings = self.section_strings(data, sections)?;
        Ok(SectionTable::new(sections, strings))
    }

}

/// Parser for PEFLoaderInfoHeader
#[derive(Debug)]
pub struct PefLoaderParser<'data, R = &'data [u8]>
where
    R: ReadRef<'data>
{
    pub(super) header: &'data pef::PEFLoaderInfoHeader,
    pub(super) strings: StringTable<'data, R>,
    pub(super) offset: u32,
}

impl<'data, R> PefLoaderParser<'data, R>
where
    R: ReadRef<'data>,
{
    /// Parse PEF loader
    pub fn parse(data: R, offset: u32) -> Result<Self> {
        let header = pef::PEFLoaderInfoHeader::parse(data, offset)?;
        let strings = Self::loader_strings(data, *header, offset).expect("Could not get loader strings");
        Ok(PefLoaderParser{
            header,
            strings,
            offset,
        })
    }

    fn imports(&self, data: R, loader_offset: u32) -> Result<Vec<Import<'data>>> {

        let import_headers_off = loader_offset + mem::size_of::<pef::PEFLoaderInfoHeader>() as u32;
        let import_headers_count = self.header.imported_library_count.get(BE) as usize;
        let import_headers = data.read_slice_at::<pef::PEFImportedLibrary>(import_headers_off as u64, import_headers_count)
            .read_error("Could not read PEFImportedLibrary headers")?;

        let import_symbols_off = import_headers_off + mem::size_of::<pef::PEFImportedLibrary>() as u32 * import_headers_count as u32;
        let import_symbols_count = self.header.total_imported_symbol_count.get(BE) as usize;
        let import_symbols = data.read_slice_at::<pef::PEFSymbol>(import_symbols_off as u64, import_symbols_count)
            .read_error("Could not read imported symbols (PEFImportedSymbol)")?;

        let mut import_vec: Vec<Import<'data>> = Vec::with_capacity(import_symbols_count);

        for import_header in import_headers {
            let offset = import_header.name_offset.get(BE);
            let import_library_name = self.strings.get(offset).expect("Could not get imported library name");
            
            let start = import_header.first_imported_symbol.get(BE) as usize;
            let end = start + import_header.imported_symbol_count.get(BE) as usize;
            for symbol_idx in start..end {
                let symbol = import_symbols.get(symbol_idx).expect(format!("Could not get import symbol at idx {}", symbol_idx).as_str());
                let name_offset = symbol.name_offset();
                let symbol_name = self.strings.get(name_offset).expect("Could not get imported symbol name");
                import_vec.push(Import {
                    library: read::util::ByteString(import_library_name),
                    name: read::util::ByteString(symbol_name),
                });
            }
        }
        Ok(import_vec)
    }

    fn exports(&self, data: R, loader_offset: u32, sections: &SectionTable<'data, R>) -> Result<Vec<Export<'data>>> {
        let export_hash_slot_offset = loader_offset + self.header.export_hash_offset.get(BE);
        let export_hash_slot_count = (2u32)
            .checked_pow(self.header.export_hash_table_power.get(BE))
            .ok_or(Error("Invalid export hash size"))?;

        let export_key_table_offset = export_hash_slot_offset + mem::size_of::<pef::PEFExportedSymbolHashSlot>() as u32 * export_hash_slot_count;
        let export_symbol_count = self.header.exported_symbol_count.get(BE);
        let export_keys = data.read_slice_at::<pef::PEFExportedSymbolKey>(export_key_table_offset.into(), export_symbol_count as usize)
            .read_error("Could not read export key table")?;

        // align by 4 ? 
        let export_symbols_offset = export_key_table_offset + mem::size_of::<pef::PEFExportedSymbolKey>() as u32 * export_symbol_count;
        let export_symbols = data.read_slice_at::<pef::PEFExportedSymbol>(export_symbols_offset.into(), export_symbol_count as usize)
            .read_error("Could not read export symbols")?;

        let mut export_vec: Vec<Export<'data>> = Vec::with_capacity(export_symbol_count as usize);

        for symbol_idx in 0..export_symbol_count {
            let symbol_name_length = export_keys.get(symbol_idx as usize)
                .expect(&format!("Could not get export key at index {}", symbol_idx))
                .symbol_length.get(BE);

            let export_symbol = export_symbols.get(symbol_idx as usize)
                .expect(&format!("Could not get export symbol at index{}", symbol_idx));
            let symbol_name_offset = export_symbol
                .class_and_name
                .name_offset();

            let loader_strings_offset = loader_offset + self.header.loader_strings_offset.get(BE);
            let symbol_name_offset = loader_strings_offset + symbol_name_offset;

            let symbol_name = data.read_bytes_at(symbol_name_offset.into(), symbol_name_length.into())
                .expect("Could not read symbol name");

            let symbol_section = sections.section(SectionIndex(export_symbol.section_index.get(BE) as usize))
                .expect("Could not get exported symbol section");
            let symbol_offset = symbol_section.container_offset.get(BE) + export_symbol.symbol_value.get(BE);

            export_vec.push(Export {
                name: read::util::ByteString(symbol_name),
                address: symbol_offset.into(),
            });
        }
        Ok(export_vec)
    }

    fn loader_strings (
        data: R,
        header: pef::PEFLoaderInfoHeader,
        loader_offset: u32,
    ) -> read::Result<StringTable<'data, R>> {
        let start = loader_offset + header.loader_strings_offset.get(BE);
        let end = loader_offset + header.export_hash_offset.get(BE);
        Ok(StringTable::new(data, start as u64, end as u64))
    }
}

impl pef::PEFLoaderInfoHeader {
    /// Read and parse loader, relocations, symbols
    pub fn parse<'data, R: ReadRef<'data>>(data: R, offset: u32) -> read::Result<&'data Self> {
        let loader_header = data
            .read_at::<pef::PEFLoaderInfoHeader>(offset.into())
            .read_error("Invalid PEFLoaderInfoHeader header size or alignment")?;
        Ok(loader_header)
    }
}

impl<'data, R> read::private::Sealed for PefFile<'data, R>
where
    R: ReadRef<'data>,
{}

impl<'data, R: ReadRef<'data>> Object<'data> for PefFile<'data, R> {
    type Segment<'file>
        = PefSegment<'data, 'file, R>
    where
        Self: 'file,
        'data: 'file;
    type SegmentIterator<'file>
        = PefSegmentIterator<'data, 'file, R>
    where
        Self: 'file,
        'data: 'file;
    type Section<'file>
        = PefSection<'data, 'file, R>
    where
        Self: 'file,
        'data: 'file;
    type SectionIterator<'file>
        = PefSectionIterator<'data, 'file, R>
    where
        Self: 'file,
        'data: 'file;
    type Comdat<'file>
        = PefComdat<'data, 'file, R>
    where
        Self: 'file,
        'data: 'file;
    type ComdatIterator<'file>
        = PefComdatIterator<'data, 'file, R>
    where
        Self: 'file,
        'data: 'file;
    type Symbol<'file>
        = PefSymbol<'data, 'file>
    where
        Self: 'file,
        'data: 'file;
    type SymbolIterator<'file>
        = PefSymbolIterator<'data, 'file>
    where
        Self: 'file,
        'data: 'file;
    type SymbolTable<'file>
        = PefSymbolTable<'data, 'file>
    where
        Self: 'file,
        'data: 'file;
    type DynamicRelocationIterator<'file>
        = NoDynamicRelocationIterator
    where
        Self: 'file,
        'data: 'file;

    #[inline]
    fn architecture(&self) -> Architecture {
        match self.header.architecture.get(BE) {
            pef::ARCHITECTURE_PPC => Architecture::PowerPc,
            pef::ARCHITECTURE_68K => Architecture::M68k,
            _ => Architecture::Unknown,
        }
    }

    #[inline]
    fn is_little_endian(&self) -> bool {
        false
    }

    #[inline]
    fn is_64(&self) -> bool {
        false
    }

    fn kind(&self) -> ObjectKind {
        ObjectKind::Executable
    }

    fn segments(&self) -> Self::SegmentIterator<'_> {
        todo!();
    }

    fn section_by_name_bytes<'file>(
        &'file self,
        section_name: &[u8],
    ) -> Option<PefSection<'data, 'file, R>> {
        self.sections()
            .find(|section| section.name_bytes() == Ok(section_name))
    }

    fn section_by_index(&self, index: SectionIndex) -> Result<PefSection<'data, '_, R>> {
        let section = self.sections.section(index)?;
        Ok(PefSection {
            file: self,
            index,
            section,
        })
    }

    fn sections(&self) -> Self::SectionIterator<'_> {
        PefSectionIterator {
            file: self,
            iter: self.sections.iter().enumerate(),
        }
    }

    fn comdats(&self) -> Self::ComdatIterator<'_> {
        todo!()
    }

    #[inline]
    fn symbol_by_index(&self, index: SymbolIndex) -> Result<PefSymbol<'data, '_>> {
        todo!()
    }

    fn symbols(&self) -> Self::SymbolIterator<'_> {
        todo!()
    }

    fn symbol_table(&self) -> Option<PefSymbolTable<'data, '_>> {
        todo!()
    }

    fn dynamic_symbols(&self) -> Self::SymbolIterator<'_> {
        todo!()
    }

    #[inline]
    fn dynamic_symbol_table(&self) -> Option<PefSymbolTable<'data, '_>> {
        None
    }

    #[inline]
    fn dynamic_relocations(&self) -> Option<NoDynamicRelocationIterator> {
        None
    }

    fn imports(&self) -> Result<Vec<Import<'data>>> {
        self.loader.imports(self.data, self.loader.offset)
    }

    fn exports(&self) -> Result<Vec<Export<'data>>> {
        self.loader.exports(self.data, self.loader.offset, &self.sections)
    }

    fn has_debug_symbols(&self) -> bool {
        todo!()
    }

    fn relative_address_base(&self) -> u64 {
        0
    }

    #[inline]
    fn entry(&self) -> u64 {
        // all relocations must be applied
        /* 
        let entry_section_idx = self.loader.header.main_section.get(BE);
        let entry_section_offset = self.loader.header.main_offset.get(BE);
        let entry_section = self.sections.section(SectionIndex((entry_section_idx + 1) as usize))
            .expect("Could not get entry section");
        let entry_ptr_offset = entry_section.container_offset.get(BE) as u64 + entry_section_offset as u64;
        let ep_offset = self.data
            .read_at::<U32<BE>>(entry_ptr_offset)
            .expect("Could not read EP");
        ep_offset.get(BE) as u64
        */
        todo!();
    }

    #[inline]
    fn flags(&self) -> FileFlags {
        FileFlags::None
    }

}

impl<'data, R> PefFile<'data, R>
where 
    R: ReadRef<'data>,
{
    /// Returns compilation timestamp
    pub fn timestamp(&self) -> Option<DateTime<Utc>> {
        const Hfs_Unix_Offset : i64 = 2082844800;
        let seconds = self.header.date_time_stamp.get(BE) as i64 - Hfs_Unix_Offset;
        DateTime::from_timestamp(seconds, 0)
    }
}

/// An iterator for the COMDAT section groups in a [`PefFile`].
///
/// This is a stub that doesn't implement any functionality.
#[derive(Debug)]
pub struct PefComdatIterator<'data, 'file, R = &'data [u8]> 
where
    R: ReadRef<'data>,
{
    #[allow(unused)]
    file: &'file PefFile<'data, R>,
}

impl<'data, 'file, R> Iterator for PefComdatIterator<'data, 'file, R> 
where     
    R: ReadRef<'data>,
{
    type Item = PefComdat<'data, 'file, R>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}

/// A COMDAT section group in a [`PefFile`].
///
/// This is a stub that doesn't implement any functionality.
#[derive(Debug)]
pub struct PefComdat<'data, 'file, R = &'data [u8]> 
where
    R: ReadRef<'data>,
{
    #[allow(unused)]
    file: &'file PefFile<'data, R>,
}

impl<'data, 'file, R> read::private::Sealed for PefComdat<'data, 'file, R> 
where 
    R: ReadRef<'data>
{}

impl<'data, 'file, R> ObjectComdat<'data> for PefComdat<'data, 'file, R> 
where
    R: ReadRef<'data>,
{
    type SectionIterator = PefComdatSectionIterator<'data, 'file, R>;

    #[inline]
    fn kind(&self) -> ComdatKind {
        unreachable!();
    }

    #[inline]
    fn symbol(&self) -> SymbolIndex {
        unreachable!();
    }

    #[inline]
    fn name_bytes(&self) -> Result<&'data [u8]> {
        unreachable!();
    }

    #[inline]
    fn name(&self) -> Result<&'data str> {
        unreachable!();
    }

    #[inline]
    fn sections(&self) -> Self::SectionIterator {
        unreachable!();
    }
}

/// An iterator for the sections in a COMDAT section group in a [`PefFile`].
///
/// This is a stub that doesn't implement any functionality.
#[derive(Debug)]
pub struct PefComdatSectionIterator<'data, 'file, R = &'data [u8]> 
where
    R: ReadRef<'data>,
{
    #[allow(unused)]
    file: &'file PefFile<'data, R>,
}

impl<'data, 'file, R> Iterator for PefComdatSectionIterator<'data, 'file, R> 
where
    R: ReadRef<'data>,
{
    type Item = SectionIndex;

    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}

/// A symbol table in a [`PefFile`].
#[derive(Debug)]
pub struct PefSymbolTable<'data, 'file> {
    symbols: &'file [PefSymbolInternal<'data>],
}

impl<'data, 'file> read::private::Sealed for PefSymbolTable<'data, 'file> {}

impl<'data, 'file> ObjectSymbolTable<'data> for PefSymbolTable<'data, 'file> {
    type Symbol = PefSymbol<'data, 'file>;
    type SymbolIterator = PefSymbolIterator<'data, 'file>;

    fn symbols(&self) -> Self::SymbolIterator {
        PefSymbolIterator {
            symbols: self.symbols.iter().enumerate(),
        }
    }

    fn symbol_by_index(&self, index: SymbolIndex) -> Result<Self::Symbol> {
        let symbol = self
            .symbols
            .get(index.0)
            .read_error("Invalid PEF symbol index")?;
        Ok(PefSymbol { index, symbol })
    }
}

/// An iterator for the symbols in a [`PefFile`].
#[derive(Debug)]
pub struct PefSymbolIterator<'data, 'file> {
    symbols: core::iter::Enumerate<slice::Iter<'file, PefSymbolInternal<'data>>>,
}

impl<'data, 'file> Iterator for PefSymbolIterator<'data, 'file> {
    type Item = PefSymbol<'data, 'file>;

    fn next(&mut self) -> Option<Self::Item> {
        let (index, symbol) = self.symbols.next()?;
        Some(PefSymbol {
            index: SymbolIndex(index),
            symbol,
        })
    }
}

/// A symbol in a [`PefFile`].
///
/// Most functionality is provided by the [`ObjectSymbol`] trait implementation.
#[derive(Clone, Copy, Debug)]
pub struct PefSymbol<'data, 'file> {
    index: SymbolIndex,
    symbol: &'file PefSymbolInternal<'data>,
}

#[derive(Clone, Debug)]
struct PefSymbolInternal<'data> {
    name: &'data str,
    address: u64,
    size: u64,
    kind: SymbolKind,
    section: SymbolSection,
    scope: SymbolScope,
}

impl<'data, 'file> read::private::Sealed for PefSymbol<'data, 'file> {}

impl<'data, 'file> ObjectSymbol<'data> for PefSymbol<'data, 'file> {
    #[inline]
    fn index(&self) -> SymbolIndex {
        self.index
    }

    #[inline]
    fn name_bytes(&self) -> read::Result<&'data [u8]> {
        Ok(self.symbol.name.as_bytes())
    }

    #[inline]
    fn name(&self) -> read::Result<&'data str> {
        Ok(self.symbol.name)
    }

    #[inline]
    fn address(&self) -> u64 {
        self.symbol.address
    }

    #[inline]
    fn size(&self) -> u64 {
        self.symbol.size
    }

    #[inline]
    fn kind(&self) -> SymbolKind {
        self.symbol.kind
    }

    #[inline]
    fn section(&self) -> SymbolSection {
        self.symbol.section
    }

    #[inline]
    fn is_undefined(&self) -> bool {
        self.symbol.section == SymbolSection::Undefined
    }

    #[inline]
    fn is_definition(&self) -> bool {
        (self.symbol.kind == SymbolKind::Text || self.symbol.kind == SymbolKind::Data)
            && self.symbol.section != SymbolSection::Undefined
    }

    #[inline]
    fn is_common(&self) -> bool {
        self.symbol.section == SymbolSection::Common
    }

    #[inline]
    fn is_weak(&self) -> bool {
        false
    }

    #[inline]
    fn scope(&self) -> SymbolScope {
        self.symbol.scope
    }

    #[inline]
    fn is_global(&self) -> bool {
        self.symbol.scope != SymbolScope::Compilation
    }

    #[inline]
    fn is_local(&self) -> bool {
        self.symbol.scope == SymbolScope::Compilation
    }

    #[inline]
    fn flags(&self) -> SymbolFlags<SectionIndex, SymbolIndex> {
        SymbolFlags::None
    }
}
