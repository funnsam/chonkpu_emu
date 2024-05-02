use emu::*;

fn main() {
    let mut cpu = Chonkpu::new(&[0x400; 256]);

    loop {
        cpu.step();
        println!("{cpu:?}");
    }
}
