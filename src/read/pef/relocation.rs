use alloc::fmt;
use crate::read::{ ReadRef, Relocation, RelocationMap, Result, SectionIndex };
use crate::read::pef::PefFile;

/// An iterator for the relocations for an [`PefSection`](super::PefSection).
pub struct PefSectionRelocationIterator<'data, 'file, R = &'data [u8]>
where
    R: ReadRef<'data>,
{
    /// The current pointer in the chain of relocation sections.
    pub(super) section_index: SectionIndex,
    pub(super) file: &'file PefFile<'data, R>,
    //pub(super) relocations: Option<ElfRelaIterator<'data>>,
}

impl<'data, 'file, R> Iterator for PefSectionRelocationIterator<'data, 'file, R>
where
    R: ReadRef<'data>,
{
    type Item = (u64, Relocation);

    fn next(&mut self) -> Option<Self::Item> {
        todo!();
        /* 
        loop {
            if let Some(ref mut relocations) = self.relocations {
                if let Some(reloc) = relocations.next() {
                    let relocation =
                        parse_relocation(self.file.header, endian, reloc, relocations.is_rel());
                    return Some((reloc.r_offset(endian).into(), relocation));
                }
                self.relocations = None;
            }
            self.section_index = self.file.relocations.get(self.section_index)?;
            // The construction of RelocationSections ensures section_index is valid.
            let section = self.file.sections.section(self.section_index).unwrap();
            match section.sh_type(endian) {
                elf::SHT_REL => {
                    if let Ok(relocations) = section.data_as_array(endian, self.file.data) {
                        self.relocations = Some(ElfRelaIterator::Rel(relocations.iter()));
                    }
                }
                elf::SHT_RELA => {
                    if let Ok(relocations) = section.data_as_array(endian, self.file.data) {
                        self.relocations = Some(ElfRelaIterator::Rela(relocations.iter()));
                    }
                }
                _ => {}
            }
        }
        */
    }
}

impl<'data, 'file, R> fmt::Debug for PefSectionRelocationIterator<'data, 'file, R>
where
    R: ReadRef<'data>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PefSectionRelocationIterator").finish()
    }
}