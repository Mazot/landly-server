use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use dotenv::dotenv;
use std::env;
use serde_json::json;
use landly_server::data::models::{Country, CreateCountry};
use landly_server::utils::db::establish_connection;
use country_parser::MergedCountry;

fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();

    let db_pool = establish_connection();
    let args: Vec<String> = env::args().collect(); // Собираем аргументы в вектор строк

    let file_path = args[1].clone();
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);

    let countries: Vec<MergedCountry> = serde_json::from_reader(reader)?;

    println!("Loading of {} countries to DB", countries.len());

    let connection = &mut db_pool.get()?;

    for country_data in countries {
        let country = CreateCountry {
            name: country_data.name,
            geo_json: Some(json!(country_data.geo_json)),
            flag: Some(country_data.flag),
            capital_city: country_data.capital,
            description: None,
        };

        match Country::create(connection, &country) {
            Ok(_) => println!("Country added: {}", country.name),
            Err(e) => eprintln!("Error {}: {}", country.name, e),
        }
    }

    println!("Loading completed successfully!");

    Ok(())
}
