use serde::{Deserialize, Serialize, de::value};
use serde_big_array::BigArray;
use crate::gb::{bus::Bus, ppu::PpuMode::OamScan};

#[derive(PartialEq, Clone, Copy, Serialize, Deserialize)]
enum PpuMode {
    OamScan,
    Drawing,
    HBlank,
    VBlank,
}

struct SpriteAttr {
    y: u8,
    x: u8,
    tile: u8,
    flags: u8,
}

#[derive(Serialize, Deserialize, Clone, Copy, Default)]
pub struct PpuRegs {
    pub lcdc: u8,  // FF40
    pub stat: u8,  // FF41 (only bits 3-6 are writable)
    pub scy: u8,   // FF42
    pub scx: u8,   // FF43
    pub ly: u8,    // FF44 (read-only)
    pub lyc: u8,   // FF45
    pub bgp: u8,   // FF47
    pub obp0: u8,  // FF48
    pub obp1: u8,  // FF49
    pub wy: u8,    // FF4A
    pub wx: u8,    // FF4B
}

impl PpuRegs {
    pub fn post_boot(cgb: bool) -> Self {
        Self {
            lcdc: 0x91,
            stat: 0x85,
            scy:  0x00,
            scx:  0x00,
            ly:   0x00,
            lyc:  0x00,
            bgp:  0xFC,
            obp0: 0xFF,
            obp1: 0xFF,
            wy:   0x00,
            wx:   0x00,
        }
    }
}

#[derive(Clone, Copy, Default)]
pub struct PpuEvents {
    pub vblank: bool,
    pub stat: bool,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct Ppu {
    #[serde(with = "BigArray")]
    pub frame_buffer: [u32; 160 * 144],
    pub ready: bool,
    pub dot_counter: u16,
    mode: PpuMode,
    #[serde(with = "BigArray")]
    bg_color_ids: [u8; 160],
    just_enabled: bool,
    was_on_last_step: bool,
    
    pub regs: PpuRegs,
    #[serde(with = "BigArray")]
    vram: [u8; 0x2000],
    #[serde(with = "BigArray")]
    oam: [u8; 0xA0],

    stat_line: bool,
}

impl Ppu {
    pub fn read_reg(&self, addr: u16) -> u8 {
        match addr {
            0xFF40 => self.regs.lcdc,
            0xFF41 => {
                let on = self.regs.lcdc & 0x80 != 0;
                let (mode, coin) = if on { (self.mode_bits(), (self.regs.ly == self.regs.lyc) as u8) } else { (0, 0) };
                0x80 | (self.regs.stat & 0x78) | (coin << 2) | mode
            }
            0xFF42 => self.regs.scy,
            0xFF43 => self.regs.scx,
            0xFF44 => self.regs.ly,
            0xFF45 => self.regs.lyc,
            0xFF47 => self.regs.bgp,
            0xFF48 => self.regs.obp0,
            0xFF49 => self.regs.obp1,
            0xFF4A => self.regs.wy,
            0xFF4B => self.regs.wx,
            _ => 0xFF,
        }
    }

    pub fn write_reg(&mut self, addr: u16, v: u8) {
        match addr {
            0xFF40 => self.regs.lcdc = v,
            0xFF41 => self.regs.stat = (self.regs.stat & !0x78) | (v & 0x78),
            0xFF42 => self.regs.scy = v,
            0xFF43 => self.regs.scx = v,
            0xFF44 => {} // read-only
            0xFF45 => self.regs.lyc = v,
            0xFF47 => self.regs.bgp = v,
            0xFF48 => self.regs.obp0 = v,
            0xFF49 => self.regs.obp1 = v,
            0xFF4A => self.regs.wy = v,
            0xFF4B => self.regs.wx = v,
            _ => {}
        }
    }

    pub fn read_vram(&self, addr: u16) -> u8 {
        self.vram[(addr - 0x8000) as usize]
    }

    pub fn write_vram(&mut self, addr: u16, value: u8) {
        self.vram[(addr - 0x8000) as usize] = value
    }

