use core::fmt::Debug;
use crate::pef::{self};
use crate::endian::BigEndian as BE;
use core::{slice, str, mem};
use alloc::vec::Vec;
use crate::read::{
    self, Architecture, ComdatKind, Error, Export, FileFlags,
    Import, NoDynamicRelocationIterator, Object, ObjectComdat, ObjectKind, ObjectSection,
    ObjectSymbol, ObjectSymbolTable, ReadError, ReadRef,
    Result, SectionIndex, SymbolFlags, SymbolIndex,
    SymbolKind, SymbolScope, SymbolSection, StringTable
};
use chrono::{DateTime, TimeZone, Utc};

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
}

impl<'data, R> PefFile<'data, R>
where
    R: ReadRef<'data>,
{
    /// Parse the raw PEF file data.
    pub fn parse(data: R) -> Result<Self> {
        let header = pef::PEFContainerHeader::parse(data)?;
        let sections = header.sections(data)?;
        Ok(PefFile {
            header,
            sections,
            data,
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
        ObjectKind::Unknown
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
        todo!()
    }

    fn exports(&self) -> Result<Vec<Export<'data>>> {
        todo!()
    }

    fn has_debug_symbols(&self) -> bool {
        todo!()
    }

    fn relative_address_base(&self) -> u64 {
        todo!()
    }

    #[inline]
    fn entry(&self) -> u64 {
        todo!()
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
