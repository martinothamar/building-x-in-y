#![allow(dead_code)]
#![feature(stdsimd)]
#![feature(allocator_api)]

use serde::Deserialize;

#[derive(Deserialize)]
pub struct TeamDto {
    pub name: String,
    #[serde(alias = "expectedGoals")]
    pub expected_goals: f64,
}

pub mod sim {
    use itertools::Itertools;
    use mem::size_of;
    use std::arch::x86_64::*;
    use std::mem::{self, transmute};
    use std::ops::Neg;
    use std::{alloc, slice};

    use rand::{RngCore, SeedableRng};
    use simd_rand::specific::avx512::*;

    use crate::TeamDto;

    const ALIGN: usize = 64;
    pub const HOME_ADVANTAGE: f64 = 0.25;

    type RngImpl = Xoshiro256PlusX8;
    type RngImplSeed = Xoshiro256PlusX8Seed;

    pub enum MarketType {
        Winner,
        Top4,
    }

    pub struct Market {
        market_type: MarketType,
        outcomes: Vec<Outcome>,
    }

    impl Market {
        pub fn new_collection() -> Vec<Self> {
            Vec::with_capacity(4)
        }
    }

    pub struct Outcome {
        team_idx: usize,
        probability: f64,
    }

    pub struct State {
        rng: RngImpl,
        number_of_teams: usize,
        poisson_len: u32,
        table_len: u32,
        sorted_table_len: u32,
        table_position_history_len: u32,
        home_poisson: *mut f64,
        away_poisson: *mut f64,
        table: *mut f64,
        sorted_table: *mut (u8, i8),
        table_position_history: *mut u16,
    }

    impl State {
        fn size_of(&self) -> usize {
            mem::size_of::<Self>()
                + (self.poisson_len as usize * mem::size_of::<f64>())
                + (self.poisson_len as usize * mem::size_of::<f64>())
                + (self.table_len as usize * mem::size_of::<f64>())
                + (self.sorted_table_len as usize * mem::size_of::<(u8, i8)>())
                + (self.table_position_history_len as usize * mem::size_of::<u16>())
        }
    }

    impl Drop for State {
        fn drop(&mut self) {
            unsafe {
                let poisson_layout =
                    alloc::Layout::from_size_align(size_of::<f64>() * self.poisson_len as usize, ALIGN).unwrap();
                let table_layout =
                    alloc::Layout::from_size_align(size_of::<f64>() * self.table_len as usize, ALIGN).unwrap();
                let sorted_table_layout =
                    alloc::Layout::from_size_align(size_of::<(u8, i8)>() * self.sorted_table_len as usize, ALIGN)
                        .unwrap();
                let table_position_history_layout =
                    alloc::Layout::from_size_align(size_of::<u16>() * self.table_position_history_len as usize, ALIGN)
                        .unwrap();

                alloc::dealloc(self.home_poisson.cast(), poisson_layout);
                alloc::dealloc(self.away_poisson.cast(), poisson_layout);
                alloc::dealloc(self.table.cast(), table_layout);
                alloc::dealloc(self.sorted_table.cast(), sorted_table_layout);
                alloc::dealloc(self.table_position_history.cast(), table_position_history_layout);
            }
        }
    }

