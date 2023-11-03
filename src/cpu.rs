use std::{collections::HashMap, hash::Hash, arch::global_asm};
use crate::opcodes;
use bitflags::bitflags;

const STACK: u16 = 0x0100;


pub struct CPU{
    pub register_a: u8,
    pub register_x: u8,
    pub register_y: u8,
    
    /// # Status Register (P) http://wiki.nesdev.com/w/index.php/Status_flagszs
    ///  7 6 5 4 3 2 1 0
    ///  N V _ B D I Z C
    ///  | |   | | | | +--- Carry Flag
    ///  | |   | | | +----- Zero Flag
    ///  | |   | | +------- Interrupt Disable
    ///  | |   | +--------- Decimal Mode (not used on NES)
    ///  | |   +----------- Break Command
    ///  | +--------------- Overflow Flag
    ///  +----------------- Negative Flag
    pub status: u8,
    pub program_counter: u16,
    pub stack_pointer: u8,
    memory: [u8; 0xffff],
}


#[derive(Debug)]
#[allow(non_camel_case_types)]
pub enum AddressingMode{
    Immediate,
    ZeroPage,
    ZeroPage_X,
    ZeroPage_Y,
    Absolute,
    Absolute_X,
    Absolute_Y,
    Indirect_X,
    Indirect_Y,
    NoneAddressing,
}

trait  Mem {
    fn mem_read(&self, addr: u16) -> u8;

    fn mem_write(&mut self, addr: u16, data: u8);

    fn mem_read_u16(&self, pos: u16) -> u16{
        let lo = self.mem_read(pos) as u16;
        let hi = self.mem_read(pos + 1) as u16;
        return (hi << 8) | lo;
    }

    fn mem_write_u16(&mut self, pos: u16, data: u16){
        let lo = (data & 0xff) as u8;
        let hi = (data >> 8) as u8;
        self.mem_write(pos, lo);
        self.mem_write(pos+1, hi)
    }
}

impl Mem for CPU {
    fn mem_read(&self, addr: u16) -> u8{
        self.memory[addr as usize]
    }

    fn mem_write(&mut self, addr: u16, data: u8){
        self.memory[addr as usize] =  data;
    }
}



impl CPU{
    pub fn new() -> Self{
        CPU{
            register_a: 0,
            status: 0b100100,
            program_counter: 0,
            register_x: 0,
            register_y: 0,
            memory: [0; 0xffff],
            stack_pointer: 0xff,
        }
    }

    fn set_carry_flag(&mut self){
        self.status = self.status | 0b0000_0001;
    }

    fn clear_carry_flag(&mut self){
        self.status = self.status & 0b1111_1110;
    }

    fn stack_push(&mut self, data: u8){
        let addr = self.stack_pointer as u16 + STACK;
        self.mem_write(addr, data);
        self.stack_pointer = self.stack_pointer.wrapping_sub(1);
    }
    
    fn stack_pop(&mut self) -> u8{
        self.stack_pointer = self.stack_pointer.wrapping_add(1);
        return self.mem_read(self.stack_pointer as u16 + STACK);
    }

