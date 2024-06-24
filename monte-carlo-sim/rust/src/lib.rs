#![allow(dead_code)]
#![feature(stdarch_x86_avx512)]
#![feature(allocator_api)]

use serde::Deserialize;

pub mod util;

#[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
pub mod sim_avx2;

#[cfg(all(target_arch = "x86_64", target_feature = "avx512f", target_feature = "avx512dq"))]
pub mod sim_avx512;

pub mod sim {
    cfg_if::cfg_if! {
        if #[cfg(all(target_arch = "x86_64", target_feature = "avx512f", target_feature = "avx512dq"))] {
            pub use super::sim_avx512::*;
        } else if #[cfg(all(target_arch = "x86_64", target_feature = "avx2"))] {
            pub use super::sim_avx2::*;
        }
    }
}

/// A DTO for deserializing from the input.json file
#[derive(Deserialize)]
pub struct TeamDto {
    pub name: String,
    #[serde(alias = "expectedGoals")]
    pub expected_goals: f64,
}
