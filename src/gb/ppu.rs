pub struct Ppu {
    pub frame_buffer: [u32; 160 * 144],
    pub ready: bool,// signals ready to draw frame
    dot_counter: u16,
}

impl Ppu {
    pub fn new() -> Self {
        Self { 
            frame_buffer: [0; 160 * 144],
            dot_counter: 0, 
            ready: false,
        }
    }

    // pub fn step(&mut self, cycles: u8, bus: &mut Bus) {

    // }
}