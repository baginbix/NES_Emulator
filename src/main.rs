pub mod cpu;
pub mod opcodes;
pub mod bus;
pub mod cartridge;
pub mod addr_register;
pub mod PPU;

use std::{collections::HashMap, default, fmt::format, thread::sleep, time::Duration};

use bus::Bus;
use cpu::{Mem, CPU};
use rand::{thread_rng, Rng};
use sdl3::{EventPump, event::Event, keyboard::Keycode, pixels::{Color, PixelFormat}};

#[macro_use]
extern crate lazy_static; 



fn handle_user_input(cpu: &mut CPU, event_pump: &mut EventPump){
    for event in event_pump.poll_iter(){
        match event {
            Event::Quit{..} | Event::KeyDown{keycode: Some(Keycode::Escape), ..} => {
                std::process::exit(0);
            },
            Event::KeyDown { keycode: Some(Keycode::W), .. } => {
                cpu.mem_write(0xff, 0x77);
            },
            Event::KeyDown { keycode: Some(Keycode::S), .. } => {
                cpu.mem_write(0xff, 0x73);
            },
            Event::KeyDown { keycode: Some(Keycode::A), .. } => {
                cpu.mem_write(0xff, 0x61);
            },
            Event::KeyDown { keycode: Some(Keycode::D), .. } => {
                cpu.mem_write(0xff, 0x64);
            },
            
            _=>{}
        }
    }           
} 

fn color(byte: u8) -> Color{
    match byte {
        0 => Color::BLACK,
        1 => Color::WHITE,
        2 | 9 => Color::GREY,
        3 | 10 => Color::RED,
        4 | 11 => Color::GREEN,
        5 | 12 => Color::BLUE,
        6 | 13 => Color::MAGENTA,
        7 | 14 => Color::YELLOW,
        _=> Color::CYAN
    }
}

fn read_screen_state(cpu: &CPU, frame: &mut [u8; 32*3*32]) -> bool{
    let mut frame_idx = 0;
    let mut update = false;

    for i in 0x0200..0x0600{
        let color_idx = cpu.mem_read(i as u16);
        let (b1,b2,b3) = color(color_idx).rgb();

        if frame[frame_idx] != b1 ||  frame[frame_idx + 1] != b2 || frame[frame_idx + 2] != b3{
            frame[frame_idx] = b1;
            frame[frame_idx + 1] = b2;
            frame[frame_idx + 2] = b3;
            update = true;
        }
        frame_idx += 3;
    }

    return update;
} 


fn main() {
    let sdl_context = sdl3::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
    .window("Snake Game", (32 * 10) as u32,(32 * 10) as u32)
    .position_centered()
    .build()
    .unwrap();

    let mut canvas = window.into_canvas();
    let mut event_pump = sdl_context.event_pump().unwrap();
    canvas.set_scale(10.0, 10.0).unwrap();

    let creator = canvas.texture_creator();
    let mut texture = creator.create_texture_target(PixelFormat::RGB24, 32, 32).unwrap();


    let game_code = vec![
        0x20, 0x06, 0x06, 0x20, 0x38, 0x06, 0x20, 0x0d, 0x06, 0x20, 0x2a, 0x06, 0x60, 0xa9, 0x02, 0x85,
        0x02, 0xa9, 0x04, 0x85, 0x03, 0xa9, 0x11, 0x85, 0x10, 0xa9, 0x10, 0x85, 0x12, 0xa9, 0x0f, 0x85,
        0x14, 0xa9, 0x04, 0x85, 0x11, 0x85, 0x13, 0x85, 0x15, 0x60, 0xa5, 0xfe, 0x85, 0x00, 0xa5, 0xfe,
        0x29, 0x03, 0x18, 0x69, 0x02, 0x85, 0x01, 0x60, 0x20, 0x4d, 0x06, 0x20, 0x8d, 0x06, 0x20, 0xc3,
        0x06, 0x20, 0x19, 0x07, 0x20, 0x20, 0x07, 0x20, 0x2d, 0x07, 0x4c, 0x38, 0x06, 0xa5, 0xff, 0xc9,
        0x77, 0xf0, 0x0d, 0xc9, 0x64, 0xf0, 0x14, 0xc9, 0x73, 0xf0, 0x1b, 0xc9, 0x61, 0xf0, 0x22, 0x60,
        0xa9, 0x04, 0x24, 0x02, 0xd0, 0x26, 0xa9, 0x01, 0x85, 0x02, 0x60, 0xa9, 0x08, 0x24, 0x02, 0xd0,
        0x1b, 0xa9, 0x02, 0x85, 0x02, 0x60, 0xa9, 0x01, 0x24, 0x02, 0xd0, 0x10, 0xa9, 0x04, 0x85, 0x02,
        0x60, 0xa9, 0x02, 0x24, 0x02, 0xd0, 0x05, 0xa9, 0x08, 0x85, 0x02, 0x60, 0x60, 0x20, 0x94, 0x06,
        0x20, 0xa8, 0x06, 0x60, 0xa5, 0x00, 0xc5, 0x10, 0xd0, 0x0d, 0xa5, 0x01, 0xc5, 0x11, 0xd0, 0x07,
        0xe6, 0x03, 0xe6, 0x03, 0x20, 0x2a, 0x06, 0x60, 0xa2, 0x02, 0xb5, 0x10, 0xc5, 0x10, 0xd0, 0x06,
        0xb5, 0x11, 0xc5, 0x11, 0xf0, 0x09, 0xe8, 0xe8, 0xe4, 0x03, 0xf0, 0x06, 0x4c, 0xaa, 0x06, 0x4c,
        0x35, 0x07, 0x60, 0xa6, 0x03, 0xca, 0x8a, 0xb5, 0x10, 0x95, 0x12, 0xca, 0x10, 0xf9, 0xa5, 0x02,
        0x4a, 0xb0, 0x09, 0x4a, 0xb0, 0x19, 0x4a, 0xb0, 0x1f, 0x4a, 0xb0, 0x2f, 0xa5, 0x10, 0x38, 0xe9,
        0x20, 0x85, 0x10, 0x90, 0x01, 0x60, 0xc6, 0x11, 0xa9, 0x01, 0xc5, 0x11, 0xf0, 0x28, 0x60, 0xe6,
        0x10, 0xa9, 0x1f, 0x24, 0x10, 0xf0, 0x1f, 0x60, 0xa5, 0x10, 0x18, 0x69, 0x20, 0x85, 0x10, 0xb0,
        0x01, 0x60, 0xe6, 0x11, 0xa9, 0x06, 0xc5, 0x11, 0xf0, 0x0c, 0x60, 0xc6, 0x10, 0xa5, 0x10, 0x29,
        0x1f, 0xc9, 0x1f, 0xf0, 0x01, 0x60, 0x4c, 0x35, 0x07, 0xa0, 0x00, 0xa5, 0xfe, 0x91, 0x00, 0x60,
        0xa6, 0x03, 0xa9, 0x00, 0x81, 0x10, 0xa2, 0x00, 0xa9, 0x01, 0x81, 0x10, 0x60, 0xa2, 0x00, 0xea,
        0xea, 0xca, 0xd0, 0xfb, 0x60
    ];

    let rom = cartridge::test::test_rom(game_code);
    let mut cpu = CPU::new(Bus::new(rom));

    let mut screen_state = [0 as u8; 32*3*32];
    let mut rng = thread_rng();
    cpu.run_with_callback(move |cpu|{
        /*
        handle_user_input(cpu, &mut event_pump);
        cpu.mem_write(0xfe, rng.gen_range(1, 16));

        if read_screen_state(cpu, &mut screen_state){
            texture.update(None, &screen_state, 32*3).unwrap();
            canvas.copy(&texture, None, None).unwrap();
            canvas.present();
        }

        sleep(Duration::new(0, 70_000));

        */

       println!("{}", trace(cpu));
    });
}

