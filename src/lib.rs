#[macro_use]
mod log;

pub struct Chonkpu<'a> {
    pub regs: [u8; 7],
    pub pc: u8,
    pub ram: [u8; 16],
    pub rom: &'a [u16; 256],
    ports: [Port; 2],

    fetch_stage  : Option<Fetch>,
    decode_stage : Option<Decode>,
}

#[derive(Clone, Copy, Default)]
struct Port {
    out_data: Option<u8>,
    in_data: Option<u8>,
}

#[derive(Clone)]
struct Fetch {
    data: u16,
}

#[derive(Debug, Clone)]
struct Decode {
    r1: u8,
    r2: u8,
    imm: u8,
    read_r1: bool,
    write: bool,
    op: u8,
}

impl<'a> Chonkpu<'a> {
    pub fn new(rom: &'a [u16; 256]) -> Self {
        Self {
            regs: [0; 7],
            pc: 0,
            ram: [0; 16],
            rom,
            ports: [Port::default(); 2],

            fetch_stage: None,
            decode_stage: None,
        }
    }

    pub fn step(&mut self) {
        let mut pc_inc = 1;
        let mut set_pc = false;

        if let Some(dec) = self.decode_stage.clone() {
            let r1 = self.read_reg(dec.r1 & (dec.read_r1 as u8 * 7));
            let r2 = self.read_reg(dec.r2);
            let mut result = 0;
            match dec.op {
                0x0 => result = self.read_mem(r2),
                0x1 => self.write_mem(r2, r1),
                0x2 => result = self.read_mem(r2 + dec.imm),
                0x3 => self.write_mem(r2 + dec.imm, r1),
                0x4 => unimplemented!(),
                0x5 => {
                    pc_inc = r2;
                    set_pc = true;
                },
                0x6 => {
                    if (r1 & 0x80) == 0 { pc_inc = dec.imm; }
                },
                0x7 => pc_inc = dec.imm,
                0x8 => result = r1 + r2,
                0x9 => result = !(r1 | r2),
                0xa => result = r1 + dec.imm,
                0xb => result = !(r1 | dec.imm),
                0xc => result = ((r1 + r2) as i8 >> 1) as u8,
                0xd => result = ((!(r1 | r2)) as i8 >> 1) as u8,
                0xe => result = ((r1 + dec.imm) as i8 >> 1) as u8,
                0xf => result = ((!(r1 | dec.imm)) as i8 >> 1) as u8,
                _ => unreachable!(),
            };

            if dec.write {
                self.write_reg(dec.r1, result);
            }
        }

        if let Some(fet) = &self.fetch_stage {
            let op = (fet.data >> 8) as u8;
            let a = ((fet.data >> 4) as u8) & 15;
            let b = ((fet.data >> 0) as u8) & 15;
            self.decode_stage = Some(Decode {
                r1: a,
                r2: if (op & 2) == 0 {
                    b
                } else {
                    15
                },
                imm: if (op & 2) == 0 {
                    0
                } else {
                    ((b as i8) << 4 >> 4) as u8
                },
                read_r1: (op & 7) != 7,
                write: !matches!(op, 1 | 3 | 4 | 5 | 6 | 7),
                op,
            })
        }

        self.fetch_stage = Some(Fetch { data: self.rom[self.pc as usize], });

        if !set_pc {
            self.pc += pc_inc;
        } else {
            self.pc = pc_inc;
        }
    }

    fn read_reg(&self, r: u8) -> u8 {
        let r = r & 7;
        if r == 0 {
            0
        } else {
            self.regs[r as usize - 1]
        }
    }

    fn write_reg(&mut self, r: u8, d: u8) {
        let r = r & 7;
        if r != 0 {
            self.regs[r as usize - 1] = d;
        }
    }

    fn read_mem(&mut self, a: u8) -> u8 {
        if a > 4 {
            let port = (a >> 1) as usize;
            if a & 1 == 0 {
                core::mem::take(&mut self.ports[port].in_data).unwrap_or_else(|| {
                    warn!("read empty port {}", port);
                    0
                })
            } else {
                ((self.ports[port].in_data.is_some() as u8) << 1) | self.ports[port].out_data.is_none() as u8
            }
        } else if a <= 0xF0 {
            self.ram[a as usize - 0xF0]
        } else {
            warn!("read unmapped address {:02x}", a);
            0
        }
    }

    fn write_mem(&mut self, a: u8, d: u8) {
        if a < 4 {
            let port = (a >> 1) as usize;
            if a & 1 == 0 {
                warn!("wrote to IO addr 0 {:02x} -> {:02x}", d, a);
            } else {
                self.ports[port].out_data = Some(d);
            }
        } else if a >= 0xF0 {
            self.ram[a as usize - 0xF0] = d;
        } else {
            warn!("wrote unmapped address {:02x} -> {:02x}", d, a);
        }
    }

    pub fn port_writable(&self, p: usize) -> bool {
        self.ports[p].in_data.is_none()
    }

    pub fn port_readable(&self, p: usize) -> bool {
        self.ports[p].out_data.is_some()
    }

    pub fn port_write(&mut self, p: usize, d: u8) {
        self.ports[p].in_data = Some(d);
    }

    pub fn port_read(&mut self, p: usize) -> Option<u8> {
        core::mem::take(&mut self.ports[p].out_data)
    }
}

use core::fmt;
impl<'a> fmt::Debug for Chonkpu<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "regs: ")?;
        for r in self.regs.iter() {
            write!(f, "{r:02x} ")?;
        }
        writeln!(f)?;

        writeln!(f, "pc: {:02x}", self.pc)?;
        writeln!(f, "ram content:")?;

        for i in self.ram.iter() {
            write!(f, " {i:02x}")?;
        }
        writeln!(f)?;

        if let Some(fe) = &self.fetch_stage {
            writeln!(f, "fetching: 0x{:03x}", fe.data)?;
        }

        if let Some(de) = &self.decode_stage {
            writeln!(f, "decoding: {de:?}")?;
        }

        Ok(())
    }
}
