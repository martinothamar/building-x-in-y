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
    let mut state = sim::State::new(&teams_dto);
    let mut elapsed: [Duration; ITERATIONS] = Default::default();

    for i in 0..ITERATIONS {
        let start = Instant::now();
        sim::simulate::<100_000>(&mut state);
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

    pub struct State {
        rng: Xoshiro256PlusPlusX4,

        poisson: Vec<f64>,
        matches: Vec<u8>,
        scores: Vec<u8>,
    }

    impl State {
        pub fn new(teams: &[TeamDto]) -> Self {
            let number_of_matches = (teams.len() - 1) * teams.len();
            let mut seed: Xoshiro256PlusPlusX4Seed = Default::default();
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
                rng: Xoshiro256PlusPlusX4::from_seed(seed),
                poisson: poisson,
                matches: matches,
                scores: scores,
            }
        }
    }

    pub fn simulate<const S: usize>(state: &mut State) {
        unsafe {
            let mut goals = _mm256_set1_pd(0.0);
            let mut goals_mem: F64x4 = Default::default();

            for _ in 0..S {
                for i in (0..state.matches.len()).step_by(4) {
                    let home_id_1 = state.matches[i + 0];
                    let away_id_1 = state.matches[i + 1];
                    let home_poisson_index_1 = (home_id_1 * 2) as usize;
                    let away_poisson_index_1 = (away_id_1 * 2) as usize;
                    let home_1 = state.poisson[home_poisson_index_1 + 0];
                    let away_1 = state.poisson[away_poisson_index_1 + 1];
                    debug_assert!(home_1 != 0.0);
                    debug_assert!(away_1 != 0.0);

                    let home_id_2 = state.matches[i + 2];
                    let away_id_2 = state.matches[i + 3];
                    let home_poisson_index_2 = (home_id_2 * 2) as usize;
                    let away_poisson_index_2 = (away_id_2 * 2) as usize;
                    let home_2 = state.poisson[home_poisson_index_2 + 0];
                    let away_2 = state.poisson[away_poisson_index_2 + 1];
                    debug_assert!(home_2 != 0.0);
                    debug_assert!(away_2 != 0.0);

                    let poisson_vec = _mm256_set_pd(home_1, away_1, home_2, away_2);
                    goals = _mm256_set1_pd(0.0);

                    simulate_match(&poisson_vec, &mut goals, &mut state.rng);

                    _mm256_store_pd(goals_mem.as_mut_ptr(), goals);

                    state.scores[i + 0] = goals_mem[0] as u8;
                    state.scores[i + 1] = goals_mem[1] as u8;
                    state.scores[i + 2] = goals_mem[2] as u8;
                    state.scores[i + 3] = goals_mem[3] as u8;
                }
            }
        }

        state.scores.fill(0);
    }

    unsafe fn simulate_match(
        poisson_vec: &__m256d,
        goals: &mut __m256d,
        rng: &mut Xoshiro256PlusPlusX4,
    ) {
        let mut product_vec = _mm256_set1_pd(0.0);
        rng.next_m256d(&mut product_vec);

        loop {
            let sub = _mm256_sub_pd(product_vec, *poisson_vec);
            let mask = _mm256_movemask_pd(sub);
            if mask == 0x000F {
                break;
            }

            *goals = _mm256_add_pd(*goals, _mm256_ceil_pd(sub));

            let mut next_product_vec = _mm256_set1_pd(0.0);
            rng.next_m256d(&mut next_product_vec);
            product_vec = _mm256_mul_pd(product_vec, next_product_vec);
        }
    }
}
