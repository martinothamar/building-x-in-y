#![allow(dead_code)]

use std::time::{Duration, Instant};

use color_eyre::eyre::Result;
use serde::Deserialize;
use tokio::{
    fs::File,
    io::{AsyncReadExt, BufReader},
};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let file = File::open("../input.json").await?;
    let mut file = BufReader::new(file);
    let mut buf = Vec::with_capacity(512);
    file.read_to_end(&mut buf).await?;

    let teams_dto = serde_json::from_slice::<Vec<TeamDto>>(&buf)?;

    const ITERATIONS: usize = 16;
    let mut state = sim4::State::new(&teams_dto);
    let mut elapsed = [Duration::ZERO; ITERATIONS];

    for i in 0..ITERATIONS {
        let start = Instant::now();
        sim4::simulate::<100_000>(&mut state);
        let stop = Instant::now();
        let duration = stop.duration_since(start);
        elapsed[i] = duration;
    }

    for i in 0..ITERATIONS {
        println!("Elapsed: {:.3}ms", elapsed[i].as_secs_f64() * 1000.0);
    }

    Ok(())
}

#[derive(Deserialize)]
pub struct TeamDto {
    pub name: String,
    #[serde(alias = "expectedGoals")]
    pub expected_goals: f64,
}

mod sim {
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

