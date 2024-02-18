use std::fmt::Debug;

use anyhow::Result;
use backend::{window, League, Region, RegionalGameQueue};
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
            window!(11/9/2006 from 9:30 to 11:00)?,
            window!(11/9/2006 from 11:30 to 13:00)?,
            window!(12/9/2006 from 13:30 to 15:00)?,
            window!(12/9/2006 from 8:00 to 9:30)?,
            window!(12/9/2006 from 10:00 to 11:30)?,
            window!(13/9/2006 from 12:00 to 13:00)?,
            window!(14/9/2006 from 8:00 to 9:30)?,
            window!(14/9/2006 from 14:00 to 15:30)?,
        ],
    );

    r1.add_field(
        "Field Two",
        &[
            window!(11/9/2006 from 9:30 to 11:00)?,
            window!(11/9/2006 from 11:30 to 13:00)?,
            window!(11/9/2006 from 13:30 to 15:00)?,
            window!(12/9/2006 from 13:30 to 15:00)?,
        ],
    );

    r1.add_team("Rockies");
    r1.add_team("Purple Dragons");
    r1.add_team("Green Machine");
    r1.add_team("Red Rubies");

    println!("Providing times:");
    for time in RegionalGameQueue::new(&r1) {
        println!("- {time}");
    }
    println!();

    league.add_region(r1);

    let out = league.schedule()?;

    println!("\nValid Games");

    let mut games = out.valid_games().iter().collect::<Vec<_>>();
    games.sort_by(|x, y| x.time().cmp(y.time()));
        
    for game in games {
        println!("{game}")
    }

    println!("\nCould not use these times because of conflicts");
    for unused in out.unable_to_schedule() {
        println!("- {unused}")
    }

    println!("\nUnscheduled matches");
    for unused in out.unplayed_matches() {
        println!("- {unused}")
    }

    Ok(())
}
