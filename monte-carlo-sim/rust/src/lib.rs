#![allow(dead_code)]
#![feature(stdsimd)]

use serde::Deserialize;

#[derive(Deserialize)]
pub struct TeamDto {
    pub name: String,
    #[serde(alias = "expectedGoals")]
    pub expected_goals: f64,
}

pub mod sim {
    use std::arch::x86_64::*;
    use std::mem::transmute;
    use std::{collections::HashSet, ops::Neg};

    use rand::{RngCore, SeedableRng};
    use simd_prng::specific::avx512::*;

    use crate::TeamDto;

    pub const HOME_ADVANTAGE: f64 = 0.25;

    type RngImpl = Xoshiro256PlusX8;
    type RngImplSeed = Xoshiro256PlusX8Seed;

    pub struct State {
        rng: RngImpl,

        matches: Matches,
    }

    #[derive(Default, Clone)]
    struct Matches {
        poisson: Vec<__m512d>,
        home: Vec<u8>,
        away: Vec<u8>,
        score: Vec<__m512d>,
    }

    impl Matches {
        pub fn new(number_of_matches: usize, teams: &[TeamDto]) -> Self {
            unsafe {
                let mut poisson = vec![_mm512_setzero_pd(); (number_of_matches * 2) / 8];
                let mut home = vec![0u8; number_of_matches];
                let mut away = vec![0u8; number_of_matches];
                let score = vec![_mm512_setzero_pd(); (number_of_matches * 2) / 8];

                let mut matchups = HashSet::with_capacity(number_of_matches);

                let mut match_index: usize = 0;
                let mut current_vec = [0.0; 8];
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
                            if current_vec_index == 8 {
                                poisson[poisson_index] = _mm512_set_pd(
                                    current_vec[0],
                                    current_vec[1],
                                    current_vec[2],
                                    current_vec[3],
                                    current_vec[4],
                                    current_vec[5],
                                    current_vec[6],
                                    current_vec[7],
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
                    *vec = _mm512_setzero_pd();
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
            assert!(state.matches.poisson.len() == state.matches.score.len());

            for _ in 0..S {
                for i in 0..state.matches.poisson.len()
                {
                    let poisson_vec = state.matches.poisson.get_unchecked_mut(i);
                    let goals = state.matches.score.get_unchecked_mut(i);

                    *goals = _mm512_setzero_pd();

                    simulate_matches(poisson_vec, goals, &mut state.rng);
                }
            }
        }

        state.matches.reset_scores();
    }

    #[inline(always)]
    unsafe fn simulate_matches(poisson_vec: &__m512d, goals: &mut __m512d, rng: &mut RngImpl) {
        let mut product_vec = _mm512_setzero_pd();
        rng.next_m512d(&mut product_vec);

        loop {
            let sub = _mm512_sub_pd(product_vec, *poisson_vec);
            let mask = mm512_movemask_pd(sub);
            if mask == 0xFF {
                break;
            }

            *goals = _mm512_add_pd(*goals, _mm512_roundscale_pd::<2>(sub));

            let mut next_product_vec = _mm512_setzero_pd();
            rng.next_m512d(&mut next_product_vec);
            product_vec = _mm512_mul_pd(product_vec, next_product_vec);
        }
    }

    // There is no simple intrinsic for movemask as there is in other AVX2 (which is a single instruction intrinsic)
    // here is an equivalent for AVX512
    // taken from here: https://github.com/flang-compiler/flang/blob/d9280ff4e0cb296abec03ee7bb4a2b04f7dae932/runtime/libpgmath/lib/common/mth_avx512helper.h#L215
    pub unsafe fn mm512_movemask_pd(x: __m512d) -> u8 {
        _mm512_cmpneq_epi64_mask(
            _mm512_setzero_si512(),
            _mm512_and_si512(
                _mm512_set1_epi64(transmute::<_, i64>(0x8000000000000000u64)),
                _mm512_castpd_si512(x),
            ),
        )

        // Alt that is slower:
        // let mut mask: u8;
        // asm!(
        //     "vpmovq2m {1}, {0}",
        //     in(zmm_reg) sub,
        //     out(kreg) mask,
        // );
    }
}
