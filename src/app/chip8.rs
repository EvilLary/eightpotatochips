
#[derive(Debug)]
pub struct Chip8 {
    pub(crate) opcode: u16,
    pub(crate) program_counter: usize,
    pub(crate) index: usize,

    pub(crate) stack_pointer: usize,
    pub(crate) stack: [usize; 16],
    pub(crate) registers: [u8; 16],
    pub(crate) memory: [u8; 4096],
    pub(crate) framebuffer: [u8; WIDTH * HEIGHT],

    pub(crate) sound_timer: u8,
    pub(crate) delay_timer: u8,
    pub(crate) keys: [bool; 16],
    pub(crate) wait_for_keys: bool,
    pub need_redraw: bool,
}

impl Chip8 {
    pub fn load(&mut self, rom: &str) -> std::io::Result<()> {
        let mut rom = std::fs::File::open(rom)?;
        self.memory[..80].copy_from_slice(&FONTSET_1);
        let len = std::io::Read::read(&mut rom, &mut self.memory[0x200..])?;
        Ok(())
    }
    pub fn new() -> Self {
        let mut memory = [0u8; MEMORY_SIZE];
        memory[..80].copy_from_slice(&FONTSET_1);
        Self {
            memory,
            opcode: 0,
            program_counter: 0x200,
            index: 0,
            stack_pointer: 0,
            stack: [0; 16],
            registers: [0; 16],
            framebuffer: [0; WIDTH * HEIGHT],
            sound_timer: 0,
            delay_timer: 0,
            keys: [false; 16],
            wait_for_keys: false,
            need_redraw: false
        }
    }

    // x & y, are always used to index into `registers`
    // I might as well just promote them to `usize` here.
    // Y or the high nipple of the low byte
    fn get_y(&self) -> usize {
        // ----|
        //     |
        // 0xixYn
        ((self.opcode & 0x00F0) >> 4) as usize
    }

    // X or the low nipple of the high byte
    fn get_x(&self) -> usize {
        ((self.opcode & 0x0F00) >> 8) as usize
    }

    // NNN or addr = low nipple of high byte &  low byte
    fn get_addr(&self) -> usize {
        (self.opcode & 0x0FFF) as usize
    }

    // KK or low byte
    fn get_kk(&self) -> u8 {
        (self.opcode & 0x00FF) as u8
    }

    fn get_nipple(&self) -> u8 {
        (self.opcode & 0x000F) as u8
    }
    pub fn update_opcode(&mut self) {
        // 0000_0000_0000_0000
        self.opcode = (self.memory[self.program_counter] as u16) << 8
            | (self.memory[self.program_counter + 1] as u16)
    }
    pub fn cycle(&mut self) {
        self.update_opcode();

        let identifier = (self.opcode & 0xF000) >> 12;
        match identifier {
            0x0 => match self.get_kk() {
                0xE0 => self.cls(),
                0xEE => self.ret(),
                _ => panic!("Unknown or unimplemented insruction: 0x{:x}", self.opcode),
            },
            0x1 => self.jp_addr(),
            0x2 => self.call_addr(),
            0x3 => self.se_vxkk(),
            0x4 => self.sne_vxkk(),
            0x5 => self.se_vxvy(),
            0x6 => self.ld_vxkk(),
            0x7 => self.add_vxkk(),
            0x8 => match self.get_nipple() {
                0x0 => self.ld_vxvy(),
                0x1 => self.or_vxvy(),
                0x2 => self.and_vxvy(),
                0x3 => self.xor_vxvy(),
                0x4 => self.add_vxvy(),
                0x5 => self.sub_vxvy(),
                0x6 => self.shr_vx(),
                0x7 => self.subn_vxvy(),
                0xE => self.shl_vx(),
                _ => panic!("Unknown or unimplemented insruction: 0x{:x}", self.opcode),
            },
            0x9 => self.sne_vxvy(),
            0xA => self.ld_iaddr(),
            0xB => self.jp_v0addr(),
            0xC => self.rnd_xkk(),
            0xD => self.drw_xyn(),
            0xE => match self.get_kk() {
                0x9E => self.skp_vx(),
                0xA1 => self.sknp_vx(),
                _ => panic!("Unknown or unimplemented insruction: 0x{:x}", self.opcode),
            },
            0xF => match self.get_kk() {
                0x07 => self.ld_vxdt(),
                0x0A => self.ld_vxk(),
                0x15 => self.ld_dtvx(),
                0x18 => self.ld_stvx(),
                0x1E => self.add_ivx(),
                0x29 => self.ld_fvx(),
                0x33 => self.ld_bvx(),
                0x55 => self.ld_ivx(),
                0x65 => self.ld_vxi(),
                _ => panic!("Unknown or unimplemented insruction: 0x{:0x}", self.opcode),
            },
            _ => panic!("Unknown or unimplemented insruction: 0x{:0x}", self.opcode),
        }
        if !self.wait_for_keys {
            if self.delay_timer > 0 {
                self.delay_timer -= 1
            }
            if self.sound_timer > 0 {
                self.sound_timer -= 1
            }
        }
    }
    fn inc_pc(&mut self) {
        self.program_counter += 2;
    }
}

