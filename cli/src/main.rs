use anyhow::{Context, Result};
use backend::{window, League, Region, RegionalGameQueue};
use clap::{Parser, Subcommand};
use std::{fmt::Debug, path::Path};

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    cmd: Commands,
    #[arg(long = "db", required(false))]
    db_path: Option<String>,
}

#[derive(Subcommand, Debug, Clone)]
enum Commands {
    Playground,
    /// All subcommands related to regions
    Region {
        #[command(subcommand)]
        cmd: RegionCommands,
    },
}

#[derive(Subcommand, Debug, Clone)]
enum RegionCommands {
    /// Create a new region
    Create {
        /// The name of the region
        #[arg(long, required(true))]
        name: String,
    },
    /// List all regions
    List,
    /// Delete all regions
    Purge,
}

fn test_schedule() -> Result<()> {
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
        println!("{game}");
    }

    println!("\nCould not use these times because of conflicts");
    for unused in out.unable_to_schedule() {
        println!("- {unused}");
    }

    println!("\nUnscheduled matches");
    for unused in out.unplayed_matches() {
        println!("- {unused}");
    }

    println!("\nSummary");
    for team in league.teams() {
        println!("\t{team}");
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::from_path(Path::new(module_path!()).join(".env"))?;

    let args = Args::parse();

    let db_path = if let Ok(from_env) = std::env::var("DATABASE_URL") {
        from_env
    } else {
        args.db_path
            .context("`DATABASE_URL` was not set, and no database path was supplied via `--db`")?
    };

    let config = db::Config::new(db_path);

    let client = db::connect(&config).await?;

    match args.cmd {
        Commands::Playground => {
            test_schedule()?;
        }
        Commands::Region { cmd } => match cmd {
            RegionCommands::List => {
                let result = client.get_regions().await?;
                println!("{result:#?}");
            }
            RegionCommands::Create { name } => {
                let result = client.create_region(name).await?;
                println!("{result:#?}");
            }
            RegionCommands::Purge => {
                let result = client.delete_regions().await?;
                println!("{result:#?}");
            }
        },
    }

    Ok(())
}
