use serde::{Deserialize};
use color_eyre::eyre::Result;
use tokio::{fs::File, io::{BufReader, AsyncReadExt}};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let file = File::open("../input.json").await?;
    let mut file = BufReader::new(file);
    let mut buf = Vec::with_capacity(512);
    file.read_to_end(&mut buf).await?;

    let teams_dto = serde_json::from_slice::<Vec<TeamDto>>(&buf)?;

    sim::simulate::<10_000>(&teams_dto);
    println!("Hello world!");

    Ok(())
}

#[derive(Deserialize)]
pub struct TeamDto {
    pub name: String,
    #[serde(alias = "expectedGoals")]
    pub expected_goals: f64,
}

mod sim {
    pub fn simulate<const S: usize>(input: &[super::TeamDto]) {
        let number_of_matches = (input.len() - 1) * input.len();
    }
}
