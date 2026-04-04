use std::collections::HashMap;
use crate::{bus::Bus, opcodes};

const STACK: u16 = 0x0100;
const STACK_RESET: u8 = 0xfd;

pub struct CPU{
    pub register_a: u8,
    pub register_x: u8,
    pub register_y: u8,
    
    /// # Status Register (P) http://wiki.nesdev.com/w/index.php/Status_flags
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
    //memory: [u8; 0xffff],
    pub bus: Bus
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

pub trait  Mem {
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
        self.bus.mem_read(addr)
    }

    fn mem_read_u16(&self, pos: u16) -> u16 {
        self.bus.mem_read_u16(pos)
    }

    fn mem_write(&mut self, addr: u16, data: u8){
        self.bus.mem_write(addr, data);
    }

    fn mem_write_u16(&mut self, pos: u16, data: u16) {
        self.bus.mem_write_u16(pos, data);
    }
}



impl CPU{
    pub fn new(bus: Bus) -> Self{
        CPU{
            register_a: 0,
            status: 0b100100,
            program_counter: 0,
            register_x: 0,
            register_y: 0,
            stack_pointer: 0xff,
            bus: bus
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


    fn stack_push_u16(&mut self, data: u16){
        let hi = (data >> 8) as u8;
        let lo = (data & 0xff) as u8;
        self.stack_push(hi);
        self.stack_push(lo);
    }
    
    fn stack_pop(&mut self) -> u8{
        self.stack_pointer = self.stack_pointer.wrapping_add(1);
        return self.mem_read(self.stack_pointer as u16 + STACK);
    }

    fn stack_pop_u16(&mut self) -> u16{
        let lo = self.stack_pop() as u16;
        let hi = self.stack_pop() as u16;

        return  hi <<8 | lo
    }

    pub fn get_operand_address(&self, mode: &AddressingMode) -> u16{
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
            self.status = 0b100100;
            self.stack_pointer = STACK_RESET;   

            self.program_counter = self.mem_read_u16(0xFFFC);
        }

        pub fn load(&mut self, program: Vec<u8>){
            for i in 0..(program.len() as u16){
                self.mem_write(0x0600 + i, program[i as usize]);
            }
            self.mem_write_u16(0xFFFC, 0x0600);
        }

        pub fn run(&mut self){
            self.run_with_callback(|_| {});
        }

        pub fn run_with_callback<F>(&mut self, mut callback: F)
        where
            F: FnMut(&mut CPU)
        {

            let ref opcodes: HashMap<u8, &'static opcodes::OpCode> = *opcodes::OPCODES_MAP;

            loop {
                callback(self);
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

                    0x69 | 0x65 | 0x75 | 0x6d | 0x7d | 0x79 | 0x61 | 0x71 =>{
                        self.adc(&opcode.mode);
                    },
                    0x29 | 0x25 | 0x35 | 0x2d | 0x3d | 0x39 | 0x21 | 0x31 =>{
                        self.and(&opcode.mode);
                    },
                    0x0a | 0x06 | 0x16 | 0x0e | 0x1e => {
                        self.asl(&opcode.mode);
                    },
                    0x90 => self.bcc(),
                    0xb0 => self.bcs(),
                    0xf0 => self.beq(),
                    0x24 | 0x2c =>{
                        self.bit(&opcode.mode);
                    },
                    0x30 => self.bmi(),
                    0xd0 => self.bne(),
                    0x10 => self.bpl(),
                    0x50 => self.bvc(),
                    0x70 => self.bvs(),
                    0x18 => self.clear_carry_flag(),
                    0xd8 =>self.cld(),
                    0x58 => self.cli(),
                    0xb8 => self.clv(),
                    0xc9 | 0xc5 | 0xd5 | 0xcd | 0xdd | 0xd9 | 0xc1 | 0xd1 => {
                        self.cmp(&opcode.mode);
                    },
                    0xe0 | 0xe4 | 0xec =>{
                        self.cpx(&opcode.mode);
                    },
                    0xc0 | 0xc4 | 0xcc =>{
                        self.cpy(&opcode.mode);
                    },
                    0xc6 |0xD6 | 0xce | 0xde => {
                        self.dec(&opcode.mode);
                    },
                    0xca => self.dex(),
                    0x88 => self.dey(),
                    0x49 |0x45| 0x55 | 0x4d | 0x5d | 0x59 | 0x41 | 0x51 => {
                        self.eor(&opcode.mode);
                    },
                    0xe6 | 0xf6 | 0xee | 0xfe =>{
                        self.inc(&opcode.mode);
                    },
                    0xc8 => self.iny(),
                    0x4c =>{
                        self.jmp_absolute();
                    },
                    0x6c =>{
                        self.jmp_indirect();
                    },
                    0x20 => self.jsr(),
                    0xa2 | 0xa6 | 0xb6 | 0xae | 0xbe =>{
                        self.ldx(&opcode.mode);
                    },
                    0xa0 | 0xa4 | 0xb4 | 0xac | 0xbc => {
                        self.ldy(&opcode.mode);
                    },
                    0x4a => self.lsr_accumualtor(),
                    0x46 | 0x56 | 0x4e | 0x53 => {
                        self.lsr(&opcode.mode);
                    } ,
                    
                    0xea =>{
                        //NOP
                        //do nothing
                    },
                    0x09 | 0x05 | 0x15 | 0x0d | 0x1d | 0x19 | 0x01 | 0x11 => {
                        self.ora(&opcode.mode);
                    },
                    0x48 => {
                        self.pha();
                    },
                    0x08 => {
                        self.php();
                    },
                    0x68 => {
                        self.pla();
                    },
                    0x28 => {
                        self.plp();
                    },
                    0x2a | 0x26 | 0x36 | 0x2e | 0x3e => {
                        self.rol(&opcode.mode);
                    },
                    0x6a | 0x66 | 0x76 | 0x6e | 0x7e => {
                        self.ror(&opcode.mode);
                    }
                    0x40 => self.rti(),
                    0x60 => self.rts(),
                    0xe9 | 0xe5 | 0xf5 | 0xed | 0xfd | 0xf9 | 0xe1 | 0xf1 => {
                        self.sbc(&opcode.mode);
                    }
                    0x38 => self.set_carry_flag(),
                    0xf8 => self.sed(),
                    0x78 => self.sei(),
                    0x86 | 0x96 | 0x8e => {
                        self.stx(&opcode.mode);
                    },
                    0x84 |0x94 | 0x8c => { 
                        self.sty(&opcode.mode);
                    },
                    0xa8 => self.tay(),
                    0xba => self.tsx(),
                    0x8a => self.txa(),
                    0x9a => self.txs(),
                    0x98 => self.tya(), 
                    0x00 => return,

                    _ => todo!()
                }
                
                if program_counter_state == self.program_counter{
                    self.program_counter += (opcode.len-1) as u16;
                }
            }
        }


       fn branch(&mut self, condition: bool){
            if condition{
                let data = self.mem_read(self.program_counter) as i8;
                let jump_addr = self.program_counter.wrapping_add(1).wrapping_add(data as u16);
                self.program_counter = jump_addr;
            }
       }

       fn compare(&mut self, mode: &AddressingMode, compare_with: u8){
        let addr = self.get_operand_address(mode);
        let data = self.mem_read(addr);

        if compare_with >= data{
            self.set_carry_flag();
        }
        else{
            self.clear_carry_flag();
        }

        self.update_zero_and_negative_flags(compare_with.wrapping_sub(data));

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

        fn tay(&mut self){
            self.register_y = self.register_a;
            self.update_zero_and_negative_flags(self.register_y);
        }

        fn tsx(&mut self){
            self.register_x = self.stack_pointer;
            self.update_zero_and_negative_flags(self.register_x);
        }

        fn txa(&mut self){
            self.register_a = self.register_x;
            self.update_zero_and_negative_flags(self.register_a);
        }

        fn txs(&mut self){
            self.stack_pointer = self.register_x;
        }

        fn tya(&mut self){
            self.register_a = self.register_y;
            self.update_zero_and_negative_flags(self.register_a);
        }

        fn inx(&mut self){
            self.register_x = self.register_x.wrapping_add(1);
            self.update_zero_and_negative_flags(self.register_x);
        }
        
        fn sta(&mut self, mode: &AddressingMode){
            let addr = self.get_operand_address(mode);
            self.mem_write(addr, self.register_a);
        }
        fn lsr_accumualtor(&mut self){
            let mut data = self.register_a;

            if data & 1 == 1{
                self.set_carry_flag();
            }
            else{
                self.clear_carry_flag();
            }

            data = data >> 1;
            self.register_a = data;

            self.update_zero_and_negative_flags(data);
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

            
            
            let result = data as u8;
            if (data ^ result as u16) & (result ^ self.register_a) as u16 & 0x80 != 0 {
                self.status = self.status | 0b0100_0000;
            } else {
                self.status = self.status & 0b1011_1111;
            } 
            
            self.register_a = result;
            self.update_zero_and_negative_flags(self.register_a);
        }

        fn and(&mut self, mode: &AddressingMode){
            let addr = self.get_operand_address(mode);
            let data = self.mem_read(addr);

            self.register_a = self.register_a & data;

            self.update_zero_and_negative_flags(self.register_a);
        }

        fn asl(&mut self, mode: &AddressingMode) -> u8{
            let addr = self.get_operand_address(mode);
            let mut  data = self.mem_read(addr) ;
            
            
            if data >> 7 == 1{
                self.set_carry_flag();
            }
            else{
                self.clear_carry_flag();
            }
            
            data = data << 1;
            
            self.mem_write(addr, data);
            self.update_zero_and_negative_flags(data);

            return data
        }

        fn bcc(&mut self){
            self.branch(self.status & 0b0000_0001 == 0);
        }

        fn bcs(&mut self){
            self.branch(self.status & 0b0000_0001 == 1)
        }

        fn beq(&mut self){
            self.branch(self.status & 0b0000_0010 == 2);
        }

        fn bit(&mut self, mode: &AddressingMode){
            let addr = self.get_operand_address(mode);
            let data = self.mem_read(addr);

            let result = self.register_a & data;
            if result == 0{
                self.register_a = self.register_a | 0b0000_0001;
            }
            else{
                self.register_a = self.register_a & 0b1111_1110;
            }
            if result & 0b0100_0000 == 0b0100_0000{
                self.register_a = self.register_a | 0b0100_0000;
            }
            if result & 0b1000_0000 == 0b1000_0000{

                self.register_a = self.register_a | 0b1000_0000;
            }

            
        }

        fn bmi(&mut self){
            self.branch(self.status & 0b1000_0000 != 0)
        }

        fn bne(&mut self){
            self.branch(self.status & 0b0000_0010 == 0)
        }

        fn bpl(&mut self){
            self.branch(self.status & 0b1000_0000 == 0);
        }

        fn bvc(&mut self){
            self.branch(self.status & 0b0100_0000 == 0);
        }

        fn bvs(&mut self){
            self.branch(self.status & 0b0100_0000 != 0);

        }

        fn cld(&mut self){
            self.register_a = self.register_a & 0b1111_0111;
        }

        fn cli(&mut self){
            self.register_a = self.register_a & 0b1111_1011;
        }

        fn clv(&mut self){
            self.register_a = self.register_a & 0b1011_1111;
        }

        fn cmp(&mut self, mode: &AddressingMode){
            self.compare(mode, self.register_a);
        }

        fn cpx(&mut self, mode: &AddressingMode){
            self.compare(mode, self.register_x);
        }

        fn cpy(&mut self, mode: &AddressingMode){
            self.compare(mode, self.register_y);
        }

        fn dec(&mut self, mode: &AddressingMode) -> u8{
            let addr = self.get_operand_address(mode);
            let data = self.mem_read(addr);
            let result = data.wrapping_sub(1);

            self.mem_write(addr, result);

            self.update_zero_and_negative_flags(result);
            return data;
        }

        fn dex(&mut self){
            self.register_x = self.register_x.wrapping_sub(1);
            self.update_zero_and_negative_flags(self.register_x);
        }
        
        fn dey(&mut self){
            self.register_y = self.register_y.wrapping_sub(1);
            self.update_zero_and_negative_flags(self.register_y);

        }
        

        fn eor(&mut self, mode: &AddressingMode){
            let addr = self.get_operand_address(mode);
            let data = self.mem_read(addr);
            self.register_a ^= data;

            self.update_zero_and_negative_flags(self.register_a);
        }


        fn inc(&mut self, mode: &AddressingMode) -> u8{
            let addr = self.get_operand_address(mode);
            let data = self.mem_read(addr).wrapping_add(1);
            self.mem_write(addr, data);
            self.update_zero_and_negative_flags(data);
            return data
        }
        
        fn iny(&mut self){
            self.register_y = self.register_y.wrapping_add(1);
            self.update_zero_and_negative_flags(self.register_y);
        }

        fn jmp_absolute(&mut self){
            let data = self.mem_read_u16(self.program_counter);
            self.program_counter = data;
        }

        fn jmp_indirect(&mut self){
            let addr = self.mem_read_u16(self.program_counter);
            
            let indirect_ref = if addr & 0x00FF == 0x00FF{
                let lo = self.mem_read(addr);
                let hi = self.mem_read(addr & 0xFF00);
                (hi as u16) << 8 | (lo as u16)
            }
            else {
                self.mem_read_u16(addr)
            };

            self.program_counter = indirect_ref;
        }

        fn jsr(&mut self){
            self.stack_push_u16(self.program_counter as u16  + 2 -1 );
            let target_addr = self.mem_read_u16(self.program_counter);
           
            self.program_counter = target_addr;
        }

        fn update_zero_and_negative_flags(&mut self, result: u8){
            if result == 0 {
                self.status = self.status | 0b0000_0010;
            }
            else {
                self.status = self.status & 0b1111_1101;
            }

            if result & 0b1000_0000 != 0{
                self.status = self.status | 0b1000_0000;
            }
            else {
                self.status = self.status & 0b0111_1111;
            }
        }

        fn rol(&mut self, mode: &AddressingMode) -> u8{
            let addr = self.get_operand_address(mode);
            let mut data = self.mem_read(addr);
            let carry = self.status & 0x1;

            if data >> 7 == 1{
                self.set_carry_flag();
            }
            else{
                self.clear_carry_flag();
            }
            data = (data << 1) + carry;
            self.mem_write(addr, data);
            self.update_zero_and_negative_flags(data);
            return data
        }

        fn ror(&mut self, mode: &AddressingMode) -> u8{
            let addr = self.get_operand_address(mode);
            let mut data = self.mem_read(addr);
            let carry = self.status & 0x1;

            if data & 1 == 1{
                self.set_carry_flag();
            }
            else{
                self.clear_carry_flag();
            }
            data = (data >> 1) | (carry << 7);
            self.mem_write(addr, data);
            self.update_zero_and_negative_flags(data);
            return data
        }

        fn rti(&mut self){
            self.status = self.stack_pop();
            self.status &= 0b1101_1111;
            self.status |= 0b0001_0000;
        }


        fn rts(&mut self){
            self.program_counter = self.stack_pop_u16() + 1;
        }

        fn sbc(&mut self, mode: &AddressingMode){
            let addr = self.get_operand_address(mode);
            let mut data = self.mem_read(addr) as u16;
            data = (data as i8).wrapping_neg().wrapping_sub(1) as u16;
            let carry = self.status & 0b0000_0001;
            data = data + carry as u16 + self.register_a as u16;

            if data > 0xff{
                self.set_carry_flag();
            }
            else {
                self.clear_carry_flag();
            }
            

            let result = data as u8;
            if (data ^ result as u16) & (result ^ self.register_a) as u16 & 0x80 != 0 {
                self.status = self.status | 0b0100_0000;
            } else {
                self.status = self.status & 0b1011_1111;
            } 

            self.register_a = result;
            self.update_zero_and_negative_flags(self.register_a);
        }

        fn sed(&mut self){
            self.status |= 0b0000_1000;
        }
        fn sei(&mut self){
            self.status |= 0b0000_0100;
        }

        fn stx(&mut self, mode: &AddressingMode){
            let addr = self.get_operand_address(mode);
            let data = self.mem_read(addr);
            self.register_x = data;
        }

        fn sty(&mut self, mode: &AddressingMode){
            let addr = self.get_operand_address(mode);
            let data = self.mem_read(addr);
            self.register_y = data;
        }
}
#[cfg(test)]
mod test {
    use crate::cartridge::{self, Rom};

    use super::*;

    #[test]
    fn test_0xa9_lda_immediate_load_data(){
        let rom = cartridge::test::test_rom();
        let mut cpu = CPU::new(Bus::new(rom));
        cpu.load_and_run(vec![0xa9,0x05, 0x00]);
        assert_eq!(cpu.register_a, 5);
        assert!(cpu.status & 0b0000_0010 == 0);
        assert!(cpu.status & 0b1000_0000 == 0);

    }

    #[test]
    fn test_0xa9_lda_zero_flag(){
        let rom = cartridge::test::test_rom();
        let mut cpu = CPU::new(Bus::new(rom));
        cpu.load_and_run(vec![0xa9, 0x00, 0x00]);
        assert!(cpu.status & 0b0000_0010 == 0b10);
    }

    #[test]
    fn test_0xaa_tax_move_a_to_x(){
        let rom = cartridge::test::test_rom();
        let mut cpu = CPU::new(Bus::new(rom));
        cpu.load_and_run(vec![0xa9, 0x0A,0xAA, 0x00]);  

        assert_eq!(cpu.register_x, 10);
    }

    #[test]
    fn test_5_ops_working_together(){
        let rom = cartridge::test::test_rom();
        let mut cpu = CPU::new(Bus::new(rom));
        cpu.load_and_run(vec![0xa9,0xc0, 0xaa, 0xe8, 0x00 ]);

        assert_eq!(cpu.register_x, 0xc1)
    }

    #[test]
    fn test_inx_overflow(){
        let rom = cartridge::test::test_rom();
        let mut cpu = CPU::new(Bus::new(rom));
        cpu.load_and_run(vec![0xa9, 0xff, 0xaa,0xe8, 0xe8, 0x00]);

        assert_eq!(cpu.register_x,  1);
    }

    #[test]
    fn test_lda_from_memory(){
        let rom = cartridge::test::test_rom();
        let mut cpu = CPU::new(Bus::new(rom));
        cpu.mem_write(0x10, 0x55);
        cpu.load_and_run(vec![0xa5, 0x10, 0x00]);

        assert_eq!(cpu.register_a,  0x55);
    }
    #[test]
    fn test_adc_immediate(){
        let rom = cartridge::test::test_rom();
        let mut cpu = CPU::new(Bus::new(rom));

        cpu.load_and_run(vec![0x69, 0x10, 0x00 ]);

        assert_eq!(cpu.register_a, 0x10);
    }

    #[test]
    fn test_adc_immediate_carry_set(){
        let rom = cartridge::test::test_rom();
        let mut cpu = CPU::new(Bus::new(rom));
        cpu.mem_write(0x10,0xff);
        cpu.load_and_run(vec![0xa5, 0x10,0x69, 0x01, 0x00 ]);
        let carry_flag = cpu.status & 0x0000_0001;
        assert_eq!(carry_flag , 0x1);
    }
    
}
