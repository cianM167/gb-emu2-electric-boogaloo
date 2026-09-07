#[derive(Debug)]
pub struct CartHeader {
    pub title: String,
    pub cartridge_type: u8,
    pub rom_size: u8,
    pub ram_size: u8,
    pub old_lic_code: u8,
    pub new_lic_code: [u8; 2],
    pub dest_code: u8,
    pub version: u8,
    pub checksum: u8,
}

impl CartHeader {
    pub fn parse(rom: &[u8]) -> Result<Self, String> {
        let title_bytes = &rom[0x0134..0x0144];
        let title = title_bytes
            .iter()
            .take_while(|&&b| b != 0)
            .map(|&b| b as char)
            .collect();

        Ok(
            Self {
                title,
                cartridge_type: rom[0x0147],
                rom_size: rom[0x0148],
                ram_size: rom[0x0149],
                old_lic_code: rom[0x014B],
                new_lic_code: [rom[0x0144], rom[0x0145]],
                dest_code: rom[0x014A],
                version: rom[0x014C],
                checksum: rom[0x014D],
            }
        )
    }

    
}