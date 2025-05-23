use core::marker::PhantomData;
use core::{iter, slice, str};

use crate::{endian, pef};
use crate::endian::BigEndian as BE;
use crate::read::util::StringTable;
use crate::read::{
    self, CompressedData, CompressedFileRange, ObjectSection, ObjectSegment, ReadError, ReadRef,
    Relocation, RelocationMap, Result, SectionFlags, SectionIndex, SectionKind, SegmentFlags,
};

use super::PefFile;

/// An iterator for the loadable sections in a [`PefFile`].
#[derive(Debug)]
pub struct PefSegmentIterator<'data, 'file, R = &'data [u8]>
where
    R: ReadRef<'data>,
{
    pub(super) file: &'file PefFile<'data, R>,
    pub(super) iter: slice::Iter<'data, pef::PEFSectionHeader>,
}

impl<'data, 'file, R> Iterator for PefSegmentIterator<'data, 'file, R>
where
    R: ReadRef<'data>,
{
    type Item = PefSegment<'data, 'file, R>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|section| PefSegment {
            file: self.file,
            section,
        })
    }
}

/// A loadable section in a [`PefFile`].
///
/// Most functionality is provided by the [`ObjectSegment`] trait implementation.
#[derive(Debug)]
pub struct PefSegment<'data, 'file, R = &'data [u8]>
where
    R: ReadRef<'data>,
{
    file: &'file PefFile<'data, R>,
    section: &'data pef::PEFSectionHeader,
}

impl<'data, 'file, R> PefSegment<'data, 'file, R>
where
    R: ReadRef<'data>,
{
    /// Get the PE file containing this segment.
    pub fn pe_file(&self) -> &'file PefFile<'data, R> {
        self.file
    }

    /// Get the raw PE section header.
    pub fn pe_section(&self) -> &'data pef::PEFSectionHeader {
        self.section
    }
}

impl<'data, 'file, R> read::private::Sealed for PefSegment<'data, 'file, R>
where
    R: ReadRef<'data>,
{
}

impl<'data, 'file, R> ObjectSegment<'data> for PefSegment<'data, 'file, R>
where
    R: ReadRef<'data>,
{
    #[inline]
    fn address(&self) -> u64 {
        todo!();
    }

    #[inline]
    fn size(&self) -> u64 {
        todo!();
    }

    #[inline]
    fn align(&self) -> u64 {
        todo!();
    }

    #[inline]
    fn file_range(&self) -> (u64, u64) {
        todo!();
    }

    fn data(&self) -> Result<&'data [u8]> {
        todo!();
    }

    fn data_range(&self, address: u64, size: u64) -> Result<Option<&'data [u8]>> {
        todo!();
    }

    #[inline]
    fn name_bytes(&self) -> Result<Option<&[u8]>> {
        todo!();
    }

    #[inline]
    fn name(&self) -> Result<Option<&str>> {
        todo!();
    }

    #[inline]
    fn flags(&self) -> SegmentFlags {
        todo!();
    }
}

/// An iterator for the sections in a [`PefFile`].
#[derive(Debug)]
pub struct PefSectionIterator<'data, 'file, R = &'data [u8]>
where
    R: ReadRef<'data>,
{
    pub(super) file: &'file PefFile<'data, R>,
    pub(super) iter: iter::Enumerate<slice::Iter<'data, pef::PEFSectionHeader>>,
}

impl<'data, 'file, R> Iterator for PefSectionIterator<'data, 'file, R>
where
    R: ReadRef<'data>,
{
    type Item = PefSection<'data, 'file, R>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(index, section)| PefSection {
            file: self.file,
            index: SectionIndex(index + 1),
            section,
        })
    }
}

/// A section in a [`PefFile`].
///
/// Most functionality is provided by the [`ObjectSection`] trait implementation.
#[derive(Debug)]
pub struct PefSection<'data, 'file, R = &'data [u8]>
where
    R: ReadRef<'data>,
{
    pub(super) file: &'file PefFile<'data, R>,
    pub(super) index: SectionIndex,
    pub(super) section: &'data pef::PEFSectionHeader
}

impl<'data, 'file, R> PefSection<'data, 'file, R>
where
    R: ReadRef<'data>,
{
    /// Get the PEF file containing this section.
    pub fn pef_file(&self) -> &'file PefFile<'data, R> {
        self.file
    }

    /// Get the raw PE section header.
    pub fn pef_section(&self) -> &'data pef::PEFSectionHeader {
        self.section
    }
}