    fn get_operand_address(&self, mode: &AddressingMode) -> u16{
        match mode {
            AddressingMode::Immediate => self.program_counter,
            
            AddressingMode::ZeroPage => self.mem_read(self.program_counter) as u16,
            
            AddressingMode::Absolute => self.mem_read_u16(self.program_counter),

            AddressingMode::ZeroPage_X => {
                let pos = self.mem_read(self.program_counter);
                let addr = pos.wrapping_add(self.register_x) as u16;
                addr
            }
            AddressingMode::ZeroPage_Y => {
                let pos = self.mem_read(self.program_counter);
                let addr = pos.wrapping_add(self.register_y) as u16;
                addr
            },
            AddressingMode::Absolute_X =>{ 
                let base = self.mem_read_u16(self.program_counter);
                let addr = base.wrapping_add(self.register_x as u16);
                addr
            }
            AddressingMode::Absolute_Y =>{
                let base = self.mem_read_u16(self.program_counter);
                let addr = base.wrapping_add(self.register_y as u16);
                addr
            },
            AddressingMode::Indirect_X => {
                let base = self.mem_read(self.program_counter);

                let ptr: u8 = (base as u8).wrapping_add(self.register_x);
                let lo = self.mem_read(ptr as u16);
                let hi = self.mem_read(ptr.wrapping_add(1) as u16);
                (hi as u16) << 8 | (lo as u16)
            },
            AddressingMode::Indirect_Y => {
                let base = self.mem_read(self.program_counter);

                let lo = self.mem_read(base as u16);
                let hi = self.mem_read(base.wrapping_add(1) as u16);
                let deref_base = (hi as u16) << 8 | (lo as u16);
                let deref = deref_base.wrapping_add(self.register_y as u16);
                deref
            },
            AddressingMode::NoneAddressing => {
                panic!("mode {:?} is not supported", mode);

            }
        }
    }
    

        pub fn load_and_run(&mut self, program: Vec<u8>){
            self.load(program);
            self.reset();
            self.run();
        }

        pub fn reset(&mut self){
            self.register_a  = 0;
            self.register_x = 0;
            self.register_y = 0;
            self.status = 0;   

            self.program_counter = self.mem_read_u16(0xFFFC);
        }

        pub fn load(&mut self, program: Vec<u8>){
            self.memory[0x8000 .. (0x8000 + program.len())].copy_from_slice(&program[..]);
            self.mem_write_u16(0xFFFC, 0x8000);
        }

        pub fn run(&mut self){

            let ref opcodes: HashMap<u8, &'static opcodes::OpCode> = *opcodes::OPCODES_MAP;
            loop {
                let code = self.mem_read(self.program_counter);
                self.program_counter += 1;  
                let program_counter_state = self.program_counter;

                let opcode = opcodes.get(&code).expect(&format!("OpCode {:x} is not recognized", code));

                match code {

                    0xAA => self.tax(),
                    

                    0xA9 | 0xA5 | 0xB5 | 0xad |0xbd | 0xb9 | 0xa1 |0xb1=> {
                        self.lda(&opcode.mode);
                    }
                    0x85 | 0x95 | 0x8d | 0x9d | 0x99 | 0x81 | 0x91 => {
                        self.sta(&opcode.mode);
                    }
                    0xe8 => {
                        
                        self.inx();
                    },
                    0x00 => return,

                    _ => todo!()
                }
                
                if program_counter_state == self.program_counter{
                    self.program_counter += (opcode.len-1) as u16;
                }
            }
        }


       
     
        fn lda(&mut self, mode: &AddressingMode){
            let addr = self.get_operand_address(mode);
            let value = self.mem_read(addr);

            self.register_a = value;
            self.update_zero_and_negative_flags(self.register_a);
        }

        fn ldx(&mut self, mode: &AddressingMode){
            let addr = self.get_operand_address(mode);
            let value = self.mem_read(addr);

            self.register_x = value;
            self.update_zero_and_negative_flags(self.register_x);

        }

        fn ldy(&mut self, mode: &AddressingMode){
            let addr = self.get_operand_address(mode);
            let value = self.mem_read(addr);

            self.register_y = value;
            self.update_zero_and_negative_flags(self.register_y);

        }

        fn tax(&mut self){
            self.register_x = self.register_a;
            self.update_zero_and_negative_flags(self.register_x);
        }

        fn inx(&mut self){
            self.register_x = self.register_x.wrapping_add(1);
            self.update_zero_and_negative_flags(self.register_x);
        }
        
