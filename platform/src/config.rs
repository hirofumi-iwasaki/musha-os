// SPDX-License-Identifier: Apache-2.0
//! Allocation-free startup configuration. Parse atomically; callers use defaults on any error.
pub const MAX_CONFIG_BYTES: usize = 4096;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum LogLevel {
    Quiet,
    #[default]
    Ready,
    Phases,
    Devices,
    Trace,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Display {
    #[default]
    Max,
    Exact(u32, u32),
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Config {
    pub log: LogLevel,
    pub display: Display,
    pub pair: bool,
    /// Explicit startup opt-in; absence or atomic parse fallback disables echo.
    pub lan_echo: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParseError {
    TooLarge,
    NonAscii,
    Syntax,
    Value,
}
pub fn parse(bytes: &[u8]) -> Result<Config, ParseError> {
    if bytes.len() > MAX_CONFIG_BYTES {
        return Err(ParseError::TooLarge);
    }
    if bytes
        .iter()
        .any(|&b| !b.is_ascii() || (b < 32 && !matches!(b, b'\r' | b'\n' | b'\t')))
    {
        return Err(ParseError::NonAscii);
    }
    let text = core::str::from_utf8(bytes).map_err(|_| ParseError::NonAscii)?;
    let mut config = Config::default();
    for raw in text.split('\n') {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let (key, value) = line.split_once('=').ok_or(ParseError::Syntax)?;
        let (key, value) = (key.trim(), value.trim());
        if key.is_empty() {
            return Err(ParseError::Syntax);
        }
        match key {
            "log" => {
                config.log = match value {
                    "quiet" => LogLevel::Quiet,
                    "ready" => LogLevel::Ready,
                    "phases" => LogLevel::Phases,
                    "devices" => LogLevel::Devices,
                    "trace" => LogLevel::Trace,
                    _ => return Err(ParseError::Value),
                }
            }
            "display" => {
                config.display = if value == "max" {
                    Display::Max
                } else {
                    let (w, h) = value.split_once('x').ok_or(ParseError::Value)?;
                    if w.is_empty()
                        || h.is_empty()
                        || !w.bytes().chain(h.bytes()).all(|b| b.is_ascii_digit())
                    {
                        return Err(ParseError::Value);
                    }
                    let w = w.parse::<u32>().map_err(|_| ParseError::Value)?;
                    let h = h.parse::<u32>().map_err(|_| ParseError::Value)?;
                    if w == 0 || h == 0 {
                        return Err(ParseError::Value);
                    }
                    Display::Exact(w, h)
                }
            }
            "lan.echo" => {
                config.lan_echo = match value {
                    "on" => true,
                    "off" => false,
                    _ => return Err(ParseError::Value),
                }
            }
            "pair" => {
                config.pair = match value {
                    "on" => true,
                    _ => return Err(ParseError::Value),
                }
            }
            _ => {}
        }
    }
    Ok(config)
}
pub fn force_devices(scan: u16, unicode: u16) -> bool {
    scan == 0x12 || unicode == b'8' as u16
}
/// Validate the boot device's FAT32 BPB, not its label or the mere presence of SFS.
pub fn fat32_boot_sector(bytes: &[u8], block_size: u32, blocks: u64) -> bool {
    if bytes.len() < 512 || !matches!(block_size, 512 | 4096) || bytes[510..512] != [0x55, 0xaa] {
        return false;
    }
    let u16at = |i| u16::from_le_bytes([bytes[i], bytes[i + 1]]) as u64;
    let u32at = |i| u32::from_le_bytes(bytes[i..i + 4].try_into().unwrap()) as u64;
    let spc = bytes[13] as u64;
    let reserved = u16at(14);
    let fats = bytes[16] as u64;
    let sectors = u32at(32);
    let fat = u32at(36);
    if u16at(11) != block_size as u64
        || spc == 0
        || !spc.is_power_of_two()
        || spc > 128
        || reserved == 0
        || !matches!(fats, 1 | 2)
        || u16at(17) != 0
        || u16at(19) != 0
        || u16at(22) != 0
        || fat == 0
        || sectors > blocks
    {
        return false;
    }
    let Some(data) = sectors.checked_sub(reserved + fats * fat) else {
        return false;
    };
    let clusters = data / spc;
    clusters >= 65525
        && clusters < 0x0fff_fff5
        && fat * (block_size as u64) / 4 >= clusters + 2
        && (2..clusters + 2).contains(&u32at(44))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_and_comments() {
        assert_eq!(parse(b"\r\n# config\n\t"), Ok(Config::default()));
    }
    #[test]
    fn all_keys_crlf_and_last_key_wins() {
        assert_eq!(
            parse(b"log=quiet\r\nlog = devices # diagnostic\r\ndisplay=1024x768\r\npair=on\n"),
            Ok(Config {
                log: LogLevel::Devices,
                display: Display::Exact(1024, 768),
                pair: true,
                lan_echo: false
            })
        );
    }
    #[test]
    fn every_level() {
        for (text, level) in [
            (b"quiet".as_slice(), LogLevel::Quiet),
            (b"ready", LogLevel::Ready),
            (b"phases", LogLevel::Phases),
            (b"devices", LogLevel::Devices),
        ] {
            let mut data = [0; 16];
            data[..4].copy_from_slice(b"log=");
            data[4..4 + text.len()].copy_from_slice(text);
            assert_eq!(parse(&data[..4 + text.len()]).unwrap().log, level);
        }
    }
    #[test]
    fn unknown_keys_are_ignored() {
        assert_eq!(parse(b"future=anything=at all\n"), Ok(Config::default()));
    }
    #[test]
    fn malformed_does_not_publish_partial_settings() {
        for value in [
            b"log=quiet\nbad".as_slice(),
            b"log=LOUD",
            b"display=0x768",
            b"display=1x0",
            b"display=-1x10",
            b"display=4294967296x1",
            b"pair=off",
            b"=value",
            b"log=quiet\n\0",
            b"#\xff",
        ] {
            assert!(parse(value).is_err(), "{:?}", value);
        }
    }
    #[test]
    fn bounded_size() {
        assert!(parse(&[b' '; MAX_CONFIG_BYTES]).is_ok());
        assert_eq!(
            parse(&[b' '; MAX_CONFIG_BYTES + 1]),
            Err(ParseError::TooLarge)
        );
    }
    #[test]
    fn firmware_override_exact_keys() {
        assert!(force_devices(0x12, 0));
        assert!(force_devices(0, b'8' as u16));
        assert!(!force_devices(0x11, b'7' as u16));
    }
    #[test]
    fn fat32_geometry_not_just_label() {
        let mut b = [0; 512];
        b[11..13].copy_from_slice(&512u16.to_le_bytes());
        b[13] = 1;
        b[14..16].copy_from_slice(&32u16.to_le_bytes());
        b[16] = 2;
        b[32..36].copy_from_slice(&70000u32.to_le_bytes());
        b[36..40].copy_from_slice(&550u32.to_le_bytes());
        b[44..48].copy_from_slice(&2u32.to_le_bytes());
        b[510..].copy_from_slice(&[0x55, 0xaa]);
        assert!(fat32_boot_sector(&b, 512, 70000));
        assert!(!fat32_boot_sector(&b, 512, 69999));
        assert!(!fat32_boot_sector(&b, 4096, 70000));
        b[0..8].copy_from_slice(b"EXFAT   ");
        b[11..53].fill(0);
        assert!(!fat32_boot_sector(&b, 512, 70000));
    }
}
