#![allow(dead_code)]

use serde::Deserialize;

#[derive(Deserialize)]
pub struct TeamDto {
    pub name: String,
    #[serde(alias = "expectedGoals")]
    pub expected_goals: f64,
}

pub mod sim {
    use std::arch::x86_64::*;
    use std::{collections::HashSet, ops::Neg};

    use rand::{RngCore, SeedableRng};
    use simd_prng::specific::avx2::*;

    use crate::TeamDto;

    pub const HOME_ADVANTAGE: f64 = 0.25;

    type RngImpl = Xoshiro256PlusX4;
    type RngImplSeed = Xoshiro256PlusX4Seed;

    pub struct State {
        rng: RngImpl,

        matches: Matches,
    }

    #[derive(Default, Clone)]
    struct Matches {
        poisson: Vec<__m256d>,
        home: Vec<u8>,
        away: Vec<u8>,
        score: Vec<__m256d>,
    }

    impl Matches {
        pub fn new(number_of_matches: usize, teams: &[TeamDto]) -> Self {
            unsafe {
                let mut poisson = vec![_mm256_setzero_pd(); (number_of_matches * 2) / 4];
                let mut home = vec![0u8; number_of_matches];
                let mut away = vec![0u8; number_of_matches];
                let score = vec![_mm256_setzero_pd(); (number_of_matches * 2) / 4];

                let mut matchups = HashSet::with_capacity(number_of_matches);

                let mut match_index: usize = 0;
                let mut current_vec = [0.0; 4];
                let mut current_vec_index = 0;
                let mut poisson_index = 0;
                for i in 0..teams.len() {
                    for j in 0..teams.len() {
                        if i == j {
                            continue;
                        }

                        if matchups.insert((i as u8, j as u8)) {
                            home[match_index] = i as u8;
                            away[match_index] = j as u8;

                            current_vec[current_vec_index + 0] =
                                (teams[i].expected_goals + HOME_ADVANTAGE).neg().exp();
                            current_vec[current_vec_index + 1] =
                                teams[j].expected_goals.neg().exp();
                            current_vec_index += 2;
                            if current_vec_index == 4 {
                                poisson[poisson_index] = _mm256_set_pd(
                                    current_vec[0],
                                    current_vec[1],
                                    current_vec[2],
                                    current_vec[3],
                                );
                                poisson_index += 1;
                                current_vec_index = 0;
                            }

                            match_index += 1;
                        }
                    }
                }

                Self {
                    poisson,
                    home,
                    away,
                    score,
                }
            }
        }

        pub fn len(&self) -> usize {
            self.home.len()
        }

        pub fn reset_scores(&mut self) {
            unsafe {
                for vec in &mut self.score {
                    *vec = _mm256_setzero_pd();
                }
            }
        }
    }

    impl State {
        pub fn new(teams: &[TeamDto]) -> Self {
            let number_of_matches = (teams.len() - 1) * teams.len();
            let mut seed: RngImplSeed = Default::default();
            rand::thread_rng().fill_bytes(&mut *seed);

            let matches = Matches::new(number_of_matches, teams);

            Self {
                rng: RngImpl::from_seed(seed),
                matches: matches,
            }
        }
    }

    #[inline(never)]
    pub fn simulate<const S: usize>(state: &mut State) {
        unsafe {
            for _ in 0..S {
                for (_, (poisson_vec, goals)) in state
                    .matches
                    .poisson
                    .iter_mut()
                    .zip(state.matches.score.iter_mut())
                    .enumerate()
                {
                    *goals = _mm256_setzero_pd();

                    simulate_match(poisson_vec, goals, &mut state.rng);
                }
            }
        }

        state.matches.reset_scores();
    }

    #[inline(always)]
    unsafe fn simulate_match(poisson_vec: &__m256d, goals: &mut __m256d, rng: &mut RngImpl) {
        let mut product_vec = _mm256_setzero_pd();
        rng.next_m256d(&mut product_vec);

        loop {
            let sub = _mm256_sub_pd(product_vec, *poisson_vec);
            let mask = _mm256_movemask_pd(sub);
            if mask == 0x000F {
                break;
            }

            *goals = _mm256_add_pd(*goals, _mm256_ceil_pd(sub));

            let mut next_product_vec = _mm256_setzero_pd();
            rng.next_m256d(&mut next_product_vec);
            product_vec = _mm256_mul_pd(product_vec, next_product_vec);
        }
    }
}