        fn sta(&mut self, mode: &AddressingMode){
            let addr = self.get_operand_address(mode);
            self.mem_write(addr, self.register_a);
        }
        fn lsr(&mut self, mode: &AddressingMode) -> u8{
            let addr = self.get_operand_address(mode);
            let mut data = self.mem_read(addr);

            if data & 1 == 1{
                self.set_carry_flag();
            }
            else{
                self.clear_carry_flag();
            }

            data = data >> 1;
            self.mem_write(addr, data);

            self.update_zero_and_negative_flags(data);
            return data
        }

        fn ora(&mut self, mode: &AddressingMode){
            let addr = self.get_operand_address(mode);
            let data = self.mem_read(addr);
            self.register_a = self.register_a | data;

            self.update_zero_and_negative_flags(self.register_a);
        }

        fn pha(&mut self){
            let mut data = self.register_a;
            data = data | 0b0011_0000;
            self.stack_push(data); 
        }

        fn php(&mut self){
            let mut flags = self.status.clone();
            flags = flags | 0b0011_0000;
            self.stack_push(flags); 
        }   

        fn pla(&mut self){
            self.register_a = self.stack_pop();
            self.update_zero_and_negative_flags(self.register_a);
        }

        fn plp(&mut self){
            self.status = self.stack_pop();
            self.status = self.status & 0b1110_1111;
            self.status = self.status | 0b0010_0000;
        }

        fn adc(&mut self, mode: &AddressingMode){
            let addr = self.get_operand_address(mode);
            let mut data = self.mem_read(addr) as u16;
            let carry = self.status & 0b0000_0001; 
            data = data + carry as u16 + self.register_a as u16;

            if data > 0xff{
                self.set_carry_flag();
            }
            else {
                self.clear_carry_flag();
            }

            self.update_zero_and_negative_flags(data as u8);
            self.register_a = data as u8;
        }

        fn update_zero_and_negative_flags(&mut self, result: u8){
            if result == 0 {
                self.status = self.status | 0b0000_0010;
            }
            else {
                self.status = self.status & 0b1111_1101;
            }

            if result & 0b1000_0000 != 0 {
                self.status = self.status | 0b1000_0000;
            }
            else {
                self.status = self.status & 0b0111_1111;
            }
        }
}
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_0xa9_lda_immediate_load_data(){
        let mut cpu = CPU::new();
        //cpu.interpret(vec![0xa9,0x05, 0x00]);
        cpu.load_and_run(vec![0xa9,0x05, 0x00]);
        assert_eq!(cpu.register_a, 5);
        assert!(cpu.status & 0b0000_0010 == 0);
        assert!(cpu.status & 0b1000_0000 == 0);

    }

    #[test]
    fn test_0xa9_lda_zero_flag(){
        let mut cpu = CPU::new();
        cpu.load_and_run(vec![0xa9, 0x00, 0x00]);
        assert!(cpu.status & 0b0000_0010 == 0b10);
    }

    #[test]
    fn test_0xaa_tax_move_a_to_x(){
        let mut cpu = CPU::new();
        cpu.load_and_run(vec![0xa9, 0x0A,0xAA, 0x00]);  

        assert_eq!(cpu.register_x, 10);
    }

    #[test]
    fn test_5_ops_working_together(){
        let mut cpu = CPU::new();
        cpu.load_and_run(vec![0xa9,0xc0, 0xaa, 0xe8, 0x00 ]);

        assert_eq!(cpu.register_x, 0xc1)
    }

    #[test]
    fn test_inx_overflow(){
        let mut cpu = CPU::new();
        cpu.load_and_run(vec![0xa9, 0xff, 0xaa,0xe8, 0xe8, 0x00]);

        assert_eq!(cpu.register_x,  1);
    }

    #[test]
    fn test_lda_from_memory(){
        let mut cpu = CPU::new();
        cpu.mem_write(0x10, 0x55);
        cpu.load_and_run(vec![0xa5, 0x10, 0x00]);

        assert_eq!(cpu.register_a,  0x55);
    }
    
}