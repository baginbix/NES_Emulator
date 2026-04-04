use crate::{addr_register::AddrRegister, cartridge::Mirroring};

pub struct PPU{
    pub chr_rom: Vec<u8>,
    pub vram: [u8; 2048],
    pub palette_table: [u8; 32],
    pub oam_data: [u8; 256],

    pub mirroring: Mirroring,
    addr: AddrRegister,
}

impl PPU{
    pub fn new(chr_rom: Vec<u8>, mirroring: Mirroring) -> Self{
        PPU { 
            chr_rom: chr_rom, 
            vram: [0; 2048], 
            palette_table: [0; 32], 
            oam_data: [0; 256],
            mirroring: mirroring,
            addr: AddrRegister::new()
        }
    }

    fn write_to_ppu_addr(&mut self, data: u8){
        self.addr.update(data);
    }
}