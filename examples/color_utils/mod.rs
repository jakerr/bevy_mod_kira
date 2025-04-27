#![allow(dead_code)]
use bevy_egui::egui::{Color32, Rgba, epaint::Hsva};

#[derive(Debug, Clone, Copy)]
pub enum Pallete {
    FreshGreen = 0x99dd55,
    LeafGreen = 0x44dd88,
    MintGreen = 0x22ccbb,
    AquaBlue = 0x0099cc,
    DeepBlue = 0x3366bb,
    GrapePurple = 0x663399,
}

impl From<Pallete> for Rgba {
    fn from(p: Pallete) -> Self {
        let col = p as u32;
        let r: u8 = ((col >> 16) & 0xff) as u8;
        let g: u8 = ((col >> 8) & 0xff) as u8;
        let b: u8 = (col & 0xff) as u8;
        Rgba::from_srgba_unmultiplied(r, g, b, 255)
    }
}

impl From<Pallete> for Color32 {
    fn from(p: Pallete) -> Self {
        let rgb: Rgba = p.into();
        rgb.into()
    }
}

pub fn shift_color(color: impl Into<Rgba>, degrees: f32) -> Color32 {
    let mut color = Hsva::from(color.into());
    color.h += degrees / 360.0;
    if color.h > 1.0 {
        color.h -= 1.0;
    }
    color.into()
}

pub fn light_color(color: impl Into<Rgba>) -> Color32 {
    let mut color = Hsva::from(color.into());
    color.s *= 0.8;
    color.v *= 0.70;
    color.into()
}

pub fn muted_color(color: impl Into<Rgba>) -> Color32 {
    let mut color = Hsva::from(color.into());
    color.s *= 0.35;
    color.v *= 0.30;
    color.into()
}

pub fn dark_color(color: impl Into<Rgba>) -> Color32 {
    let mut color = Hsva::from(color.into());
    color.s *= 0.35;
    color.v *= 0.10;
    color.into()
}

pub fn contrasty(color: impl Into<Rgba>) -> Color32 {
    let mut color = Hsva::from(color.into());
    let brightness = color.v * color.s;
    color.v = if brightness > 0.3 { 0.2 } else { 0.8 };
    color.into()
}
