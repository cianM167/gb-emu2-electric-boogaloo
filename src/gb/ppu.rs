use crate::gb::{bus::Bus, ppu::PpuMode::OamScan};

#[derive(PartialEq, Clone, Copy)]
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

pub struct Ppu {
    pub frame_buffer: [u32; 160 * 144],
    pub ready: bool,
    dot_counter: u16,
    ly: u8,
    mode: PpuMode,
    bg_color_ids: [u8; 160],
}

impl Ppu {
    pub fn new() -> Self {
        Self { 
            frame_buffer: [0; 160 * 144],
            dot_counter: 0, 
            ready: false,
            ly: 0,
            mode: OamScan,
            bg_color_ids: [0; 160],
        }
    }

    pub fn step(&mut self, cycles: u8, bus: &mut Bus) {
        let lcdc = bus.get_lcdc();
        if lcdc & 0x80 == 0 {
            self.dot_counter = 0;
            self.mode = PpuMode::OamScan;
            bus.set_ly(0);

            let stat = bus.get_stat();
            bus.set_stat(stat & !0x07);

            return;
        }

        self.dot_counter += cycles as u16;

        match self.mode {
            PpuMode::OamScan => {
                while self.dot_counter >= 80 {
                    self.dot_counter -= 80;
                    self.mode = PpuMode::Drawing
                }
            }

            PpuMode::Drawing => {
                while self.dot_counter >= 172 {
                    self.dot_counter -= 172;
                    self.render_scanline(bus);
                    self.mode = PpuMode::HBlank;
                    self.update_stat(bus);
                }
            }

            PpuMode::HBlank => {
                while self.dot_counter >= 204 {
                    self.dot_counter -= 204;
                    self.ly += 1;
                    bus.set_ly(self.ly);

                    if self.ly == 144 {
                        self.mode = PpuMode::VBlank;
                        self.ready = true;
                        bus.request_interrupt(0);// vblank
                    } else {
                        self.mode = PpuMode::OamScan;
                    }
                    self.update_stat(bus);
                }
            }

            PpuMode::VBlank => {
                 while self.dot_counter >= 456 {
                    self.dot_counter -= 456;
                    self.ly += 1;
                    if self.ly > 153 {
                        self.ly = 0;
                        self.mode = PpuMode::OamScan;
                    }
                    bus.set_ly(self.ly);
                    self.update_stat(bus);
                }
            }
        }
    }

    fn render_scanline(&mut self, bus: &Bus) {
        let lcdc = bus.get_lcdc();
        if lcdc & 0x80 == 0 { return; } // LCD off

        if lcdc & 0x01 != 0 {
            self.render_background_line(bus);
        }
        if lcdc & 0x20 != 0 {
            self.render_window_line(bus);
        }
        if lcdc & 0x02 != 0 {
            self.render_sprites_line(bus);
        }
    }

    fn render_background_line(&mut self, bus: &Bus) {
        let lcdc = bus.get_lcdc();
        let scy = bus.get_scy();
        let scx = bus.get_scx();
        let bgp = bus.get_bgp();

        let tile_map_base: u16 = if lcdc & 0x08 != 0 { 0x9C00 } else { 0x9800 };
        let signed_tile_addressing = lcdc & 0x10 == 0; // LCDC bit 4 clear => 0x8800 signed mode

        let y = self.ly.wrapping_add(scy); // which row of the 256x256 background we're on
        let tile_row = (y / 8) as u16;      // which tile row (0-31)
        let pixel_row_in_tile = y % 8;      // which of the 8 rows within that tile

        for screen_x in 0..160u8 {
            let x = screen_x.wrapping_add(scx);
            let tile_col = (x / 8) as u16;
            let pixel_col_in_tile = x % 8;

            let tile_map_addr = tile_map_base + tile_row * 32 + tile_col;
            let tile_index = bus.read_vram(tile_map_addr);

            let tile_data_addr = if signed_tile_addressing {
                let signed_index = tile_index as i8 as i16;
                ((0x9000u16 as i16) + signed_index * 16) as u16
            } else {
                0x8000 + (tile_index as u16) * 16
            };

            // each row of a tile is 2 bytes (2bpp), low byte then high byte
            let row_addr = tile_data_addr + (pixel_row_in_tile as u16) * 2;
            let low_byte = bus.read_vram(row_addr);
            let high_byte = bus.read_vram(row_addr + 1);

            // bit 7 is the leftmost pixel
            let bit = 7 - pixel_col_in_tile;
            let color_id = ((high_byte >> bit) & 1) << 1 | ((low_byte >> bit) & 1);

            let shade = apply_palette(color_id, bgp);
            let fb_index = self.ly as usize * 160 + screen_x as usize;
            self.frame_buffer[fb_index] = shade_to_rgb(shade);

            // stash color_id for sprite priority checks later this scanline
            self.bg_color_ids[screen_x as usize] = color_id;
        }
    }