fn trace(cpu: &CPU) -> String{
    let ref opcodes: HashMap<u8, &'static opcodes::OpCode> = *opcodes::OPCODES_MAP;
    let code = cpu.mem_read(cpu.program_counter);
    let opcode = opcodes.get(&code).unwrap();
    
    let mut output = "".to_string();
    output.push_str(format!("{:04X}  ",cpu .program_counter).as_str());
    output.push_str(format!("{code:X}").as_str());
    output.push_str(opcode.mnemonic);
    //let addr = cpu.get_operand_address(&opcode.mode);
    
    String::from(output) 
}


#[cfg(test)]
mod test {
   use super::*;
   use crate::bus::Bus;
   use crate::cartridge::test::test_rom;

   #[test]
   fn test_format_trace() {
       let mut bus = Bus::new(test_rom(vec![0x00]));
       bus.mem_write(100, 0xa2);
       bus.mem_write(101, 0x01);
       bus.mem_write(102, 0xca);
       bus.mem_write(103, 0x88);
       bus.mem_write(104, 0x00);

       let mut cpu = CPU::new(bus);
       cpu.program_counter = 0x64;
       cpu.register_a = 1;
       cpu.register_x = 2;
       cpu.register_y = 3;
       let mut result: Vec<String> = vec![];
       cpu.run_with_callback(|cpu| {
           result.push(trace(cpu));
       });
       assert_eq!(
           "0064  A2 01     LDX #$01                        A:01 X:02 Y:03 P:24 SP:FD",
           result[0]
       );
       assert_eq!(
           "0066  CA        DEX                             A:01 X:01 Y:03 P:24 SP:FD",
           result[1]
       );
       assert_eq!(
           "0067  88        DEY                             A:01 X:00 Y:03 P:26 SP:FD",
           result[2]
       );
   }

   #[test]
   fn test_format_mem_access() {
       let mut bus = Bus::new(test_rom(vec![0x00]));
       // ORA ($33), Y
       bus.mem_write(100, 0x11);
       bus.mem_write(101, 0x33);


       //data
       bus.mem_write(0x33, 00);
       bus.mem_write(0x34, 04);

       //target cell
       bus.mem_write(0x400, 0xAA);

       let mut cpu = CPU::new(bus);
       cpu.program_counter = 0x64;
       cpu.register_y = 0;
       let mut result: Vec<String> = vec![];
       cpu.run_with_callback(|cpu| {
           result.push(trace(cpu));
       });
       assert_eq!(
           "0064  11 33     ORA ($33),Y = 0400 @ 0400 = AA  A:00 X:00 Y:00 P:24 SP:FD",
           result[0]
       );
   }
}