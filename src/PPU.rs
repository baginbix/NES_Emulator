use crate::{addr_register::AddrRegister, cartridge::Mirroring, control_register::ControlRegister};
 
pub struct PPU{
    pub chr_rom: Vec<u8>,
    pub vram: [u8; 2048],
    pub palette_table: [u8; 32],
    pub oam_data: [u8; 256],

    pub mirroring: Mirroring,
    addr: AddrRegister,
    ctrl: ControlRegister,
    internal_dat_buf:u8,
}

impl PPU{
    pub fn new(chr_rom: Vec<u8>, mirroring: Mirroring) -> Self{
        PPU { 
            chr_rom: chr_rom, 
            vram: [0; 2048], 
            palette_table: [0; 32], 
            oam_data: [0; 256],
            mirroring: mirroring,
            addr: AddrRegister::new(),
            ctrl: ControlRegister ::new(),
            internal_dat_buf : 0,
        }
    }

    fn write_to_addr(&mut self, data: u8){
        self.addr.update(data);
    }

    fn write_to_ctrl(&mut self, data: u8){
        self.ctrl.update(data);
    }
    fn vram_addr_increment(&mut self) {
        self.addr.increment(self.ctrl.vram_addr_increment());
    }

    fn read_data(&mut self) -> u8{
        let addr = self.addr.get(); 
        self.vram_addr_increment();
        match addr{
            0..=0x1fff=>{
                let result = self.internal_dat_buf;
                self.internal_dat_buf = self.chr_rom[addr as usize];
                result
            },
            0x2000..=0x2fff=>{
                let result = self.internal_dat_buf;
                self.internal_dat_buf = self.vram[self.mirror_vram_addr(addr) as usize];
                result
            },
            0x3000..=0x3eff=>panic!("addr space 0x3000..0x3eff is not expected to be used, requested = {} ", addr),
            0x3f00..=0x3fff=>self.palette_table[(addr-0x3f00)as usize],
            _=> panic!("unexpected access to mirrored space {}", addr),
        }
 
    }

    fn mirror_vram_addr(&self, addr:u16) -> u16{
        let mirrored_vram = addr & 0x2fff;
        let vram_index = mirrored_vram - 0x2000;
        let nametable = vram_index  / 0x400;
        match (&self.mirroring, nametable) {
            (Mirroring::Vertical, 2) | (Mirroring::Vertical, 3) => vram_index -0x800,
            (Mirroring::Horizontal, 2) => vram_index - 0x400,
            (Mirroring::Horizontal, 1) => vram_index - 0x400,
            (Mirroring::Horizontal, 3) => vram_index - 0x800,
            _=> vram_index,
        }
    }
}
 