use country_parser::MergedCountry;
use dotenv::dotenv;
use landly_server::data::models::{Country, CreateCountry};
use landly_server::utils::db::establish_connection;
use serde_json::json;
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::process::exit;

/// Loads countries from a merged_countries.json file (produced by
/// country_parser) into the `countries` table. Skips nothing: every entry is
/// inserted; per-row failures are reported and counted but do not abort the
/// run, so re-running on a partially seeded DB is safe.
fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();

    let args: Vec<String> = env::args().collect();
    let Some(file_path) = args.get(1) else {
        eprintln!("Usage: country_loader <path/to/merged_countries.json>");
        exit(1);
    };

    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    let countries: Vec<MergedCountry> = serde_json::from_reader(reader)?;

    println!("Loading of {} countries to DB", countries.len());

    let db_pool = establish_connection();
    let connection = &mut db_pool.get()?;

    let mut loaded = 0usize;
    let mut failed = 0usize;

    for country_data in countries {
        let country = CreateCountry {
            name: country_data.name,
            geo_json: Some(json!(country_data.geo_json)),
            flag: Some(country_data.flag),
            capital_city: country_data.capital,
            description: None,
            // Not present in the merged dataset; fill in later via API/SQL.
            currency: None,
            phone_code: None,
            top_cities: None,
        };

        match Country::create(connection, &country) {
            Ok(_) => {
                loaded += 1;
                println!("Country added: {}", country.name);
            }
            Err(e) => {
                failed += 1;
                eprintln!("Error {}: {}", country.name, e);
            }
        }
    }

    println!("Loading completed: {} added, {} failed", loaded, failed);

    // Total failure (e.g. wrong schema / dead DB) should fail the container
    // start; partial failures are only reported so start.sh can proceed.
    if loaded == 0 && failed > 0 {
        exit(2);
    }

    Ok(())
}
