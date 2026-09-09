use crate::gb::bus::Bus;

pub struct Timer {
    div_counter: u16,
    tima_counter: u16,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            div_counter: 0,
            tima_counter: 0,
        }
    }

    pub fn step(&mut self, cycles: u8, bus: &mut Bus) {
        self.div_counter += cycles as u16;
        if self.div_counter >= 256 {
            self.div_counter -= 256;
            bus.set_div(bus.get_div().wrapping_add(1));
        }

        if bus.get_tac() & 0x04 == 0 {
            return;
        }

        let freq = match bus.get_tac() & 0x03 {
            0 => 1024,
            1 => 16,
            2 => 64,
            3 => 256,
            _ => unreachable!(),
        };

        self.tima_counter += cycles as u16;
        while self.tima_counter >= freq {
            self.tima_counter -= freq;

            if bus.get_tima() == 0xFF {
                bus.set_tima(bus.get_tma());

                bus.request_interrupt(2);
            } else {
                bus.set_tima(bus.get_tima().wrapping_add(1));
            }
        }
    }
}