use crate::{ppu::PPU, cartridge::{Mirroring, Rom}, cpu::Mem};


const RAM: u16 = 0x0000;
const RAM_MIRRORS_END: u16 = 0x1fff;
const PPU_REGISTERS: u16 = 0x2000;
const PPU_REGISTERS_READABLE_START: u16 = 0x2008;
const PPU_REGISTERS_MIRRORS_END:u16 = 0x3fff; 
const PPU_DATA_REGISTER:u16 = 0x2007;
const PPU_ADDRESS_REGISTER:u16 = 0x2006;
const PPU_CTRL_REGISTER:u16 = 0x2000;
const PPU_SCROLL_REGISTER:u16 = 0x2005;

pub struct Bus{
    cpu_vram: [u8;2048],
    prg_rom:Vec<u8>,
    ppu:PPU
}

impl Bus{
    pub fn new(rom: Rom) -> Bus{
        let ppu = PPU::new(rom.chr_rom, rom.screen_mirroring);
        Bus { 
            cpu_vram: [0;2048],
            prg_rom: rom.prg_rom,
            ppu: ppu
        }
    }

    fn read_prg_rom(&self, mut addr: u16) -> u8{
        addr -= 0x8000;
        if self.prg_rom.len() == 0x4000 && addr >= 0x4000{
            addr = addr % 0x4000;
        }
        self.prg_rom[addr as usize]
    }
}

impl Mem for Bus{
    fn mem_read(&mut self, addr: u16) -> u8 {
        match addr{
            RAM ..= RAM_MIRRORS_END => {
                let mirror_down_addr = addr & 0x7FF;
                return self.cpu_vram[mirror_down_addr as usize] 
            },
            PPU_REGISTERS | 0x2001 | 0x2003 | 0x2005 | 0x2006 | 0x4014 =>{
                panic!("Attempt to read from write-only PPU address {:x}", addr);
            },
            0x2007 => self.ppu.read_data()
            PPU_REGISTERS_READABLE_START..= PPU_REGISTERS_MIRRORS_END =>{
                let mirror_down_addr = addr & 0b0010_0000_0000_0111;
                self.mem_read(mirror_down_addr)
            },
            0x8000..=0xffff => self.read_prg_rom(addr),
            _=>{
                println!("Ignoring mem access at {}", addr);
                0
            }
        }
    }

    fn mem_write(&mut self, addr: u16, data: u8) {
        match addr{
            RAM ..= RAM_MIRRORS_END => {
                let mirror_down_addr = addr & 0b1111_1111_1111_1111;
                self.cpu_vram[mirror_down_addr as usize] = data; 
            },
            PPU_CTRL_REGISTER => self.ppu.write_to_ctrl(data),
            PPU_SCROLL_REGISTER => ,
            PPU_ADDRESS_REGISTER => self.ppu.write_to_addr(data),
            PPU_DATA_REGISTER => self.ppu.write_to_data(data),
            0x2008 ..= PPU_REGISTERS_MIRRORS_END =>{
                let mirror_down_addr = addr & 0b0010_0000_0000_0111;
                self.mem_write(mirror_down_addr, data);
            },
            0x8000..=0xffff => panic!("Attempt to write to Cartridge ROM space"),
            _=>{
                println!("Ignore mem write-access at {}", addr);
            }
        }
    }
}