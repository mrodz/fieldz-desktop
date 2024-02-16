use std::fmt::Debug;

use anyhow::Result;
use backend::{AvailabilityWindow, League, Region};
use clap::Parser;

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Args {}

fn main() -> Result<()> {
    let mut league = League::new();

    let mut r1 = Region::new();
    r1.add_field(
        "Big Blue Park",
        &[
            backend::window!(9/11/2006 from 12:30 to 14:00)?,
            backend::window!(9/12/2006 from 12:30 to 14:00)?,
            backend::window!(9/12/2006 from 14:30 to 16:00)?,
        ],
    );

    r1.add_team("Rockies");
    r1.add_team("Purple Dragons");

    league.add_region(r1);

    league.schedule();

    dbg!(league);

    Ok(())
}