    impl State {
        pub fn new(teams: &[TeamDto]) -> Self {
            unsafe {
                let mut seed: RngImplSeed = Default::default();
                rand::thread_rng().fill_bytes(&mut *seed);

                let poisson_len = next_multiple_of(teams.len(), 8);
                let table_len = next_multiple_of(teams.len(), 8);
                let sorted_table_len = teams.len();
                let table_position_history_len = teams.len() * teams.len();

                let poisson_layout = alloc::Layout::from_size_align(size_of::<f64>() * poisson_len, ALIGN).unwrap();
                let table_layout = alloc::Layout::from_size_align(size_of::<f64>() * table_len, ALIGN).unwrap();
                let sorted_table_layout =
                    alloc::Layout::from_size_align(size_of::<(u8, i8)>() * sorted_table_len, ALIGN).unwrap();
                let table_position_history_layout =
                    alloc::Layout::from_size_align(size_of::<u16>() * table_position_history_len, ALIGN).unwrap();

                let home_poisson_ptr = alloc::alloc(poisson_layout).cast::<f64>();
                let away_poisson_ptr = alloc::alloc(poisson_layout).cast::<f64>();
                let table_ptr = alloc::alloc_zeroed(table_layout).cast::<f64>();
                let sorted_table_ptr = alloc::alloc_zeroed(sorted_table_layout).cast::<(u8, i8)>();
                let table_position_history_ptr = alloc::alloc_zeroed(table_position_history_layout).cast::<u16>();

                let home_poisson = slice::from_raw_parts_mut(home_poisson_ptr.cast(), poisson_len);
                let away_poisson = slice::from_raw_parts_mut(away_poisson_ptr.cast(), poisson_len);

                for i in 0..poisson_len {
                    if i < teams.len() {
                        home_poisson[i] = (teams[i].expected_goals + HOME_ADVANTAGE).neg().exp();
                    } else {
                        home_poisson[i] = 1.0;
                    }
                }

                for j in 0..poisson_len {
                    if j < teams.len() {
                        away_poisson[j] = teams[j].expected_goals.neg().exp();
                    } else {
                        away_poisson[j] = 1.0;
                    }
                }

                Self {
                    rng: RngImpl::from_seed(seed),
                    number_of_teams: teams.len(),
                    poisson_len: poisson_len as u32,
                    table_len: table_len as u32,
                    sorted_table_len: sorted_table_len as u32,
                    table_position_history_len: table_position_history_len as u32,
                    home_poisson: home_poisson_ptr,
                    away_poisson: away_poisson_ptr,
                    table: table_ptr,
                    sorted_table: sorted_table_ptr,
                    table_position_history: table_position_history_ptr,
                }
            }
        }

        pub fn reset(&mut self) {
            unsafe {
                for i in 0..self.table_position_history_len {
                    *self.table_position_history.add(i as usize) = 0;
                }
            }
        }

        fn reset_table(&mut self) {
            unsafe {
                for i in 0..self.table_len {
                    *self.table.add(i as usize) = 0.;
                }
            }
        }
    }

    #[inline(never)]
    pub fn simulate<const S: usize>(state: &mut State, markets: &mut Vec<Market>) -> u16 {
        if markets.len() > 0 {
            markets.clear();
        }

        unsafe {
            let home_poisson = state.home_poisson;
            let away_poisson = state.away_poisson;
            let table_ptr = state.table;

            assert!(state.poisson_len % 8 == 0);
            assert!(state.number_of_teams < state.poisson_len as usize);
            let len = state.number_of_teams as u32;

            let table = slice::from_raw_parts_mut(table_ptr, len as usize);
            let sorted_table = state.sorted_table;
            let table_pos_history = state.table_position_history;

            for _ in 0..S {
                tick(&mut state.rng, home_poisson, away_poisson, table_ptr, len);

                let results = table
                    .iter()
                    .take(state.number_of_teams as usize)
                    .map(|v| *v as i8)
                    .enumerate()
                    .map(|(i, v)| (i as u8, v))
                    .sorted_unstable_by_key(|a| -a.1);

                std::ptr::copy(results.as_ref().as_ptr(), sorted_table, state.number_of_teams as usize);

                for p in 0..state.number_of_teams {
                    let (i, _) = *sorted_table.add(p);
                    let idx = i as u32 * len + p as u32;
                    *table_pos_history.add(idx as usize) += 1;
                }

                state.reset_table();
            }

            extract_markets::<S>(state, markets);

            *table_pos_history
        }
    }

    #[inline(always)]
    unsafe fn tick(rng: &mut RngImpl, home_poisson: *mut f64, away_poisson: *mut f64, table: *mut f64, len: u32) {
        for i in 0..len {
            let home_poisson = _mm512_set1_pd(*home_poisson.add(i as usize));
            let mut home_points = _mm512_setzero_pd();

            let mut j = 0u32;
            while j < len {
                let exclude_mask = if i >= j && i < j + 8 {
                    let pos = j + 8 - i;
                    0b1000_0000u8 >> (pos - 1)
                } else {
                    0b0000_0000u8
                };

                let away_poisson = _mm512_load_pd(away_poisson.add(j as usize));

                let mut home_goals = _mm512_setzero_pd();
                let mut away_goals = _mm512_setzero_pd();

                simulate_sides(home_poisson, &mut home_goals, rng);
                simulate_sides(away_poisson, &mut away_goals, rng);

                let home = _mm512_cmp_pd_mask::<_CMP_GT_OQ>(home_goals, away_goals);
                let draw = _mm512_cmp_pd_mask::<_CMP_EQ_OQ>(home_goals, away_goals);
                let away = !home & !draw;

                let draw_mask = draw & !exclude_mask;

                // Home
                let home_mask = home & !exclude_mask;
                home_points = _mm512_mask_add_pd(home_points, home_mask, home_points, _mm512_set1_pd(3.0));
                home_points = _mm512_mask_add_pd(home_points, draw_mask, home_points, _mm512_set1_pd(1.0));

                // Away
                let mut away_points = _mm512_setzero_pd();
                let away_mask = away & !exclude_mask;
                away_points = _mm512_mask_add_pd(away_points, away_mask, away_points, _mm512_set1_pd(3.0));
                away_points = _mm512_mask_add_pd(away_points, draw_mask, away_points, _mm512_set1_pd(1.0));

                let table_section = _mm512_load_pd(table.add(j as usize));
                _mm512_store_pd(table.add(j as usize), _mm512_add_pd(table_section, away_points));

                j += 8;
            }

            *table.add(i as usize) += _mm512_reduce_add_pd(home_points);
        }
    }

