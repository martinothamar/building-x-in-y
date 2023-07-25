#![allow(dead_code)]

use std::time::{Duration, Instant};

use color_eyre::eyre::Result;
use monte_carlo_sim::{sim, TeamDto};
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

    const ITERATIONS: usize = 32;
    let mut state = sim::State::new(&teams_dto);
    let mut elapsed = [Duration::ZERO; ITERATIONS];

    for i in 0..ITERATIONS {
        let start = Instant::now();
        sim::simulate::<100_000>(&mut state);
        let stop = Instant::now();

        let duration = stop.duration_since(start);
        elapsed[i] = duration;

        state.reset();
    }

    for i in 0..ITERATIONS {
        println!("Elapsed: {:.3}ms", elapsed[i].as_secs_f64() * 1000.0);
    }

    Ok(())
}
