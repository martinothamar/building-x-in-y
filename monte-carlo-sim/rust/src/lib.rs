#![allow(dead_code)]
#![feature(stdsimd)]
#![feature(allocator_api)]

use serde::Deserialize;

/// A DTO for deserializing from the input.json file
#[derive(Deserialize)]
pub struct TeamDto {
    pub name: String,
    #[serde(alias = "expectedGoals")]
    pub expected_goals: f64,
}

pub mod sim {
    use bumpalo::Bump;
    use itertools::Itertools;
    use mem::size_of;
    use std::arch::x86_64::*;
    use std::mem::{self, transmute};
    use std::ops::Neg;
    use std::ptr::NonNull;
    use std::{alloc, slice};

    use rand::{RngCore, SeedableRng};
    use simd_rand::specific::avx512::*;

    use crate::TeamDto;

    // Memory should be aligned to 64 bytes, benefits:
    // - cacheline size aligned
    // - efficient aligned loads into SIMD vectors (512bit wide vectors require this for normal load operations)
    const ALIGN: usize = 64;

    // We expect 0.25 more goals from teams on average when they are playing at home
    pub const HOME_ADVANTAGE: f64 = 0.25;

    type RngImpl = Xoshiro256PlusX8;
    type RngImplSeed = Xoshiro256PlusX8Seed;

    type MarketVec<'a> = Vec<Market<'a>, &'a Bump>;
    type OutcomeVec<'a> = Vec<Outcome, &'a Bump>;

    // Simulation returns slices into memory allocated
    // in the 'markets_allocator' arena allocator
    pub type Markets<'a> = &'a[Market<'a>];
    pub type Outcomes<'a> = &'a[Outcome];

    pub enum MarketType {
        Winner,
        Top4,
    }

    pub struct Market<'a> {
        pub market_type: MarketType,
        pub outcomes: Outcomes<'a>,
    }

    pub struct Outcome {
        // Index from the teams input array
        pub team_idx: usize,
        // Probability range 0..1
        pub probability: f64,
    }

    pub struct State {
        rng: RngImpl,
        number_of_teams: usize,
        // We do manual memory allocation
        // to keep alignment requirements
        // Allocating Vec's to alignment seems to be a big pain..
        // And we have arena allocators that make this pretty effortless
        // Raw pointers also let us avoid bounds checks,
        // which admittedly probably wouldn't impact performance much
        poisson_len: u32,
        table_len: u32,
        sorted_table_len: u32,
        table_position_history_len: u32,
        home_poisson: NonNull<f64>,
        away_poisson: NonNull<f64>,
        table: NonNull<f64>,
        sorted_table: NonNull<(u8, i8)>,
        table_position_history: NonNull<u16>,
    }

    impl State {
        fn size_of(&self) -> usize {
            // It's important that the size of our state
            // fits completely in L1 cache, so there's a unit-test for this.
            // All of these fields are used in the inner loops
            mem::size_of::<Self>()
                + (self.poisson_len as usize * mem::size_of::<f64>())
                + (self.poisson_len as usize * mem::size_of::<f64>())
                + (self.table_len as usize * mem::size_of::<f64>())
                + (self.sorted_table_len as usize * mem::size_of::<(u8, i8)>())
                + (self.table_position_history_len as usize * mem::size_of::<u16>())
        }
    }

    pub fn new_allocator() -> Bump {
        // I have 4k pages on this system.
        // I should still have tests
        // that ensure that these arenas don't grow
        // as a result of simulation
        const ALLOC: usize = 1024 * 4;
        let allocator = Bump::with_capacity(ALLOC);
        allocator.set_allocation_limit(Some(ALLOC));
        allocator
    }

