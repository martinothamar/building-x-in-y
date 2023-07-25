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

    pub struct State {
        rng: RngImpl,
        number_of_teams: u32,
        poisson_len: u32,
        table_len: u32,
        home_poisson: *mut f64, // len = next_multiple_of(len(teams), 8)
        away_poisson: *mut f64, // len = next_multiple_of(len(teams), 8)
        table: *mut f64,
    }

    impl State {
        fn size_of(&self) -> usize {
            mem::size_of::<Self>()
                + (self.poisson_len as usize * mem::size_of::<f64>())
                + (self.poisson_len as usize * mem::size_of::<f64>())
                + (self.table_len as usize * mem::size_of::<f64>())
        }
    }

    impl Drop for State {
        fn drop(&mut self) {
            unsafe {
                let poisson_layout =
                    alloc::Layout::from_size_align(size_of::<f64>() * self.poisson_len as usize, ALIGN).unwrap();
                let table_layout =
                    alloc::Layout::from_size_align(size_of::<f64>() * self.table_len as usize, ALIGN).unwrap();

                alloc::dealloc(self.home_poisson.cast(), poisson_layout);
                alloc::dealloc(self.away_poisson.cast(), poisson_layout);
                alloc::dealloc(self.table.cast(), table_layout);
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

                let poisson_layout = alloc::Layout::from_size_align(size_of::<f64>() * poisson_len, ALIGN).unwrap();
                let table_layout = alloc::Layout::from_size_align(size_of::<f64>() * table_len, ALIGN).unwrap();

                let home_poisson_ptr = alloc::alloc(poisson_layout).cast::<f64>();
                let away_poisson_ptr = alloc::alloc(poisson_layout).cast::<f64>();
                let table_ptr = alloc::alloc(table_layout).cast::<f64>();

                let home_poisson = slice::from_raw_parts_mut(home_poisson_ptr.cast(), poisson_len);
                let away_poisson = slice::from_raw_parts_mut(away_poisson_ptr.cast(), poisson_len);
                let table = slice::from_raw_parts_mut(table_ptr, table_len);

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

                for i in 0..table_len {
                    table[i] = 0f64;
                }

                Self {
                    rng: RngImpl::from_seed(seed),
                    number_of_teams: teams.len() as u32,
                    poisson_len: poisson_len as u32,
                    table_len: table_len as u32,
                    home_poisson: home_poisson_ptr,
                    away_poisson: away_poisson_ptr,
                    table: table_ptr,
                }
            }
        }

        pub fn reset_table(&mut self) {
            unsafe {
                for i in 0..self.table_len {
                    *self.table.add(i as usize) = 0f64;
                }
            }
        }
    }

    #[inline(never)]
    pub fn simulate<const S: usize>(state: &mut State) {
        unsafe {
            // assert!(state.matches.poisson.len() == state.matches.score.len());

            // let poisson = state.matches.poisson.as_ptr();
            // let scores = state.matches.score.as_mut_ptr();

            let home_poisson = state.home_poisson;
            let away_poisson = state.away_poisson;
            let table_ptr = state.table;

            assert!(state.poisson_len % 8 == 0);
            assert!(state.number_of_teams < state.poisson_len);
            let len = state.number_of_teams;

            let table = slice::from_raw_parts_mut(table_ptr, len as usize);

            for _ in 0..S {
                tick(&mut state.rng, home_poisson, away_poisson, table_ptr, len);

                // TODO copy table
                // let results = table
                //     .iter()
                //     .map(|v| *v as i16)
                //     .enumerate()
                //     .sorted_unstable_by_key(|a| -a.1);

                state.reset_table();
            }
        }
    }

    #[inline(always)]
    unsafe fn tick(rng: &mut RngImpl, home_poisson: *mut f64, away_poisson: *mut f64, table: *mut f64, len: u32) {
        let _table_res = slice::from_raw_parts(table, len as usize);

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

                // let exclude_masks = format!("{:#010b}", exclude_mask);

                let away_poisson = _mm512_load_pd(away_poisson.add(j as usize));

                let mut home_goals = _mm512_setzero_pd();
                let mut away_goals = _mm512_setzero_pd();

                simulate_sides(home_poisson, &mut home_goals, rng);
                simulate_sides(away_poisson, &mut away_goals, rng);

                let home = _mm512_cmp_pd_mask::<_CMP_GT_OQ>(home_goals, away_goals);
                let draw = _mm512_cmp_pd_mask::<_CMP_EQ_OQ>(home_goals, away_goals);
                let away = !home & !draw;

                let draw_mask = draw & !exclude_mask;
                // let draw_masks = format!("{:#010b}", draw_mask);

                // Home
                let home_mask = home & !exclude_mask;
                // let home_masks = format!("{:#010b}", home_mask);
                home_points = _mm512_mask_add_pd(home_points, home_mask, home_points, _mm512_set1_pd(3.0));
                home_points = _mm512_mask_add_pd(home_points, draw_mask, home_points, _mm512_set1_pd(1.0));

                // Away
                let mut away_points = _mm512_setzero_pd();
                let away_mask = away & !exclude_mask;
                // let away_masks = format!("{:#010b}", away_mask);
                away_points = _mm512_mask_add_pd(away_points, away_mask, away_points, _mm512_set1_pd(3.0));
                away_points = _mm512_mask_add_pd(away_points, draw_mask, away_points, _mm512_set1_pd(1.0));

                let table_section = _mm512_load_pd(table.add(j as usize));
                _mm512_store_pd(table.add(j as usize), _mm512_add_pd(table_section, away_points));

                j += 8;
                // dbg!(
                //     home_points,
                //     away_points,
                //     home_masks,
                //     draw_masks,
                //     away_masks,
                //     exclude_masks
                // );
            }

            *table.add(i as usize) += _mm512_reduce_add_pd(home_points);

            // TODO - remainders of i, j?
        }

        // dbg!(table_res);
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

        use itertools::Itertools;

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
        fn bitstuff() {
            fn ha(a: u8, b: u8) -> u16 {
                let a = a as u16;
                let b = b as u16;
                let sub = (1 * ((b - a) / 20)) + 1;
                let r = a * 20 + b - sub;
                let z = r * r;
                return r;
            }

            fn har(r: u16) -> (u8, u8) {
                let a = (r - r % 20) / 20;
                let b = r % 20;
                (a as u8, b as u8)
            }

            let mut has = Vec::new();
            for i in 0..20 {
                for j in 0..20 {
                    if i == j {
                        continue;
                    }

                    let ha = ha(i, j);
                    let (i2, j2) = har(ha);
                    // let r = (i, j, ha, i2, j2);
                    has.push(ha);
                }
            }

            let binding = has.iter().counts();
            let gaps = has.iter().tuple_windows().filter(|&(a, b)| b - a > 1).collect_vec();
            let dups = binding.iter().filter(|&(_, c)| *c > 1).collect_vec();

            dbg!(gaps, dups);
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