    pub fn read_oam(&self, addr: u16) -> u8 {
        self.oam[(addr - 0xFE00) as usize]
    }

    pub fn write_oam(&mut self, addr: u16, value: u8) {
        // println!("addr going in: {}", addr - 0xFE00);
        self.oam[(addr - 0xFE00) as usize] = value
    }

    pub fn new(cgb: bool) -> Self {
        Self { 
            frame_buffer: [0; 160 * 144],
            dot_counter: 0, 
            ready: false,
            mode: OamScan,
            bg_color_ids: [0; 160],
            just_enabled: false,
            was_on_last_step: false,

            regs: PpuRegs::post_boot(cgb),

            vram: [0; 0x2000],
            oam: [0; 0xA0],

            stat_line: false,
        }
    }

    pub fn step(&mut self, cycles: u8) -> PpuEvents {
        let mut events = PpuEvents::default();
        let lcd_on = self.regs.lcdc & 0x80 != 0;

        if !lcd_on {
            self.dot_counter = 0;
            self.mode = PpuMode::HBlank;
            self.regs.ly = 0;

            self.stat_line = false;

            self.was_on_last_step = false;

            return events;
        }

        if !self.was_on_last_step {
            self.dot_counter = 0;
            self.mode = PpuMode::HBlank;
            self.just_enabled = true;
        }
        self.was_on_last_step = true;

        self.dot_counter += cycles as u16;

        match self.mode {
            PpuMode::OamScan => {
                while self.dot_counter >= 80 {
                    self.dot_counter -= 80;
                    self.mode = PpuMode::Drawing;
                }
            }
            PpuMode::Drawing => {
                while self.dot_counter >= 172 {
                    self.dot_counter -= 172;
                    self.render_scanline();
                    self.mode = PpuMode::HBlank;
                }
            }
            PpuMode::HBlank => {
                if self.just_enabled {
                    while self.dot_counter >= 80 {
                        self.dot_counter -= 80;
                        self.mode = PpuMode::Drawing;
                        self.just_enabled = false;
                    }
                } else {
                    while self.dot_counter >= 204 {
                        self.dot_counter -= 204;
                        self.regs.ly += 1;
                        if self.regs.ly == 144 {
                            self.mode = PpuMode::VBlank;
                            self.ready = true;
                            events.vblank = true;
                        } else {
                            self.mode = PpuMode::OamScan;
                        }
                    }
                }
            }
            PpuMode::VBlank => {
                while self.dot_counter >= 456 {
                    self.dot_counter -= 456;
                    self.regs.ly += 1;
                    if self.regs.ly > 153 {
                        self.regs.ly = 0;
                        self.mode = PpuMode::OamScan;
                    }
                }
            }
        }

        events.stat = self.update_stat_line();
        events
    }

    fn render_scanline(&mut self) {
        let lcdc = self.regs.lcdc;
        if lcdc & 0x80 == 0 { return; } // LCD off

        if lcdc & 0x01 != 0 {
            self.render_background_line();
        }
        if lcdc & 0x20 != 0 {
            self.render_window_line();
        }
        if lcdc & 0x02 != 0 {
            self.render_sprites_line();
        }
    }

