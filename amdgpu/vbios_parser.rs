use core::mem::{size_of, MaybeUninit};
use core::ptr;
pub use crate::bindings::{atom_common_table_header, atom_rom_header_v2_2, atom_master_data_table_v2_1, atom_firmware_info_v3_4, atom_firmware_info_v3_5};
use crate::AMDGPU::PPTable;

// ref: drivers/gpu/drm/amd/amdgpu/amdgpu_bios.c
// ref: drivers/gpu/drm/amd/amdgpu/atom.c

const SIGNATURE: &[u8] = b" 761295520";
const SIGNATURE_OFFSET: usize = 0x30;
const SIGNATURE_END: usize = SIGNATURE_OFFSET + SIGNATURE.len();
const VALID_VBIOS: &[u8] = &[0x55, 0xAA];
const ROM_TABLE_PTR: usize = 0x48;
const VBIOS_DATE_OFFSET: usize = 0x50;

#[derive(Debug, Clone)]
pub struct VbiosParser(Vec<u8>);

impl VbiosParser {
    pub fn new(v: Vec<u8>) -> Self {
        Self(v)
    }

    pub fn vbios(&self) -> Vec<u8> {
        self.0.clone()
    }

    pub fn length(&self) -> usize {
        let Some(length) = self.0.get(2) else { return 0 };

        usize::from(*length) << 9
    }

    pub fn valid_vbios(&self) -> bool {
        let Some(p) = self.0.get(..2) else { return false };
        let Some(sig) = self.0.get(SIGNATURE_OFFSET..SIGNATURE_END) else { return false };

        p == VALID_VBIOS && sig == SIGNATURE
    }

    pub fn check_length(&self) -> bool {
        self.length() == self.0.len()
    }

    pub fn get_vbios_name(&self) -> Option<String> {
        const OFFSET_TO_GET_ATOMBIOS_NUMBER_OF_STRINGS: usize = 0x2F;
        const OFFSET_TO_GET_ATOMBIOS_STRING_START: usize = 0x6E;
        const STRLEN_LONG: usize = 64;

        let str_num = self.0.get(OFFSET_TO_GET_ATOMBIOS_NUMBER_OF_STRINGS)?;
        let offset_str_start = self.read_u16(OFFSET_TO_GET_ATOMBIOS_STRING_START)? as usize;
        let mut skip_str = 0usize;

        for _i in 0..*str_num {
            while let Some(s) = self.0.get(offset_str_start+skip_str) {
                if *s != 0 {
                    skip_str += 1;
                } else {
                    break;
                }
            }

            skip_str += 1;
        }

        // skip: 0x0D 0x0A
        skip_str += 2;

        let offset = offset_str_start+skip_str;
        let s = self.0.get(offset..offset+STRLEN_LONG)?;
        let padding_pos = s.iter().rev().position(|&x| x != b' ')?;
        let s = s.get(..STRLEN_LONG-padding_pos)?;

        String::from_utf8(s.to_vec()).ok()
    }

    pub fn get_date(&self) -> Option<Vec<u8>> {
        let rom = self.0.get(VBIOS_DATE_OFFSET..VBIOS_DATE_OFFSET+14)?;

        let mut date: [u8; 16] = [
            b'2', b'0', 0, 0, b'/', 0, 0, b'/', 0, 0, b' ', 0, 0, 0, 0, 0, // b'\0',
        ];

        date[2] = rom[6];
        date[3] = rom[7];
        date[5] = rom[0];
        date[6] = rom[1];
        date[8] = rom[3];
        date[9] = rom[4];
        date[11] = rom[9];
        date[12] = rom[10];
        date[13] = rom[11];
        date[14] = rom[12];
        date[15] = rom[13];

        Some(date.to_vec())
    }

    fn to_struct<T>(bin: &[u8], size: usize) -> T {
        unsafe {
            let mut s = MaybeUninit::<T>::zeroed();

            ptr::copy_nonoverlapping(
                bin.as_ptr(),
                s.as_mut_ptr() as *mut u8,
                size,
            );

            s.assume_init()
        }
    }

    fn read_u16(&self, offset: usize) -> Option<u16> {
        self.0.get(offset..offset+2)
            .and_then(|r| r.try_into().ok())
            .map(|arr| u16::from_le_bytes(arr))
    }

    fn get_size_from_header(&self, offset: usize) -> Option<usize> {
        let size = self.read_header(offset)?.structuresize as usize;

        Some(size)
    }

    pub fn read_header(&self, offset: usize) -> Option<atom_common_table_header> {
        if offset == 0 { return None }

        let size = size_of::<atom_common_table_header>();
        let range = offset..offset+size;

        let h = self.0.get(range)?;

        Some(Self::to_struct(h, size))
    }

    fn read_table_unchecked_size<T>(&self, offset: usize) -> Option<T> {
        let size = self.get_size_from_header(offset)?;
        let t = self.0.get(offset..offset+size)?;

        Some(Self::to_struct(t, size))
    }

    pub fn read_table<T>(&self, offset: usize) -> Option<T> {
        let size = self.get_size_from_header(offset)?;

        if size != size_of::<T>() { return None }

        let t = self.0.get(offset..offset+size)?;

        Some(Self::to_struct(t, size))
    }

    pub fn get_atom_rom_header(&self) -> Option<atom_rom_header_v2_2> {
        let offset = self.read_u16(ROM_TABLE_PTR)?;

        /* only `atom_rom_header_v2_2` is defined */
        let rom_header = self.read_table_unchecked_size::<atom_rom_header_v2_2>(offset as usize)?;

        if &rom_header.atom_bios_string == b"ATOM" {
            Some(rom_header)
        } else {
            None
        }
    }

    pub fn get_atom_data_table(
        &self,
        rom_header: &atom_rom_header_v2_2,
    ) -> Option<atom_master_data_table_v2_1> {
        self.read_table(rom_header.masterdatatable_offset as usize)
    }

    pub fn get_atom_firmware_info_header(
        &self,
        data_table: &atom_master_data_table_v2_1,
    ) -> Option<atom_common_table_header> {
        self.read_header(data_table.listOfdatatables.firmwareinfo as usize)
    }

    pub fn get_atom_firmware_info(
        &self,
        data_table: &atom_master_data_table_v2_1,
    ) -> Option<atom_firmware_info_v3_4> {
        self.read_table_unchecked_size(data_table.listOfdatatables.firmwareinfo as usize)
    }

    pub fn get_atom_firmware_info_v3_5(
        &self,
        data_table: &atom_master_data_table_v2_1,
    ) -> Option<atom_firmware_info_v3_5> {
        self.read_table_unchecked_size(data_table.listOfdatatables.firmwareinfo as usize)
    }

    pub fn get_powerplay_table_bytes(
        &self,
        data_table: &atom_master_data_table_v2_1,
    ) -> Option<&[u8]> {
        let offset = data_table.listOfdatatables.powerplayinfo as usize;
        if offset == 0 { return None }

        self.0.get(offset..)
    }

    pub fn get_powerplay_table(
        &self,
        data_table: &atom_master_data_table_v2_1,
    ) -> Option<PPTable> {
        let bytes = self.get_powerplay_table_bytes(data_table)?;

        PPTable::decode(bytes).ok()
    }
}