// Instructions
impl Chip8 {
    // 0x00E0
    fn cls(&mut self) {
        self.framebuffer = [0; WIDTH * HEIGHT];
        self.inc_pc();
    }
    // 0x00EE
    fn ret(&mut self) {
        self.stack_pointer -= 1;
        self.program_counter = self.stack[self.stack_pointer] + 2;
    }
    // 0x1nnn
    fn jp_addr(&mut self) {
        let addr = self.get_addr();
        self.program_counter = addr;
    }
    // 0x2nnn
    fn call_addr(&mut self) {
        let addr = self.get_addr();
        self.stack[self.stack_pointer] = self.program_counter;
        self.stack_pointer += 1;
        self.program_counter = addr;
    }
    // 0x3xkk
    fn se_vxkk(&mut self) {
        let x = self.get_x();
        let byte = self.get_kk();
        if self.registers[x] == byte {
            self.inc_pc();
        }
        self.inc_pc();
    }
    // 0x4xkk
    fn sne_vxkk(&mut self) {
        let x = self.get_x();
        let byte = self.get_kk();
        if self.registers[x] != byte {
            self.inc_pc();
        }
        self.inc_pc();
    }
    // 0x5xy0
    fn se_vxvy(&mut self) {
        let x = self.get_x();
        let y = self.get_y();
        if self.registers[x] == self.registers[y] {
            self.inc_pc();
        }
        self.inc_pc();
    }
    // 0x6xkk
    fn ld_vxkk(&mut self) {
        let x = self.get_x();
        let byte = self.get_kk();
        self.registers[x] = byte;
        self.inc_pc();
    }
    // 0x7xkk
    fn add_vxkk(&mut self) {
        let x = self.get_x();
        let byte = self.get_kk();
        self.registers[x] = ((self.registers[x] as u16) + (byte as u16)) as u8;
        self.inc_pc();
    }
    // 0x8xy0
    fn ld_vxvy(&mut self) {
        let x = self.get_x();
        let y = self.get_y();
        self.registers[x] = self.registers[y];
        self.inc_pc();
    }
    // 0x8xy1
    fn or_vxvy(&mut self) {
        let x = self.get_x();
        let y = self.get_y();
        self.registers[x] |= self.registers[y];
        self.inc_pc();
    }
    // 0x8xy2
    fn and_vxvy(&mut self) {
        let x = self.get_x();
        let y = self.get_y();
        self.registers[x] &= self.registers[y];
        self.inc_pc();
    }
    // 0x8xy3
    fn xor_vxvy(&mut self) {
        let x = self.get_x();
        let y = self.get_y();
        self.registers[x] ^= self.registers[y];
        self.inc_pc();
    }
    // 0x8xy4
    fn add_vxvy(&mut self) {
        let x = self.get_x();
        let y = self.get_y();
        let vx = self.registers[x] as u16;
        let vy = self.registers[y] as u16;
        let result = vx + vy;
        self.registers[x] = result as u8;
        self.registers[0xF] = if result > 255 { 1 } else { 0 };
        self.inc_pc();
    }
    // 0x8xy5
    fn sub_vxvy(&mut self) {
        let x = self.get_x();
        let y = self.get_y();
        let vx = self.registers[x];
        let vy = self.registers[y];
        self.registers[0xF] = if vx > vy { 1 } else { 0 };
        self.registers[x] = vx.wrapping_sub(vy);
        self.inc_pc();
    }
    // 0x8xy6
    fn shr_vx(&mut self) {
        let x = self.get_x();
        let lsb = self.registers[x] & 0b0000_0001;
        self.registers[0xF] = lsb;
        self.registers[x] >>= 1;
        self.inc_pc();
    }
    // 0x8xy7
    fn subn_vxvy(&mut self) {
        let x = self.get_x();
        let y = self.get_y();
        let vx = self.registers[x];
        let vy = self.registers[y];
        self.registers[0xF] = if vy > vx { 1 } else { 0 };
        self.registers[x] = vy.wrapping_sub(vx);
        self.inc_pc();
    }
    // 0x8xyE
    fn shl_vx(&mut self) {
        let x = self.get_x();
        let msb = (self.registers[x] & 0b1000_0000) >> 7;
        self.registers[0xF] = msb;
        self.registers[x] <<= 1;
        self.inc_pc();
    }
    // 0x9xy0
    fn sne_vxvy(&mut self) {
        let x = self.get_x();
        let y = self.get_y();
        if self.registers[x] != self.registers[y] {
            self.inc_pc();
        }
        self.inc_pc();
    }
    // 0xAnnn
    fn ld_iaddr(&mut self) {
        self.index = self.get_addr();
        self.inc_pc();
    }
    // 0xBnnn
    fn jp_v0addr(&mut self) {
        self.program_counter = self.get_addr() + (self.registers[0] as usize);
    }
    // 0xCxkk
    fn rnd_xkk(&mut self) {
        let x = self.get_x();
        let byte = self.get_kk();
        let rand = unsafe {
            (libc::rand() % 255) as u8
        };
        // self.rng
        //     .read_exact(&mut buf)
        //     .expect("couldn't read a byte from /dev/urandom");
        self.registers[x] = rand & byte;
        self.inc_pc();
    }
    // 0xDxyn
    // shamelessly copied from https://github.com/starrhorne/chip8-rust
    // Can't be bothred
    fn drw_xyn(&mut self) {
        let n = self.get_nipple() as usize;
        let x = self.get_x();
        let y = self.get_y();
        for byte in 0..n {
            let y = (self.registers[y] as usize + byte) % HEIGHT;
            for bit in 0..8 {
                let x = (self.registers[x] as usize + bit) % WIDTH;
                let color = (self.memory[self.index + byte] >> (7 - bit)) & 1;
                self.registers[0xF] |= color & self.framebuffer[y * WIDTH + x];
                self.framebuffer[y * WIDTH + x] ^= color;
            }
        }
        self.need_redraw = true;
        self.inc_pc();
    }
    // Ex9E
    fn skp_vx(&mut self) {
        if self.keys[self.registers[self.get_x()] as usize] {
            self.inc_pc();
        }
        self.inc_pc();
    }
    // ExA1
    fn sknp_vx(&mut self) {
        if !self.keys[self.registers[self.get_x()] as usize] {
            self.inc_pc();
        }
        self.inc_pc();
    }
    // 0xFx07
    fn ld_vxdt(&mut self) {
        self.registers[self.get_x()] = self.delay_timer;
        self.inc_pc();
    }
    // 0xFx0A
    fn ld_vxk(&mut self) {
        let mut pressed = false;
        self.wait_for_keys = true;
        for (index, key) in self.keys.iter().enumerate() {
            if *key {
                self.registers[self.get_x()] = index as u8;
                pressed = true;
                self.wait_for_keys = false;
                break;
            }
        }
        if pressed {
            self.inc_pc();
        }
    }
    // 0xFx15
    fn ld_dtvx(&mut self) {
        self.delay_timer = self.registers[self.get_x()];
        self.inc_pc();
    }
    // 0xFx18
    fn ld_stvx(&mut self) {
        self.sound_timer = self.registers[self.get_x()];
        self.inc_pc();
    }
    // 0xFx1E
    fn add_ivx(&mut self) {
        self.index += self.registers[self.get_x()] as usize;
        self.registers[0xF] = if self.index > 0x0F00 { 1 } else { 0 };
        self.inc_pc();
    }
    // 0xFx29
    fn ld_fvx(&mut self) {
        self.index = (self.registers[self.get_x()] as usize) * 5;
        self.inc_pc();
    }
    // 0xFx33
    fn ld_bvx(&mut self) {
        let vx = self.registers[self.get_x()];
        self.memory[self.index] = vx / 100;
        self.memory[self.index + 1] = (vx / 10) % 10;
        self.memory[self.index + 2] = vx % 10;
        self.inc_pc();
    }
    // 0xFx55
    fn ld_ivx(&mut self) {
        for i in 0..=self.get_x() {
            self.memory[self.index + i] = self.registers[i];
        }
        self.inc_pc();
    }
    // 0xFx65
    fn ld_vxi(&mut self) {
        for i in 0..=self.get_x() {
            self.registers[i] = self.memory[self.index + i];
        }
        self.inc_pc();
    }
}
const FONTSET_1: [u8; 80] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];

pub const WIDTH: usize = 64;
pub const HEIGHT: usize = 32;
pub const SCALE: usize = 10;
pub const MEMORY_SIZE: usize = 4096;

use super::*;