    fn render_window_line(&mut self, bus: &Bus) {
        let lcdc = bus.get_lcdc();
        let wy = bus.get_wy();
        let wx = bus.get_wx(); // real WX is offset by 7: on-screen x = WX - 7

        if self.ly < wy {
            return; // window hasn't started yet on this line
        }
        if wx > 166 {
            return; // window fully off-screen
        }

        let bgp = bus.get_bgp();
        let tile_map_base: u16 = if lcdc & 0x40 != 0 { 0x9C00 } else { 0x9800 };
        let signed_tile_addressing = lcdc & 0x10 == 0;

        let window_y = self.ly - wy; // window has its own internal line counter, starting at 0
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
            let tile_index = bus.read_vram(tile_map_addr);

            let tile_data_addr = if signed_tile_addressing {
                let signed_index = tile_index as i8 as i16;
                ((0x9000u16 as i16) + signed_index * 16) as u16
            } else {
                0x8000 + (tile_index as u16) * 16
            };

            let row_addr = tile_data_addr + (pixel_row_in_tile as u16) * 2;
            let low_byte = bus.read_vram(row_addr);
            let high_byte = bus.read_vram(row_addr + 1);

            let bit = 7 - pixel_col_in_tile;
            let color_id = ((high_byte >> bit) & 1) << 1 | ((low_byte >> bit) & 1);

            let shade = apply_palette(color_id, bgp);
            let fb_index = self.ly as usize * 160 + screen_x as usize;
            self.frame_buffer[fb_index] = shade_to_rgb(shade);
            self.bg_color_ids[screen_x as usize] = color_id;
        }
    }

    fn render_sprites_line(&mut self, bus: &Bus) {
        let lcdc = bus.get_lcdc();
        let tall_sprites = lcdc & 0x04 != 0; // 8x16 mode
        let sprite_height: u8 = if tall_sprites { 16 } else { 8 };

        // 1. gather sprites intersecting this scanline (max 10, OAM order = priority on ties)
        let mut visible = Vec::with_capacity(10);
        for i in 0..40 {
            let base = 0xFE00 + (i as u16) * 4;
            let sprite = SpriteAttr {
                y: bus.read_oam(base).wrapping_sub(16),
                x: bus.read_oam(base + 1).wrapping_sub(8),
                tile: bus.read_oam(base + 2),
                flags: bus.read_oam(base + 3),
            };

            let sprite_row = self.ly.wrapping_sub(sprite.y);
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
            let palette = if sprite.flags & 0x10 != 0 { bus.get_obp1() } else { bus.get_obp0() };
            let bg_priority = sprite.flags & 0x80 != 0; // true = sprite hidden behind BG color 1-3

            let mut row = self.ly.wrapping_sub(sprite.y);
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
            let low_byte = bus.read_vram(row_addr);
            let high_byte = bus.read_vram(row_addr + 1);

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
                let fb_index = self.ly as usize * 160 + screen_x as usize;
                self.frame_buffer[fb_index] = shade_to_rgb(shade);
            }
        }
    }

    fn update_stat(&mut self, bus: &mut Bus) {
        let mut stat = bus.get_stat();

        // mode bits (0-1)
        let mode_bits = match self.mode {
            PpuMode::HBlank => 0,
            PpuMode::VBlank => 1,
            PpuMode::OamScan => 2,
            PpuMode::Drawing => 3,
        };
        stat = (stat & !0x03) | mode_bits;

        // coincidence flag (bit 2): LY == LYC
        let lyc = bus.get_lyc();
        let coincidence = self.ly == lyc;
        stat = if coincidence { stat | 0x04 } else { stat & !0x04 };

        bus.set_stat(stat);

        // STAT interrupt sources (bits 3-6 are enable-selects, not raw status):
        // bit 3 = HBlank int enable, bit 4 = VBlank int enable,
        // bit 5 = OAM int enable, bit 6 = LYC==LY int enable
        let mut fire = false;
        if coincidence && stat & 0x40 != 0 { fire = true; }
        match self.mode {
            PpuMode::HBlank  if stat & 0x08 != 0 => fire = true,
            PpuMode::VBlank  if stat & 0x10 != 0 => fire = true,
            PpuMode::OamScan if stat & 0x20 != 0 => fire = true,
            _ => {}
        }

        if fire {
            bus.request_interrupt(1);
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
