#[derive(Default)]
pub struct Apu {
    pub div_apu: u16,
    
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

    pub dac: u8,
}

impl Apu {

}