    fn render_background_line(&mut self) {
        let lcdc = self.regs.lcdc;
        let scy = self.regs.scy;
        let scx = self.regs.scx;
        let bgp = self.regs.bgp;

        let tile_map_base: u16 = if lcdc & 0x08 != 0 { 0x9C00 } else { 0x9800 };
        let signed_tile_addressing = lcdc & 0x10 == 0; // LCDC bit 4 clear => 0x8800 signed mode

        let y = self.regs.ly.wrapping_add(scy); // which row of the 256x256 background we're on
        let tile_row = (y / 8) as u16;      // which tile row (0-31)
        let pixel_row_in_tile = y % 8;      // which of the 8 rows within that tile

        for screen_x in 0..160u8 {
            let x = screen_x.wrapping_add(scx);
            let tile_col = (x / 8) as u16;
            let pixel_col_in_tile = x % 8;

            let tile_map_addr = tile_map_base + tile_row * 32 + tile_col;
            let tile_index = self.read_vram(tile_map_addr);

            let tile_data_addr = if signed_tile_addressing {
                let signed_index = tile_index as i8 as i16;
                ((0x9000u16 as i16) + signed_index * 16) as u16
            } else {
                0x8000 + (tile_index as u16) * 16
            };

            // each row of a tile is 2 bytes (2bpp), low byte then high byte
            let row_addr = tile_data_addr + (pixel_row_in_tile as u16) * 2;
            let low_byte = self.read_vram(row_addr);
            let high_byte = self.read_vram(row_addr + 1);

            // bit 7 is the leftmost pixel
            let bit = 7 - pixel_col_in_tile;
            let color_id = ((high_byte >> bit) & 1) << 1 | ((low_byte >> bit) & 1);

            let shade = apply_palette(color_id, bgp);
            let fb_index = self.regs.ly as usize * 160 + screen_x as usize;
            self.frame_buffer[fb_index] = shade_to_rgb(shade);

            // stash color_id for sprite priority checks later this scanline
            self.bg_color_ids[screen_x as usize] = color_id;
        }
    }

    fn render_window_line(&mut self) {
        let lcdc = self.regs.lcdc;
        let wy = self.regs.wy;
        let wx = self.regs.wx; // real WX is offset by 7: on-screen x = WX - 7

        if self.regs.ly < wy {
            return; // window hasn't started yet on this line
        }
        if wx > 166 {
            return; // window fully off-screen
        }

        let bgp = self.regs.bgp;
        let tile_map_base: u16 = if lcdc & 0x40 != 0 { 0x9C00 } else { 0x9800 };
        let signed_tile_addressing = lcdc & 0x10 == 0;

        let window_y = self.regs.ly - wy; // window has its own internal line counter, starting at 0
        let tile_row = (window_y / 8) as u16;
        let pixel_row_in_tile = window_y % 8;

        let wx_signed = wx as i16 - 7;

        for screen_x in 0..160i16 {
            let window_x = screen_x - wx_signed;
            if window_x < 0 {
                continue; // window hasn't reached this column yet
            }
            let window_x = window_x as u8;

            let tile_col = (window_x / 8) as u16;
            let pixel_col_in_tile = window_x % 8;

            let tile_map_addr = tile_map_base + tile_row * 32 + tile_col;
            let tile_index = self.read_vram(tile_map_addr);

            let tile_data_addr = if signed_tile_addressing {
                let signed_index = tile_index as i8 as i16;
                ((0x9000u16 as i16) + signed_index * 16) as u16
            } else {
                0x8000 + (tile_index as u16) * 16
            };

            let row_addr = tile_data_addr + (pixel_row_in_tile as u16) * 2;
            let low_byte = self.read_vram(row_addr);
            let high_byte = self.read_vram(row_addr + 1);

            let bit = 7 - pixel_col_in_tile;
            let color_id = ((high_byte >> bit) & 1) << 1 | ((low_byte >> bit) & 1);

            let shade = apply_palette(color_id, bgp);
            let fb_index = self.regs.ly as usize * 160 + screen_x as usize;
            self.frame_buffer[fb_index] = shade_to_rgb(shade);
            self.bg_color_ids[screen_x as usize] = color_id;
        }
    }

