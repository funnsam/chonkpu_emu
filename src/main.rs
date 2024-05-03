use chonkpu_emu::*;

fn main() {
    let rom_loc = std::env::args().nth(1).expect("expected rom filename as 1st arg");
    let rom = std::fs::read(&rom_loc).expect("failed to read rom");
    let rom = rom.chunks(2).map(|a| u16::from_be_bytes(a.try_into().unwrap())).collect::<Vec<u16>>().try_into().unwrap();

    let mut cpu = Chonkpu::new(&rom);

    loop {
        cpu.step();
        if let Some(d) = cpu.port_read(0) {
            println!("{}", d);
        }

        // eprintln!("{cpu:?}");
        std::thread::sleep_ms(50);
    }
}