        poisson: Vec<f64>,
        matches: Vec<u8>,
        scores: Vec<u8>,
    }

    impl State {
        pub fn new(teams: &[TeamDto]) -> Self {
            let number_of_matches = (teams.len() - 1) * teams.len();
            let mut seed: RngImplSeed = Default::default();
            rand::thread_rng().fill_bytes(&mut *seed);

            let mut poisson = vec![0.0; teams.len() * 2];
            let mut matches = vec![0; number_of_matches * 2];
            let scores = vec![0; number_of_matches * 2];

            for i in 0..teams.len() {
                let index = i * 2;
                poisson[index + 0] = (teams[i].expected_goals + HOME_ADVANTAGE).neg().exp();
                poisson[index + 1] = teams[i].expected_goals.neg().exp();
            }

            let mut matchups = HashSet::with_capacity(number_of_matches);

            let mut match_index: usize = 0;
            for i in 0..teams.len() {
                for j in 0..teams.len() {
                    if i == j {
                        continue;
                    }

                    if matchups.insert((i as u8, j as u8)) {
                        matches[match_index + 0] = i as u8;
                        matches[match_index + 1] = j as u8;
                        match_index += 2;
                    }
                }
            }

            Self {
                rng: RngImpl::from_seed(seed),
                poisson: poisson,
                matches: matches,
                scores: scores,
            }
        }
    }

    #[inline(never)]
    pub fn simulate<const S: usize>(state: &mut State) {
        unsafe {
            let mut goals;
            let mut goals_mem: F64x4 = Default::default();

            for _ in 0..S {
                for i in (0..state.matches.len()).step_by(4) {
                    let home_id_1 = *state.matches.get_unchecked(i + 0);
                    let away_id_1 = *state.matches.get_unchecked(i + 1);
                    let home_poisson_index_1 = (home_id_1 * 2) as usize;
                    let away_poisson_index_1 = (away_id_1 * 2) as usize;
                    let home_1 = *state.poisson.get_unchecked(home_poisson_index_1 + 0);
                    let away_1 = *state.poisson.get_unchecked(away_poisson_index_1 + 1);
                    debug_assert!(home_1 != 0.0);
                    debug_assert!(away_1 != 0.0);

                    let home_id_2 = *state.matches.get_unchecked(i + 2);
                    let away_id_2 = *state.matches.get_unchecked(i + 3);
                    let home_poisson_index_2 = (home_id_2 * 2) as usize;
                    let away_poisson_index_2 = (away_id_2 * 2) as usize;
                    let home_2 = *state.poisson.get_unchecked(home_poisson_index_2 + 0);
                    let away_2 = *state.poisson.get_unchecked(away_poisson_index_2 + 1);
                    debug_assert!(home_2 != 0.0);
                    debug_assert!(away_2 != 0.0);

                    let poisson_vec = _mm256_set_pd(home_1, away_1, home_2, away_2);
                    goals = _mm256_setzero_pd();

                    simulate_match(&poisson_vec, &mut goals, &mut state.rng);

                    _mm256_store_pd(goals_mem.as_mut_ptr(), goals);

                    *state.scores.get_unchecked_mut(i + 0) = goals_mem[0] as u8;
                    *state.scores.get_unchecked_mut(i + 1) = goals_mem[1] as u8;
                    *state.scores.get_unchecked_mut(i + 2) = goals_mem[2] as u8;
                    *state.scores.get_unchecked_mut(i + 3) = goals_mem[3] as u8;
                }
            }
        }

        state.scores.fill(0);
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

mod sim2 {
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

        matches: Vec<Match>,
    }

    #[derive(Default, Clone)]
    struct Match {
        home_poisson: f64,
        away_poisson: f64,
        home: u8,
        away: u8,
        home_score: u8,
        away_score: u8,
    }

    impl State {
        pub fn new(teams: &[TeamDto]) -> Self {
            let number_of_matches = (teams.len() - 1) * teams.len();
            let mut seed: RngImplSeed = Default::default();
            rand::thread_rng().fill_bytes(&mut *seed);

            let mut matches: Vec<Match> = vec![Default::default(); number_of_matches];

            let mut matchups = HashSet::with_capacity(number_of_matches);

            let mut match_index: usize = 0;
            for i in 0..teams.len() {
                for j in 0..teams.len() {
                    if i == j {
                        continue;
                    }

                    if matchups.insert((i as u8, j as u8)) {
                        let m = &mut matches[match_index];

                        m.home_poisson = (teams[i].expected_goals + HOME_ADVANTAGE).neg().exp();
                        m.away_poisson = teams[j].expected_goals.neg().exp();
                        m.home = i as u8;
                        m.away = j as u8;
                        m.home_score = 0;
                        m.away_score = 0;
                        match_index += 1;
                    }
                }
            }

            Self {
                rng: RngImpl::from_seed(seed),
                matches: matches,
            }
        }
    }

    #[inline(never)]
    pub fn simulate<const S: usize>(state: &mut State) {
        unsafe {
            let mut goals;
            let mut goals_mem: F64x4 = Default::default();

            let matches = state.matches.as_mut_ptr();
            for _ in 0..S {
                for i in (0..state.matches.len()).step_by(2) {
                    let m1 = &mut *matches.add(i + 0);
                    let m2 = &mut *matches.add(i + 1);

                    let poisson_vec = _mm256_set_pd(
                        m1.home_poisson,
                        m1.away_poisson,
                        m2.home_poisson,
                        m2.away_poisson,
                    );
                    goals = _mm256_setzero_pd();

                    simulate_match(&poisson_vec, &mut goals, &mut state.rng);

                    _mm256_store_pd(goals_mem.as_mut_ptr(), goals);

                    m1.home_score = goals_mem[0] as u8;
                    m1.away_score = goals_mem[1] as u8;
                    m2.home_score = goals_mem[2] as u8;
                    m2.away_score = goals_mem[3] as u8;
                }
            }
        }

        for m in &mut state.matches {
            m.home_score = 0;
            m.away_score = 0;
        }
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

mod sim3 {
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
        home_poisson: Vec<f64>,
        away_poisson: Vec<f64>,
        home: Vec<u8>,
        away: Vec<u8>,
        home_score: Vec<u8>,
        away_score: Vec<u8>,
    }

    impl Matches {
        pub fn new(number_of_matches: usize) -> Self {
            let home_poisson = vec![0.0; number_of_matches];
            let away_poisson = vec![0.0; number_of_matches];
            let home = vec![0u8; number_of_matches];
            let away = vec![0u8; number_of_matches];
            let home_score = vec![0u8; number_of_matches];
            let away_score = vec![0u8; number_of_matches];
            Self {
                home_poisson,
                away_poisson,
                home,
                away,
                home_score,
                away_score,
            }
        }

        pub fn init(&mut self, m: usize, home_poisson: f64, away_poisson: f64, home: u8, away: u8) {
            unsafe {
                *self.home_poisson.get_unchecked_mut(m) = home_poisson;
                *self.away_poisson.get_unchecked_mut(m) = away_poisson;
                *self.home.get_unchecked_mut(m) = home;
                *self.away.get_unchecked_mut(m) = away;
                *self.home_score.get_unchecked_mut(m) = 0;
                *self.away_score.get_unchecked_mut(m) = 0;
            }
        }

        pub fn get_home_poisson(&self, m: usize) -> f64 {
            unsafe { *self.home_poisson.get_unchecked(m) }
        }

        pub fn get_away_poisson(&self, m: usize) -> f64 {
            unsafe { *self.away_poisson.get_unchecked(m) }
        }

        pub fn set_home_goals(&mut self, m: usize, goals: u8) {
            unsafe {
                *self.home_score.get_unchecked_mut(m) = goals;
            }
        }

        pub fn set_away_goals(&mut self, m: usize, goals: u8) {
            unsafe {
                *self.away_score.get_unchecked_mut(m) = goals;
            }
        }

        pub fn len(&self) -> usize {
            self.home_poisson.len()
        }

        pub fn reset_scores(&mut self) {
            self.home_score.fill(0);
            self.away_score.fill(0);
        }
    }

    impl State {
        pub fn new(teams: &[TeamDto]) -> Self {
            let number_of_matches = (teams.len() - 1) * teams.len();
            let mut seed: RngImplSeed = Default::default();
            rand::thread_rng().fill_bytes(&mut *seed);

            let mut matches = Matches::new(number_of_matches);

            let mut matchups = HashSet::with_capacity(number_of_matches);

            let mut match_index: usize = 0;
            for i in 0..teams.len() {
                for j in 0..teams.len() {
                    if i == j {
                        continue;
                    }

                    if matchups.insert((i as u8, j as u8)) {
                        matches.init(
                            match_index,
                            (teams[i].expected_goals + HOME_ADVANTAGE).neg().exp(),
                            teams[j].expected_goals.neg().exp(),
                            i as u8,
                            j as u8,
                        );
                        match_index += 1;
                    }
                }
            }

            Self {
                rng: RngImpl::from_seed(seed),
                matches: matches,
            }
        }
    }

    #[inline(never)]
    pub fn simulate<const S: usize>(state: &mut State) {
        unsafe {
            let mut goals;
            let mut goals_mem: F64x4 = Default::default();

            for _ in 0..S {
                for i in (0..state.matches.len()).step_by(2) {
                    let poisson_vec = _mm256_set_pd(
                        state.matches.get_home_poisson(i),
                        state.matches.get_away_poisson(i),
                        state.matches.get_home_poisson(i + 1),
                        state.matches.get_away_poisson(i + 1),
                    );
                    goals = _mm256_setzero_pd();

                    simulate_match(&poisson_vec, &mut goals, &mut state.rng);

                    _mm256_store_pd(goals_mem.as_mut_ptr(), goals);

                    state.matches.set_home_goals(i, goals_mem[0] as u8);
                    state.matches.set_away_goals(i, goals_mem[1] as u8);
                    state.matches.set_home_goals(i + 1, goals_mem[2] as u8);
                    state.matches.set_away_goals(i + 1, goals_mem[3] as u8);
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

mod sim4 {
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