impl<'data, 'file, R> read::private::Sealed for PefSection<'data, 'file, R>
where
    R: ReadRef<'data>,
{
}

impl<'data, 'file, R> ObjectSection<'data> for PefSection<'data, 'file, R>
where
    R: ReadRef<'data>,
{
    type RelocationIterator = PefRelocationIterator<'data, 'file, R>;

    #[inline]
    fn index(&self) -> SectionIndex {
        self.index
    }

    #[inline]
    fn address(&self) -> u64 {
        u64::from(self.section.default_address.get(BE))
    }

    #[inline]
    fn size(&self) -> u64 {
        u64::from(self.section.total_size.get(BE))
    }

    #[inline]
    fn align(&self) -> u64 {
        u64::from(1u64 << self.section.alignment)
    }

    #[inline]
    fn file_range(&self) -> Option<(u64, u64)> {
        (self.section.packed_size.get(BE) != 0)
        .then_some((self.section.container_offset.get(BE) as u64, self.section.packed_size.get(BE) as u64))
    }

    fn data(&self) -> Result<&'data [u8]> {
        let (offset, size) = self.file_range().expect("Empty section");
        self.file.data.read_bytes_at(offset.into(), size.into()).read_error("Invalid PEF section offset or size")
    }

    fn data_range(&self, address: u64, size: u64) -> Result<Option<&'data [u8]>> {
        todo!();
    }

    #[inline]
    fn compressed_file_range(&self) -> Result<CompressedFileRange> {
        Ok(CompressedFileRange::none(self.file_range()))
    }

    #[inline]
    fn compressed_data(&self) -> Result<CompressedData<'data>> {
        self.data().map(CompressedData::none)
    }

    #[inline]
    fn name_bytes(&self) -> Result<&'data [u8]> {
        self.file
        .sections
        .section_name(self.section)
    }

    #[inline]
    fn name(&self) -> Result<&'data str> {
        let name = self.name_bytes()?;
        str::from_utf8(name)
            .ok()
            .read_error("Non UTF-8 PEF section name")
    }

    #[inline]
    fn segment_name_bytes(&self) -> Result<Option<&[u8]>> {
        Ok(None)
    }

    #[inline]
    fn segment_name(&self) -> Result<Option<&str>> {
        Ok(None)
    }

    #[inline]
    fn kind(&self) -> SectionKind {
        match self.section.section_kind {
            pef::SectionKind::Code => SectionKind::Text,
            pef::SectionKind::UnpackedData => SectionKind::Data,
            pef::SectionKind::PatternInitializedData => SectionKind::Data,
            pef::SectionKind::Constant => SectionKind::Data,
            pef::SectionKind::Loader => SectionKind::Metadata,
            pef::SectionKind::Debug => SectionKind::Debug,
            pef::SectionKind::ExecutableData => SectionKind::Text,
            pef::SectionKind::Exception => SectionKind::Unknown,
            pef::SectionKind::Traceback => SectionKind::Unknown,
        }
    }

    fn relocations(&self) -> PefRelocationIterator<'data, 'file, R> {
        PefRelocationIterator(PhantomData)
    }

    fn relocation_map(&self) -> read::Result<RelocationMap> {
        todo!();
    }

    fn flags(&self) -> SectionFlags {
        todo!();
    }
}

/// The table of section headers in an PEF file.
///
/// Also includes the string table used for the section names.
///
/// Returned by [`PEFContainerHeader::sections`].
#[derive(Debug, Clone, Copy)]
pub struct SectionTable<'data, R = &'data [u8]>
where
    R: ReadRef<'data>,
{
    sections: &'data [pef::PEFSectionHeader],
    strings: StringTable<'data, R>,
}

impl<'data, R: ReadRef<'data>> Default for SectionTable<'data, R> {
    fn default() -> Self {
        SectionTable {
            sections: &[],
            strings: StringTable::default(),
        }
    }
}

