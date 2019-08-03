use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Runtime configuration parameters.
///
/// These are loaded from `assets/config.json` when the program starts and then saved when the
/// program closes.
///
/// If no `assets/config.json` exists, a default one will be created.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub on_startup: OnStartup,
    #[serde(default)]
    pub colouration: Colouration,
}

/// Items that should run on startup.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct OnStartup {
    #[serde(default)]
    pub fullscreen: bool,
    #[serde(default)]
    pub serial: bool,
}

/// Colouration of the visualisation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Colouration {
    #[serde(default = "default::colouration::hue")]
    pub hue: f32,
    #[serde(default = "default::colouration::saturation")]
    pub saturation: f32,
    #[serde(default = "default::colouration::brightness")]
    pub brightness: f32,
    #[serde(default = "default::colouration::alpha")]
    pub alpha: f32,
}

impl Colouration {
    pub fn hsv(&self) -> nannou::color::Hsv {
        nannou::color::hsv(self.hue, self.saturation, self.brightness)
    }
}

impl Default for Colouration {
    fn default() -> Self {
        Colouration {
            hue: default::colouration::hue(),
            saturation: default::colouration::saturation(),
            brightness: default::colouration::brightness(),
            alpha: default::colouration::alpha(),
        }
    }
}

/// The path to the configuration file.
pub fn path(assets: &Path) -> PathBuf {
    assets.join("config.json")
}

pub mod default {
    pub mod colouration {
        use nannou::prelude::*;
        fn default_lin_srgb() -> LinSrgb {
            lin_srgb(0.0, 0.8, 0.4)
        }

        fn default_hsv() -> Hsv {
            default_lin_srgb().into()
        }

        pub fn hue() -> f32 {
            rad_to_turns(deg_to_rad(default_hsv().hue.into()))
        }

        pub fn saturation() -> f32 {
            default_hsv().saturation
        }

        pub fn brightness() -> f32 {
            default_hsv().value
        }

        pub fn alpha() -> f32 {
            1.0
        }
    }
}
