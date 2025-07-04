use std::vec::Vec;

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
    output_index: usize,
    input: &'data [u8],
    output: &'data mut Vec<u8>,
}

impl<'data> PefSectionDecompressor<'data>
{
    /// Create new decompressor
    pub fn new(_input: &'data [u8], _output: &'data mut Vec<u8>) -> Self {
        PefSectionDecompressor{ 
            input_index:0, 
            output_index:0,
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
            self.input_index += 1;
            self.execute_opcode(opcode);
        }
        true
    }

    fn unpack_count(&mut self) -> usize {
        let mut unpacked: usize = 0;
		while (self.input_index < self.input.len()) {
			unpacked <<= 7;
			let next_val: u8 = self.input[self.input_index];
            unpacked += (next_val & 0x7f) as usize;
            self.input_index += 1;
			if ((next_val & 0x80) == 0x00) {
				break;
			}
		}
		return unpacked;
    }

    fn decode_opcode(&mut self, value: u8) -> Option<DecompressOpcode> {
        let opcode_raw = value >> 5;
        let mut count: usize     = (value & 0b0001_1111).into();
        if (count == 0) {
            count = self.unpack_count();
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
                self.output.extend(std::iter::repeat(0).take(opcode.count));
                self.output_index += opcode.count;
            }
            DecompressOpcodeKind::BlockCopy => {
            }
            DecompressOpcodeKind::RepeatedBlock => {
            }
            DecompressOpcodeKind::InterleaveRepeatBlockWithBlockCopy => {
            }
            DecompressOpcodeKind::InterleaveRepeatBlockWithZero => {
            }
            _ => unimplemented!(),
        }
    }
}