impl<'data, R: ReadRef<'data>> SectionTable<'data, R> {
    /// Create a new section table.
    #[inline]
    pub fn new(sections: &'data [pef::PEFSectionHeader], strings: StringTable<'data, R>) -> Self {
        SectionTable { sections, strings }
    }

    /// Iterate over the section headers.
    ///
    /// Warning: section indices start at 1.
    #[inline]
    pub fn iter(&self) -> slice::Iter<'data, pef::PEFSectionHeader> {
        self.sections.iter()
    }

    /// Iterate over the section headers and their indices.
    pub fn enumerate(&self) -> impl Iterator<Item = (SectionIndex, &'data pef::PEFSectionHeader)> {
        self.sections
            .iter()
            .enumerate()
            .map(|(i, section)| (SectionIndex(i + 1), section))
    }

    /// Return true if the section table is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.sections.is_empty()
    }

    /// The number of section headers.
    #[inline]
    pub fn len(&self) -> usize {
        self.sections.len()
    }

    /// Return the section header at the given index.
    ///
    /// The index is 1-based.
    pub fn section(&self, index: SectionIndex) -> read::Result<&'data pef::PEFSectionHeader> {
        self.sections
            .get(index.0.wrapping_sub(1))
            .read_error("Invalid PEF section index")
    }

    /// Return the section header with the given name.
    ///
    /// The returned index is 1-based.
    ///
    /// Ignores sections with invalid names.
    pub fn section_by_name(
        &self,
        name: &[u8],
    ) -> Option<(SectionIndex, &'data pef::PEFSectionHeader)> {
        self.enumerate()
        .find(|(_, section)| self.section_name(section) == Ok(name))
    }

    /// Return the section name for the given section header.
    pub fn section_name(
        &self,
        section: &pef::PEFSectionHeader,
    ) -> read::Result<&'data [u8]> {
        section.name(self.strings)
    }

    /// Compute the maximum file offset used by sections.
    ///
    /// This will usually match the end of file, unless the PE file has a
    /// [data overlay](https://security.stackexchange.com/questions/77336/how-is-the-file-overlay-read-by-an-exe-virus)
    pub fn max_section_file_offset(&self) -> u64 {
        todo!();
    }
}

impl pef::PEFSectionHeader {
    /// Return the offset and size of the section in a PE file.
    ///
    /// The size of the range will be the minimum of the file size and virtual size.
    pub fn pef_file_range(&self) -> (u32, u32) {
        todo!();
    }

    /// Return the file offset of the given virtual address, and the remaining size up
    /// to the end of the section.
    ///
    /// Returns `None` if the section does not contain the address.
    pub fn pef_file_range_at(&self, va: u32) -> Option<(u32, u32)> {
        todo!();
    }

    /// Return the virtual address and size of the section.
    pub fn pef_address_range(&self) -> (u32, u32) {
        todo!();
    }

    /// Return the section data in a PE file.
    ///
    /// The length of the data will be the minimum of the file size and virtual size.
    pub fn pef_data<'data, R: ReadRef<'data>>(&self, data: R) -> Result<&'data [u8]> {
        todo!();
    }

    /// Return the data starting at the given virtual address, up to the end of the
    /// section.
    ///
    /// Ignores sections with invalid data.
    ///
    /// Returns `None` if the section does not contain the address.
    pub fn pe_data_at<'data, R: ReadRef<'data>>(&self, data: R, va: u32) -> Option<&'data [u8]> {
        todo!();
    }

    /// Tests whether a given RVA is part of this section
    pub fn contains_rva(&self, va: u32) -> bool {
        todo!();
    }

    /// Return the section data if it contains the given virtual address.
    ///
    /// Also returns the virtual address of that section.
    ///
    /// Ignores sections with invalid data.
    pub fn pe_data_containing<'data, R: ReadRef<'data>>(
        &self,
        data: R,
        va: u32,
    ) -> Option<(&'data [u8], u32)> {
        todo!();
    }

    /// Parse the section name from the string table.
    fn name<'data, R: ReadRef<'data>>(
        &self,
        strings: StringTable<'data, R>,
    ) -> read::Result<&'data [u8]> {
        strings
            .get(self.name_offset.get(BE))
            .read_error("Invalid PEF section name offset")
    }

}

/// An iterator for the relocations in an [`PefSection`].
///
/// This is a stub that doesn't implement any functionality.
#[derive(Debug)]
pub struct PefRelocationIterator<'data, 'file, R = &'data [u8]>(
    PhantomData<(&'data (), &'file (), R)>,
);

impl<'data, 'file, R> Iterator for PefRelocationIterator<'data, 'file, R> {
    type Item = (u64, Relocation);

    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}
