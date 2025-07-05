use std::vec::Vec;

use crate::common;

#[derive(Debug)]
#[repr(u8)]
enum DecompressOpcodeKind {
    Zero = 0b000,
    BlockCopy = 0b001,
    RepeatedBlock = 0b010,
    InterleaveRepeatBlockWithBlockCopy = 0b011,
    InterleaveRepeatBlockWithZero = 0b100,
}

#[derive(Debug)]
struct DecompressOpcode {
    kind: DecompressOpcodeKind,
    count:   usize,
}

#[derive(Debug)]
/// Decompressor for PEF Pattern Initialized section
pub struct PefSectionDecompressor<'data>
{
    input_index: usize,
    input: &'data [u8],
    output: &'data mut Vec<u8>,
}

impl<'data> PefSectionDecompressor<'data>
{
    /// Create new decompressor
    pub fn new(_input: &'data [u8], _output: &'data mut Vec<u8> ) -> Self {
        PefSectionDecompressor{ 
            input_index:0, 
            input: _input,
            output: _output,
        }
    }

    /// Decompress to vector
    pub fn decompress_vec(
        &mut self,
    ) -> bool {
        while self.input_index < self.input.len() {

            let opcode = match self.decode_opcode(self.input[self.input_index]) {
                Some(op) => op,
                None => return false,
            };
            self.execute_opcode(opcode);
        }
        true
    }

    fn unpack_val(&mut self) -> usize {
        let mut unpacked = 0;
        while let Some(&byte) = self.input.get(self.input_index) {
            unpacked = (unpacked << 7) | ((byte & 0x7F) as usize);
            self.move_input_idx(1);
            if byte & 0x80 == 0 {
                break;
            }
        }
        unpacked
    }

    fn move_input_idx(&mut self, val: usize) {
        self.input_index = self.input_index
            .checked_add(val)
            .filter(|&n| n <= self.input.len())
            .expect("Out of bounds read");
    }

    fn fill_with_zeroes(&mut self, count: usize) {
        self.output.extend(std::iter::repeat(0).take(count));
    }

    fn fill_with_block(&mut self, count: usize) {
        self.output.extend_from_slice(&self.input[self.input_index..self.input_index+count]);
        self.move_input_idx(count);
    }

    fn get_slice(&mut self, count: usize) -> &[u8] {
        let start_idx = self.input_index;
        let end_idx = self.input_index + count;
        self.move_input_idx(count);
        &self.input[start_idx..end_idx]
    }

    fn decode_opcode(&mut self, value: u8) -> Option<DecompressOpcode> {
        let opcode_raw = value >> 5;
        let mut count: usize = (value & 0b0001_1111).into();
        self.move_input_idx(1);
        if (count == 0) {
            count = self.unpack_val();
        }

       let kind = match opcode_raw {
            0 => DecompressOpcodeKind::Zero,
            1 => DecompressOpcodeKind::BlockCopy,
            2 => DecompressOpcodeKind::RepeatedBlock,
            3 => DecompressOpcodeKind::InterleaveRepeatBlockWithBlockCopy,
            4 => DecompressOpcodeKind::InterleaveRepeatBlockWithZero,
            _ => return None,
        };
        Some(DecompressOpcode { kind, count: count as usize })
    }

    fn execute_opcode(&mut self, opcode: DecompressOpcode) {
        match opcode.kind {
            DecompressOpcodeKind::Zero => {
                self.fill_with_zeroes(opcode.count);
            }
            DecompressOpcodeKind::BlockCopy => {
                self.fill_with_block(opcode.count);
            }
            DecompressOpcodeKind::RepeatedBlock => {
                let repeat_count = self.unpack_val();
                let data = self.get_slice(opcode.count).to_vec();
                for _ in 0..=repeat_count {
                    self.output.extend(&data);
                }
            }
            DecompressOpcodeKind::InterleaveRepeatBlockWithBlockCopy => {
                let common_size = opcode.count;
				let custom_size = self.unpack_val();
				let repeat_count = self.unpack_val();

                let common_data = self.get_slice(common_size).to_vec();
                for _ in 0..repeat_count {
                    self.output.extend(&common_data);
                    self.fill_with_block(custom_size);
                }
                self.output.extend(&common_data);
            }
            DecompressOpcodeKind::InterleaveRepeatBlockWithZero => {
                let common_size = opcode.count;
                let custom_size = self.unpack_val();
                let repeat_count = self.unpack_val();

                for _ in 0..repeat_count {
                    self.fill_with_zeroes(common_size);
                    self.fill_with_block(custom_size);
                }
                self.fill_with_zeroes(common_size);
            }
            _ => unimplemented!(),
        }
    }
}
