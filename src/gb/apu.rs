use serde::de::value;

const DUTY_TABLE: [[u8; 8]; 4] = [
    [0, 0, 0, 0, 0, 0, 0, 0],
    [1, 0, 0, 0, 0, 0, 0, 1],
    [1, 0, 0, 0, 0, 1, 1, 1],
    [0, 1, 1, 1, 1, 1, 1, 1],
];

#[derive(Default)]
pub struct Apu {
    pub div_apu: u16,
    pub frame_seq_step: u8,
    pub ch1: PulseChannel,
    pub ch2: PulseChannel,
    pub ch3: PulseChannel,
    pub ch4: PulseChannel,
    pub nr50: u8,
    pub nr51: u8,
    pub nr52: u8,
}

#[derive(Default)]
pub struct PulseChannel {
    pub sweep: u8,
    pub duty_len: u8,
    pub envelope: u8,
    pub freq_lo: u8,
    pub freq_hi_ctrl: u8,

    enabled: bool,
    pub dac_enabled: bool,
    freq_timer: u16,
    duty_pos: u8,
    length_timer: u16,
    envelope_timer: u8,
    current_volume: u8,
    sweep_timer: u8,
    sweep_enable: bool,
    shadow_freq: u16,
}

impl PulseChannel {
    fn frequency(&self) -> u16 {
        ((self.freq_hi_ctrl as u16 & 0b1111) << 8) | self.freq_lo as u16
    }

    fn duty_cycle(&self) -> usize {
        ((self.duty_len >> 6) & 0b11) as usize
    }

    fn initial_length(&self) -> u16 {
        (self.duty_len & 0b0011_1111) as u16
    }

    fn initial_volume(&self) -> u8 {
        self.envelope >> 4
    }

    fn envelope_increasing(&self) -> bool {
        (self.envelope >> 3) & 1 == 1
    }

    fn envelope_period(&self) -> u8 {
        self.envelope & 0b1111
    }

    fn length_enabled(&self) -> bool {
        (self.freq_hi_ctrl >> 6) & 1 == 1
    }

    pub fn write_envelope(&mut self, val: u8) {
        self.envelope = val;
        self.dac_enabled = (val & 0b1111_1000) != 0;
        if !self.dac_enabled {
            self.enabled = false;
        }
    }

    pub fn trigger(&mut self) {
        self.enabled = self.dac_enabled;
        if self.length_timer == 0 {
            self.length_timer = 64;
        }

        self.freq_timer= (2048 - self.frequency()) * 4;
        self.envelope_timer = self.envelope_period();
        self.current_volume = self.initial_volume();
    }

    pub fn step_timer(&mut self, cycles: u16) {
        if self.freq_timer <= cycles {
            let remainder = cycles - self.freq_timer;
            self.freq_timer = (2048 - self.frequency()) * 4;
            self.duty_pos = (self.duty_pos + 1) % 8;
            if remainder > 0 {
                self.step_timer(remainder);
            }
        } else {
            self.freq_timer -= cycles
        }
    }

    pub fn clock_length(&mut self) {
        if self.length_enabled() && self.length_timer > 0 {
            self.length_timer -= 1;
            if self.length_timer == 0 {
                self.enabled = false;
            }
        }
    }

    pub fn clock_envelope(&mut self) {
        let period = self.envelope_period();
        if period == 0 {
            return;
        }
        if self.envelope_timer > 0 {
            self.envelope_timer -= 1;
        }
        if self.envelope_timer == 0 {
            self.envelope_timer = period;
            if self.envelope_increasing() && self.current_volume < 15 {
                self.current_volume += 1;
            } else if !self.envelope_increasing() && self.current_volume > 0 {
                self.current_volume -= 1;
            }
        }
    }

    pub fn sample(&self) -> f32 {
        if !self.enabled || !self.dac_enabled {
            return 0.0;
        }
        let bit = DUTY_TABLE[self.duty_cycle()][self.duty_pos as usize];
        (bit as f32) * (self.current_volume as f32 / 15.0)
    }
}

impl Apu {
    pub fn step(&mut self, cycles: u32) {
        self.ch2.step_timer(cycles as u16);

        self.div_apu = self.div_apu.wrapping_add(cycles as u16);
        while self.div_apu >= 8192 {
            self.div_apu -= 8192;
            self.frame_seq_step = (self.frame_seq_step + 1) % 8;
            match self.frame_seq_step {
                0 | 2 | 4 | 6 => self.ch2.clock_length(),
                7 => self.ch2.clock_envelope(),
                _ => ()
            }
        }
    }
}