    fn render_sprites_line(&mut self) {
        let lcdc = self.regs.lcdc;
        let tall_sprites = lcdc & 0x04 != 0; // 8x16 mode
        let sprite_height: u8 = if tall_sprites { 16 } else { 8 };

        // 1. gather sprites intersecting this scanline (max 10, OAM order = priority on ties)
        let mut visible = Vec::with_capacity(10);
        for i in 0..40 {
            let base = 0xFE00 + (i as u16) * 4;
            let sprite = SpriteAttr {
                y: self.read_oam(base).wrapping_sub(16),
                x: self.read_oam(base + 1).wrapping_sub(8),
                tile: self.read_oam(base + 2),
                flags: self.read_oam(base + 3),
            };

            let sprite_row = self.regs.ly.wrapping_sub(sprite.y);
            if sprite_row < sprite_height {
                visible.push((i, sprite)); // keep OAM index for tie-break ordering
                if visible.len() == 10 {
                    break;
                }
            }
        }

        // 2. sort by X ascending (lower X = higher priority); OAM index breaks ties, already preserved by stable sort
        visible.sort_by_key(|(idx, s)| (s.x, *idx));

        // 3. draw highest-priority sprite last isn't right — draw LOWEST priority first, so higher priority overwrites.
        //    Real hardware: sprite with smallest X wins on overlap. So draw in reverse priority order.
        for (_, sprite) in visible.into_iter().rev() {
            let y_flip = sprite.flags & 0x40 != 0;
            let x_flip = sprite.flags & 0x20 != 0;
            let palette = if sprite.flags & 0x10 != 0 { self.regs.obp1 } else { self.regs.obp0 };
            let bg_priority = sprite.flags & 0x80 != 0; // true = sprite hidden behind BG color 1-3

            let mut row = self.regs.ly.wrapping_sub(sprite.y);
            if y_flip {
                row = sprite_height - 1 - row;
            }

            let tile_index = if tall_sprites {
                if row < 8 { sprite.tile & 0xFE } else { sprite.tile | 0x01 }
            } else {
                sprite.tile
            };
            let row_in_tile = row % 8;

            let tile_data_addr = 0x8000 + (tile_index as u16) * 16; // sprites always use unsigned addressing
            let row_addr = tile_data_addr + (row_in_tile as u16) * 2;
            let low_byte = self.read_vram(row_addr);
            let high_byte = self.read_vram(row_addr + 1);

            for col in 0..8u8 {
                let screen_x = sprite.x.wrapping_add(col);
                if screen_x >= 160 {
                    continue;
                }

                let bit = if x_flip { col } else { 7 - col };
                let color_id = ((high_byte >> bit) & 1) << 1 | ((low_byte >> bit) & 1);

                if color_id == 0 {
                    continue; // transparent
                }

                if bg_priority && self.bg_color_ids[screen_x as usize] != 0 {
                    continue; // background wins when this sprite is set to be behind non-zero BG colors
                }

                let shade = apply_palette(color_id, palette);
                let fb_index = self.regs.ly as usize * 160 + screen_x as usize;
                self.frame_buffer[fb_index] = shade_to_rgb(shade);
            }
        }
    }

    fn update_stat_line(&mut self) -> bool {
        let stat = self.regs.stat;
        let coincidence = self.regs.ly == self.regs.lyc;

        let line = (coincidence && stat & 0x40 != 0)
            || (matches!(self.mode, PpuMode::HBlank)  && stat & 0x08 != 0)
            || (matches!(self.mode, PpuMode::VBlank)  && stat & 0x10 != 0)
            || (matches!(self.mode, PpuMode::OamScan) && stat & 0x20 != 0);

        let rising = line && !self.stat_line;
        self.stat_line = line;
        rising
    }

    fn update_stat_line_suppressed(&mut self) -> bool {
        let coincidence = self.regs.ly == self.regs.lyc;
        let line = coincidence && self.regs.stat & 0x40 != 0;
        let rising = line && !self.stat_line;
        self.stat_line = line;
        rising
    }

    fn mode_bits(&self) -> u8 {
    match self.mode {
        PpuMode::HBlank  => 0,
        PpuMode::VBlank  => 1,
        PpuMode::OamScan => 2,
        PpuMode::Drawing => 3,
    }
}
}

fn apply_palette(color_id: u8, palette: u8) -> u8 {
    // each 2-bit color id selects a 2-bit shade from the palette byte
    (palette >> (color_id * 2)) & 0x03
}

fn shade_to_rgb(shade: u8) -> u32 {
    match shade {
        0 => 0xFFFFFFFF, // white
        1 => 0xFFAAAAAA, // light gray
        2 => 0xFF555555, // dark gray
        3 => 0xFF000000, // black
        _ => unreachable!(),
    }
}
