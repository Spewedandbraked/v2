use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Serialize, Deserialize)]
pub struct RendererConfig {
    pub profile: Profile,
    /// Количество сэмплов MSAA: 1 (выкл) или 4
    pub samples: SampleCount,
    /// Скейл. я вряд-ли реализую другие оптимизации, а эта хотя бы тупо урежет колличество пикселей у чуваков
    pub scale: f32,
    /// Режим вертикальной синхронизации: 0 = Immediate, 1 = Fifo, 2 = Mailbox. Я сам нихуя не понимаю что это
    pub present_mode: PresentMode,
}
impl RendererConfig {
    pub fn load_or_default(path: &str) -> Self {
        if !Path::new(path).exists() {
            let config = Self::default();
            std::fs::write(path, serde_json::to_string_pretty(&config).unwrap()).unwrap();
            config
        } else {
            let data = std::fs::read_to_string(path).unwrap();
            serde_json::from_str(&data).unwrap_or_else(|_| {
                eprintln!("Failed to parse config, falling back to default.");
                Self::default()
            })
        }
    }
}
impl Default for RendererConfig {
    fn default() -> Self {
        Self {
            profile: Profile::GpuDriven,
            samples: SampleCount(rend3::types::SampleCount::One),
            scale: 1.0,
            present_mode: PresentMode(rend3::types::PresentMode::Fifo),
        }
    }
}


#[derive(Serialize, Deserialize)]
pub enum Profile {
    GpuDriven,
    CpuDriven,
}
impl From<Profile> for Option<rend3::RendererProfile> {
    fn from(p: Profile) -> Self {
        match p {
            Profile::GpuDriven => Some(rend3::RendererProfile::GpuDriven),
            Profile::CpuDriven => Some(rend3::RendererProfile::CpuDriven),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SampleCount(pub rend3::types::SampleCount);
impl Serialize for SampleCount {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            rend3::types::SampleCount::One => 1u8.serialize(s),
            rend3::types::SampleCount::Four => 4u8.serialize(s),
        }
    }
}
impl<'de> Deserialize<'de> for SampleCount {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = u8::deserialize(d)?;
        match v {
            1 => Ok(SampleCount(rend3::types::SampleCount::One)),
            4 => Ok(SampleCount(rend3::types::SampleCount::Four)),
            _ => Err(serde::de::Error::custom("samples must be 1 or 4")),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PresentMode(pub rend3::types::PresentMode);
impl Serialize for PresentMode {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            rend3::types::PresentMode::Immediate => 0u8.serialize(s),
            rend3::types::PresentMode::Fifo => 1u8.serialize(s),
            rend3::types::PresentMode::Mailbox => 2u8.serialize(s),
        }
    }
}
impl<'de> Deserialize<'de> for PresentMode {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = u8::deserialize(d)?;
        match v {
            0 => Ok(PresentMode(rend3::types::PresentMode::Immediate)),
            1 => Ok(PresentMode(rend3::types::PresentMode::Fifo)),
            2 => Ok(PresentMode(rend3::types::PresentMode::Mailbox)),
            _ => Err(serde::de::Error::custom("present_mode must be 0, 1 or 2")),
        }
    }
}