    impl State {
        pub fn new(allocator: &mut Bump, teams: &[TeamDto]) -> Self {
            unsafe {
                let mut seed: RngImplSeed = Default::default();
                rand::thread_rng().fill_bytes(&mut *seed);

                // Length needs to be a multiple of 8,
                // since we iterate through and load these numbers into 512bit vectors.
                // Some of these vectors will have irrelevant data, but those are masked off/not used.
                let poisson_len = next_multiple_of(teams.len(), 8);
                let table_len: usize = next_multiple_of(teams.len(), 8);
                // Now vectorization, so doesn't need to be multiple of 8
                let sorted_table_len = teams.len();
                // The table history contains `teams.len()` items per team,
                // i.e. tracking number of placements at each spot in the table
                // indices are calculated by doing `team_idx * teams.len() + position`
                let table_position_history_len = teams.len() * teams.len();

                let poisson_layout = alloc::Layout::from_size_align(size_of::<f64>() * poisson_len, ALIGN).unwrap();
                let table_layout = alloc::Layout::from_size_align(size_of::<f64>() * table_len, ALIGN).unwrap();
                let sorted_table_layout =
                    alloc::Layout::from_size_align(size_of::<(u8, i8)>() * sorted_table_len, ALIGN).unwrap();
                let table_position_history_layout =
                    alloc::Layout::from_size_align(size_of::<u16>() * table_position_history_len, ALIGN).unwrap();

                // We allocate these through the arena, so that the arena allocator can do all the cleanup.
                let home_poisson_ptr = allocator.alloc_layout(poisson_layout).cast::<f64>();
                let away_poisson_ptr = allocator.alloc_layout(poisson_layout).cast::<f64>();
                let table_ptr = allocator.alloc_layout(table_layout).cast::<f64>();
                let sorted_table_ptr = allocator.alloc_layout(sorted_table_layout).cast::<(u8, i8)>();
                let table_position_history_ptr = allocator.alloc_layout(table_position_history_layout).cast::<u16>();

                // The poisson values are the L used when sampling from poisson distribution.
                // These values are compared to the random values.
                // We need two arrays since the home value is different (home advantage), but we can still precalculate both
                let home_poisson = slice::from_raw_parts_mut(home_poisson_ptr.as_ptr(), poisson_len);
                let away_poisson = slice::from_raw_parts_mut(away_poisson_ptr.as_ptr(), poisson_len);

                for i in 0..poisson_len {
                    if i < teams.len() {
                        home_poisson[i] = (teams[i].expected_goals + HOME_ADVANTAGE).neg().exp();
                    } else {
                        // Random numbers will be [0.0..1.0),
                        // so now the poisson algorithm condition will always exit
                        // i.e. we won't continue simulating on behalf of these vector lanes
                        home_poisson[i] = 1.0;
                    }
                }

                for j in 0..poisson_len {
                    if j < teams.len() {
                        away_poisson[j] = teams[j].expected_goals.neg().exp();
                    } else {
                        // Same as above
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

        /// Resets the state so that it can be reused for new simulation runs.
        pub fn reset(&mut self) {
            unsafe {
                let table_position_history = self.table_position_history.as_ptr();
                for i in 0..self.table_position_history_len {
                    *table_position_history.add(i as usize) = 0;
                }
            }
        }

        /// Reset the table, needs to be called inbetween simulation iterations (between each 0..S)
        fn reset_table(&mut self) {
            unsafe {
                let table = self.table.as_ptr();
                for i in 0..self.table_len {
                    *table.add(i as usize) = 0.;
                }
            }
        }
    }

    #[inline(never)]
    pub fn simulate<'a, const S: usize>(state: &mut State, markets_allocator: &'a mut Bump) -> Markets<'a> {
        unsafe {
            let home_poisson = state.home_poisson.as_ptr();
            let away_poisson = state.away_poisson.as_ptr();
            let table_ptr = state.table.as_ptr();

            assert!(state.poisson_len % 8 == 0);
            assert!(state.number_of_teams < state.poisson_len as usize);
            let len = state.number_of_teams as u32;

            let table = slice::from_raw_parts_mut(table_ptr, len as usize);
            let sorted_table = state.sorted_table.as_ptr();
            let table_pos_history = state.table_position_history.as_ptr();

            for _ in 0..S {
                // Simulate goals for all the matches of the season
                // this will update the table in the state
                tick(&mut state.rng, home_poisson, away_poisson, table_ptr, len);

                // Now we can sort the table and update the table history
                // results[0] won the season
                let results = table
                    .iter()
                    .take(state.number_of_teams as usize)
                    .map(|v| *v as i8)
                    .enumerate()
                    .map(|(i, v)| (i as u8, v))
                    .sorted_unstable_by_key(|a| -a.1);

                std::ptr::copy(results.as_ref().as_ptr(), sorted_table, state.number_of_teams as usize);

                for p in 0..state.number_of_teams {
                    // i is the index in the teams input slice
                    // so it is essentially the ID of the team
                    let (i, _) = *sorted_table.add(p);
                    // We have `teams.len()` positions in the table
                    let idx = i as u32 * len + p as u32;
                    *table_pos_history.add(idx as usize) += 1;
                }

                // Make all teams start at 0 again for the next iteration
                state.reset_table();
            }

            // Now we know where each team place i the table for all the S iterations in the simulation
            // so we can use these numbers to extract some basic market probabilities.
            let markets = extract_markets::<S>(state, markets_allocator);

            markets
        }
    }

    #[inline(always)]
    unsafe fn tick(rng: &mut RngImpl, home_poisson: *mut f64, away_poisson: *mut f64, table: *mut f64, len: u32) {
        for i in 0..len {
            // We simulate the season by making `i` the hometeam in 4 consecutive matches,
            // since the order of the matches don't really matter.
            // Hopefully the compiler arranges register allocation in a good way

            let home_poisson = _mm512_set1_pd(*home_poisson.add(i as usize));
            let mut home_points = _mm512_setzero_pd();

            let mut j = 0u32;
            while j < len {
                // i could be equal to j, in which case a team is playing itself
                // I think the easiest way to deal with this is to just mask
                // off the lanes in the vectors where i == j
                let exclude_mask = if i >= j && i < j + 8 {
                    let pos = j + 8 - i;
                    0b1000_0000u8 >> (pos - 1)
                } else {
                    0b0000_0000u8
                };

                // Efficiently load all the away teams into a vector
                let away_poisson = _mm512_load_pd(away_poisson.add(j as usize));

                let mut home_goals = _mm512_setzero_pd();
                let mut away_goals = _mm512_setzero_pd();

                // This simulates the goals by sampling from a poisson distribution
                // so after this, we have our result
                simulate_sides(home_poisson, &mut home_goals, rng);
                simulate_sides(away_poisson, &mut away_goals, rng);

                // home_goals > away_goals
                let home = _mm512_cmp_pd_mask::<_CMP_GT_OQ>(home_goals, away_goals);
                // home_goals == away_goals
                let draw = _mm512_cmp_pd_mask::<_CMP_EQ_OQ>(home_goals, away_goals);
                // if neither of the above, it should be a draw
                // so lets AND the NOT'ed bits
                let away = !home & !draw;

                // Still need to mask of the i == j case
                let draw_mask = draw & !exclude_mask;

                // Home
                let home_mask = home & !exclude_mask;
                // Conditionally adds points based on the mask - points are only added when the respective lane bit is set
                home_points = _mm512_mask_add_pd(home_points, home_mask, home_points, _mm512_set1_pd(3.0));
                home_points = _mm512_mask_add_pd(home_points, draw_mask, home_points, _mm512_set1_pd(1.0));

                // Away
                let mut away_points = _mm512_setzero_pd();
                let away_mask = away & !exclude_mask;
                away_points = _mm512_mask_add_pd(away_points, away_mask, away_points, _mm512_set1_pd(3.0));
                away_points = _mm512_mask_add_pd(away_points, draw_mask, away_points, _mm512_set1_pd(1.0));

                // Write the points for the away teams back to the table by doing load, add, store
                let table_section = _mm512_load_pd(table.add(j as usize));
                _mm512_store_pd(table.add(j as usize), _mm512_add_pd(table_section, away_points));

                j += 8;
            }

            // Horizontally add the home vector
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
    unsafe fn extract_markets<'a, const S: usize>(state: &State, markets_allocator: &'a mut Bump) -> Markets<'a> {
        let allocator: &'a Bump = markets_allocator;

        let markets: Vec<Market<'a>, &'a Bump> = MarketVec::with_capacity_in(4, allocator);
        let mut markets = std::mem::ManuallyDrop::new(markets);

        let table_position_history = state.table_position_history.as_ptr();
        {
            // Winner - probability of winning the season
            let outcomes: Vec<Outcome, &'a Bump> = OutcomeVec::with_capacity_in(state.number_of_teams, allocator);
            let mut outcomes = std::mem::ManuallyDrop::new(outcomes);
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
                outcomes: slice::from_raw_parts(outcomes.as_ptr(), outcomes.len()),
            });
        }

