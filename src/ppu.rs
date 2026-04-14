pub mod control_register;

pub struct PPU{
    pub char_rom: Vec<u8>,
    pub palatte_table: [u8;32],
    pub vram: [u8; 2048],
    pub oam_data: [u8; 256],
    addr: AddrRegister,
    pub mirroring: Mirroring
}

impl PPU{
    pub fn new(char_rom: Vec<u8>, mirroring:Mirroring) -> Self{
        return PPU{
            char_rom: char_rom,
            mirroring: mirroring,
            vram: [0; 2048],
            oam_data: [0; 64*4],
            palatte_table: [0; 32],
            addr: AddrRegister::new()
        }
    }

    pub fn write_to_addr(&mut self, value:u8){
        self.addr.update(value);
    }


}