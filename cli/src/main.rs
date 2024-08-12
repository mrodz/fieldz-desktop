use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use db::Client;
use dialoguer::{theme::ColorfulTheme, Select};
use std::{fmt::Debug, path::Path};

use log::{Level, Metadata, Record};

struct SimpleLogger;

impl log::Log for SimpleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            println!("{} {}", record.level(), record.args());
        }
    }

    fn flush(&self) {}
}

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Subcommand, Debug, Clone)]
enum Commands {
    Playground,
    /// All subcommands related to regions
    Db {
        #[command(subcommand)]
        cmd: DbCommand,
        #[arg(long = "db", required(false))]
        db_path: Option<String>,
    },
}

#[derive(Subcommand, Debug, Clone)]
enum DbCommand {
    Region,
    Field,
}

fn test_schedule() -> Result<()> {
    // backend::algorithm::v2::test()?;
    backend::algorithm::practices::test()?;
    Ok(())
    /*
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

    // println!("Providing times:");
    // for time in RegionalGameQueue::new(&r1) {
    //     println!("- {time}");
    // }
    // println!();

    league.add_region(r1);

    // let out = league.schedule()?;

    // println!("\nValid Games");

    // let mut games = out.valid_games().iter().collect::<Vec<_>>();
    // games.sort_by(|x, y| x.time().cmp(y.time()));

    // for game in games {
    //     println!("{game}");
    // }

    // println!("\nCould not use these times because of conflicts");
    // for unused in out.unable_to_schedule() {
    //     println!("- {unused}");
    // }

    // println!("\nUnscheduled matches");
    // for unused in out.unplayed_matches() {
    //     println!("- {unused}");
    // }

    // println!("\nSummary");
    // for team in league.teams() {
    //     println!("\t{team}");
    // }

    Ok(())
     */
}

async fn db_command(command: DbCommand, db_path: Option<String>) -> Result<()> {
    let db_path = if let Ok(from_env) = std::env::var("DATABASE_URL") {
        from_env
    } else {
        db_path
            .context("`DATABASE_URL` was not set, and no database path was supplied via `--db`")?
    };

    let config = db::Config::new(db_path);

    let client = Client::new(&config).await?;

    let crud_options = &["create", "read", "delete", "update"];

    let selection: isize = Select::with_theme(&ColorfulTheme::default())
        .with_prompt(format!("Enter operation for \"{command:?}\""))
        .default(0)
        .items(crud_options)
        .interact()?
        .try_into()?;

    println!("{selection}, {client:?}");

    todo!();
}

static LOGGER: SimpleLogger = SimpleLogger;

#[tokio::main]
async fn main() -> Result<()> {
    if dotenv::from_path(Path::new(module_path!()).join(".env")).is_err() {
        dotenv::dotenv()?;
    }

    log::set_logger(&LOGGER)
        .map(|()| log::set_max_level(log::LevelFilter::Info))
        .map_err(|e| anyhow!(e))
        .context("could not set up logger")?;

    let args = Args::parse();

    match args.cmd {
        Commands::Playground => test_schedule()?,
        Commands::Db { cmd, db_path } => db_command(cmd, db_path).await?,
    }

    Ok(())
}
