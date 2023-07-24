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
    use std::mem::{self, transmute};
    use std::{collections::HashSet, ops::Neg};

    use rand::{RngCore, SeedableRng};
    use simd_rand::specific::avx512::*;

    use crate::TeamDto;

    pub const HOME_ADVANTAGE: f64 = 0.25;

    type RngImpl = Xoshiro256PlusX8;
    type RngImplSeed = Xoshiro256PlusX8Seed;

    pub struct State {
        rng: RngImpl,
        number_of_teams: usize,
        matches: Matches,
    }

    impl State {
        fn size_of(&self) -> usize {
            mem::size_of::<Self>()
                + (self.matches.poisson.len() * mem::size_of::<__m512d>())
                + (self.matches.home.len())
                + (self.matches.away.len())
                + (self.matches.score.len() * mem::size_of::<__m512d>())
        }
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
                let mut current_home_vec = [0.0; 8];
                let mut current_away_vec = [0.0; 8];
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

                            current_home_vec[current_vec_index] =
                                (teams[i].expected_goals + HOME_ADVANTAGE).neg().exp();
                            current_away_vec[current_vec_index] =
                                teams[j].expected_goals.neg().exp();

                            current_vec_index += 1;

                            if current_vec_index == 8 {
                                poisson[poisson_index + 0] = _mm512_set_pd(
                                    current_home_vec[0],
                                    current_home_vec[1],
                                    current_home_vec[2],
                                    current_home_vec[3],
                                    current_home_vec[4],
                                    current_home_vec[5],
                                    current_home_vec[6],
                                    current_home_vec[7],
                                );
                                poisson[poisson_index + 1] = _mm512_set_pd(
                                    current_away_vec[0],
                                    current_away_vec[1],
                                    current_away_vec[2],
                                    current_away_vec[3],
                                    current_away_vec[4],
                                    current_away_vec[5],
                                    current_away_vec[6],
                                    current_away_vec[7],
                                );
                                poisson_index += 2;
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
                number_of_teams: teams.len(),
                matches: matches,
            }
        }
    }

    #[inline(never)]
    pub fn simulate<const S: usize>(state: &mut State) {
        unsafe {
            assert!(state.matches.poisson.len() == state.matches.score.len());

            let poisson = state.matches.poisson.as_ptr();
            let scores = state.matches.score.as_mut_ptr();

            for _ in 0..S {
                let table = vec![_mm512_setzero_pd(); state.number_of_teams];

                for i in (0..state.matches.poisson.len()).step_by(2) {
                    let home_poisson = poisson.add(i + 0);
                    let away_poisson = poisson.add(i + 1);

                    let home_goals = scores.add(i + 0);
                    let away_goals = scores.add(i + 1);

                    *home_goals = _mm512_setzero_pd();
                    *away_goals = _mm512_setzero_pd();

                    simulate_sides(home_poisson, home_goals, &mut state.rng);
                    simulate_sides(away_poisson, away_goals, &mut state.rng);

                    let lt = _mm512_cmp_pd_mask::<_CMP_LT_OQ>(*home_goals, *away_goals);
                    let gt = _mm512_cmp_pd_mask::<_CMP_GT_OQ>(*home_goals, *away_goals);
                    let eq = _mm512_cmp_pd_mask::<_CMP_EQ_OQ>(*home_goals, *away_goals);

                    // _mm512_mask_add_pd(z, ge, h, _mm512_set1_pd(3));

                    dbg!({
                        let lts = format!("{:#010b}", lt);
                        let gts = format!("{:#010b}", gt);
                        let eqs = format!("{:#010b}", eq);
                        (lts, gts, eqs)
                    });
                }
            }
        }

        state.matches.reset_scores();
    }

    #[inline(always)]
    unsafe fn simulate_sides(poisson_vec: *const __m512d, goals: *mut __m512d, rng: &mut RngImpl) {
        let mut product_vec = rng.next_m512d();

        loop {
            let sub = _mm512_sub_pd(product_vec, *poisson_vec);
            let mask = mm512_movemask_pd(sub);
            if mask == 0xFF {
                break;
            }

            *goals = _mm512_add_pd(*goals, _mm512_roundscale_pd::<2>(sub));

            let next_product_vec = rng.next_m512d();
            product_vec = _mm512_mul_pd(product_vec, next_product_vec);
        }
    }

    // There is no simple intrinsic for movemask as there is in other AVX2 (which is a single instruction intrinsic)
    // here is an equivalent for AVX512
    // taken from here: https://github.com/flang-compiler/flang/blob/d9280ff4e0cb296abec03ee7bb4a2b04f7dae932/runtime/libpgmath/lib/common/mth_avx512helper.h#L215
    #[inline(always)]
    pub unsafe fn mm512_movemask_pd(x: __m512d) -> u8 {
        _mm512_cmpneq_epi64_mask(
            _mm512_setzero_si512(),
            _mm512_and_si512(
                _mm512_set1_epi64(transmute::<_, i64>(0x8000000000000000u64)),
                _mm512_castpd_si512(x),
            ),
        )

        // Alternative that was measured to be slower:
        // let mut mask: u8;
        // std::arch::asm!(
        //     "vpmovq2m {1}, {0}",
        //     in(zmm_reg) x,
        //     out(kreg) mask,
        //     // 'nostack' option tells the compiler that we won't be touching the stack
        //     // in our inline asm. This improves the generated code some.
        //     // For instance, the compiler might have align the stack frame pointer to 16 bytes in case
        //     // there is a call instruction in there
        //     options(nostack),
        // );
        // mask
    }

    #[cfg(test)]
    mod tests {
        use std::{
            fs::File,
            io::{BufReader, Read},
        };

        use super::*;

        fn get_state() -> State {
            let file = File::open("../input.json")
                .unwrap_or_else(|_| File::open("monte-carlo-sim/input.json").unwrap());
            let mut file = BufReader::new(file);
            let mut buf = Vec::with_capacity(512);
            file.read_to_end(&mut buf).unwrap();

            let teams_dto = serde_json::from_slice::<Vec<TeamDto>>(&buf).unwrap();

            const ITERATIONS: usize = 32;
            State::new(&teams_dto)
        }

        #[test]
        fn size() {
            let state = get_state();
            let size = state.size_of();

            // As per lscpu, I have 384KiB total L1 data cache, across 8 cores
            let l1d_sum_kb = 384usize;
            let core_l1d_size_b = (l1d_sum_kb * 1024) / 8;

            // This machine has hyperthreading though, so a different logical core
            // might compete over L1 cache resources, so I atleast want to stick
            // to less than half of the L1 cache for a physical core (there are 16 logical cores)
            assert!(size < (core_l1d_size_b / 2));
        }
    }
}