    #[inline(always)]
    unsafe fn simulate_sides(poisson_vec: __m512d, goals: &mut __m512d, rng: &mut RngImpl) {
        let mut product_vec = rng.next_m512d();

        loop {
            let sub = _mm512_sub_pd(product_vec, poisson_vec);
            let mask = mm512_movemask_pd(sub);
            if mask == 0xFF {
                break;
            }

            *goals = _mm512_add_pd(*goals, _mm512_roundscale_pd::<2>(sub));

            let next_product_vec = rng.next_m512d();
            product_vec = _mm512_mul_pd(product_vec, next_product_vec);
        }
    }

    #[inline(never)]
    unsafe fn extract_markets<const S: usize>(state: &State, markets: &mut Vec<Market>) {
        assert!(markets.len() == 0);

        let table_position_history = state.table_position_history;
        {
            // Winner - probability of winning the season
            let mut outcomes = Vec::with_capacity(state.number_of_teams); // TODO - smarter allocations
            for i in 0..state.number_of_teams {
                let idx = i * state.number_of_teams + 0; // 0 for the winner position
                let number_of_wins = *table_position_history.add(idx);
                if number_of_wins == 0 {
                    continue;
                }
                let probability = number_of_wins as f64 / S as f64;
                if probability < 0.01 {
                    // Very low probabilities are typically filtered out
                    continue;
                }
                outcomes.push(Outcome {
                    team_idx: i,
                    probability,
                })
            }
            markets.push(Market {
                market_type: MarketType::Winner,
                outcomes,
            });
        }

        {
            // Top 4 - probability of completing the season in the top 4
            let mut outcomes = Vec::with_capacity(state.number_of_teams);
            for i in 0..state.number_of_teams {
                let base_idx = i * state.number_of_teams; // 0 for the winner position
                let top_4_state = [
                    *table_position_history.add(base_idx + 0),
                    *table_position_history.add(base_idx + 1),
                    *table_position_history.add(base_idx + 2),
                    *table_position_history.add(base_idx + 3),
                ];
                let probability = top_4_state.iter().map(|&v| v as u32).sum::<u32>() as f64 / S as f64;
                if probability < 0.01 {
                    continue;
                }
                outcomes.push(Outcome {
                    team_idx: i,
                    probability,
                })
            }
            markets.push(Market {
                market_type: MarketType::Top4,
                outcomes,
            });
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

    fn next_multiple_of(num: usize, multiple: usize) -> usize {
        let remainder = num % multiple;
        match remainder {
            0 => num,
            v => num + multiple - v,
        }
    }

    #[cfg(test)]
    mod tests {
        use std::{
            fs::File,
            io::{BufReader, Read},
        };

        use super::*;

        fn get_state() -> State {
            let file =
                File::open("../input.json").unwrap_or_else(|_| File::open("monte-carlo-sim/input.json").unwrap());
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
            // to less than half of the L1 cache for a physical core, just in case (there are 16 logical cores)
            assert!(size < (core_l1d_size_b / 2));
        }

        #[test]
        fn actually_runs() {
            let mut state = get_state();

            let mut markets = Market::new_collection();
            let first_seed_wins = simulate::<1000>(&mut state, &mut markets);
            assert!(first_seed_wins > 100 / 10);
            assert!(markets.len() > 0);
        }

        // TODO memory management tests
    }
}
