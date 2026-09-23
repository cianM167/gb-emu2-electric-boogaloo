use serde::de::value;

use crate::gb::apu::Channel::{Ch1, Ch2, Ch3, Ch4};

const DUTY_TABLE: [[u8; 8]; 4] = [
    [0, 0, 0, 0, 0, 0, 0, 0],
    [1, 0, 0, 0, 0, 0, 0, 1],
    [1, 0, 0, 0, 0, 1, 1, 1],
    [0, 1, 1, 1, 1, 1, 1, 1],
];

#[derive(Default, Debug)]
pub struct Apu {
    pub div_apu: u16,
    pub frame_seq_step: u8,
    pub ch1: PulseChannel,
    pub ch2: PulseChannel,
    pub ch3: PulseChannel,
    pub ch4: NoiseChannel,
    pub nr50: u8,
    pub nr51: u8,
    pub nr52: u8,
}

pub enum Channel {
    Ch1,
    Ch2,
    Ch3,
    Ch4,
}

#[derive(Default, Debug)]
pub struct PulseChannel {
    pub sweep: u8,
    pub duty_len: u8,
    pub envelope: u8,
    pub freq_lo: u8,
    pub freq_hi_ctrl: u8,

    pub enabled: bool,
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
    // getters/setters

    pub fn set_sweep(&mut self, value: u8) {
        // if self.enabled || self.dac_enabled {
        self.sweep = value
        // }
        // ignore write when off
    }

    pub fn set_duty(&mut self, value: u8) {
        // if self.enabled {
        self.duty_len = value
        // }
    }

    pub fn set_freq_lo(&mut self, value: u8) {
        self.freq_lo = value
    }

    // other stuff

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

        self.freq_timer = (2048 - self.frequency()) * 4;
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

#[derive(Default, Debug)]
pub struct NoiseChannel {
    pub duty_len: u8,// nr41
    pub envelope: u8,// nr42
    pub freq_rand: u8,// nr43
    pub freq_hi_ctrl: u8,

    pub enabled: bool,
    pub dac_enabled: bool,
    envelope_timer: u8,
    length_timer: u8,
    current_volume: u8,

    lfsr: u16,
    freq_timer: u32,
}

impl NoiseChannel {
    pub fn set_length(&mut self, value: u8) {// nr41
        self.length_timer = 64 - (value & 0x3F);
    }

    pub fn write_envelope(&mut self, val: u8) {// nr42
        self.envelope = val;
        self.dac_enabled = (val & 0b1111_1000) != 0;
        if !self.dac_enabled {
            self.enabled = false;
        }
    }

    pub fn set_freq_rand(&mut self, value: u8) {// set sub regs
        self.freq_rand = value
    }

    pub fn trigger(&mut self) {//  nr44
        self.enabled = self.dac_enabled;
        if self.length_timer == 0 {
            self.length_timer = 64;
        }

        self.freq_timer = self.period();
        self.lfsr = 0x7FFF;
        self.envelope_timer = self.envelope_period();
        self.current_volume = self.initial_volume();
    }

    fn envelope_period(&self) -> u8 {
        self.envelope & 0b0111
    }

    fn initial_volume(&self) -> u8 {
        self.envelope >> 4
    }

    fn divisor(&self) -> u32 {
        let r = (self.freq_rand & 0b111) as usize;
        const DIVISOR_TABLE: [u32; 8] = [8, 16, 32, 48, 64, 80, 96, 112];
        DIVISOR_TABLE[r]
    }

    fn shift_amount(&self) -> u8 {
        self.freq_rand >> 4
    }

    fn width_mode_7bit(&self) -> bool {
        (self.freq_rand >> 3) & 1 == 1
    }

    fn period(&self) -> u32 {
        self.divisor() << self.shift_amount()
    }

    pub fn step_timer(&mut self, cycles: u32) {
        if self.freq_timer <= cycles {
            let remainder = cycles - self.freq_timer;
            self.freq_timer = self.period();
            self.shift_lfsr();
            if remainder > 0 {
                self.step_timer(remainder);
            }
        } else {
            self.freq_timer -= cycles;
        }
    }

    fn shift_lfsr(&mut self) {
        let bit0 = self.lfsr & 1;
        let bit1 = (self.lfsr >> 1) & 1;
        let xor_result = bit0 ^ bit1;

        self.lfsr >>= 1;
        self.lfsr |= xor_result << 14;

        if self.width_mode_7bit() {
            self.lfsr &= !(1 << 6);
            self.lfsr |= xor_result << 6;
        }
    }

    pub fn sample(&self) -> f32 {
        if !self.enabled || !self.dac_enabled {
            return 0.0;
        }
        let bit = (!self.lfsr) & 1;
        (bit as f32) * (self.current_volume as f32 / 15.0)
    }
}

impl Apu {
    pub fn step(&mut self, cycles: u32) {
        self.ch2.step_timer(cycles as u16);
        self.ch4.step_timer(cycles as u32);
        self.ch1.step_timer(cycles as u16);// temporary

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

    pub fn mix_output(&self) -> f32 {
        self.ch2.sample() * 0.25
    }

    pub fn power_off(&mut self) {// 10000000000% wrong and lazy :)
        let ch1_len = self.ch1.length_timer;
        let ch2_len = self.ch2.length_timer;
        let ch3_len = self.ch3.length_timer;
        let ch4_len = self.ch4.length_timer;

        self.ch1 = PulseChannel::default();
        self.ch2 = PulseChannel::default();
        self.ch3 = PulseChannel::default();
        self.ch4 = NoiseChannel::default();

        self.ch1.length_timer = ch1_len;
        self.ch2.length_timer = ch2_len;
        self.ch3.length_timer = ch3_len;
        self.ch4.length_timer = ch4_len;

        self.nr50 = 0;
        self.nr51 = 0;
    }

    pub fn set_channel_sweep(&mut self, value: u8) {
        if self.nr52 & 0x80 != 0 {
            self.ch1.set_sweep(value);
        }
    }

    pub fn set_channel_duty(&mut self, channel: Channel, value: u8) {
        if self.nr52 & 0x80 != 0 {
            match channel {
                Ch1 => self.ch1.set_duty(value),
                Ch2 => self.ch2.set_duty(value),
                Ch3 => self.ch3.set_duty(value),
                Ch4 => self.ch4.set_length(value),
            }
        }
    }

    pub fn set_channel_freq_lo(&mut self, channel: Channel, value: u8) {
        if self.nr52 & 0x80 != 0 {
            match channel {
                Ch1 => self.ch1.set_freq_lo(value),
                Ch2 => self.ch2.set_freq_lo(value),
                Ch3 => self.ch3.set_freq_lo(value),
                Ch4 => self.ch4.set_freq_rand(value),
            }
        }
    }
}