        {
            // Top 4 - probability of completing the season in the top 4
            let outcomes: Vec<Outcome, &'a Bump> = OutcomeVec::with_capacity_in(state.number_of_teams, allocator);
            let mut outcomes = std::mem::ManuallyDrop::new(outcomes);
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
                outcomes: slice::from_raw_parts(outcomes.as_ptr(), outcomes.len()),
            });
        }

        slice::from_raw_parts(markets.as_ptr(), markets.len())
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

        fn get_state(allocator: &mut Bump) -> State {
            let file =
                File::open("../input.json").unwrap_or_else(|_| File::open("monte-carlo-sim/input.json").unwrap());
            let mut file = BufReader::new(file);
            let mut buf = Vec::with_capacity(512);
            file.read_to_end(&mut buf).unwrap();

            let teams_dto = serde_json::from_slice::<Vec<TeamDto>>(&buf).unwrap();

            const ITERATIONS: usize = 32;
            State::new(allocator, &teams_dto)
        }

        #[test]
        fn size() {
            let mut allocator = new_allocator();
            let state = get_state(&mut allocator);
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
            let mut state_allocator = new_allocator();
            let mut markets_allocator = new_allocator();

            let mut state = get_state(&mut state_allocator);

            for _ in 1..=10 {
                let markets = simulate::<10_000>(&mut state, &mut markets_allocator);
                assert!(markets.len() == 2);

                state.reset();
                markets_allocator.reset();
            }
        }

        // TODO memory management tests
    }
}
