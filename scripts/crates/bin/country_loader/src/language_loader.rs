use country_parser::MergedCountry;
use diesel::prelude::*;
use diesel::sql_types::{Text, Uuid as SqlUuid};
use dotenv::dotenv;
use landly_server::utils::db::establish_connection;
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::process::exit;
use uuid::Uuid;

#[derive(QueryableByName)]
struct Id {
    #[diesel(sql_type = SqlUuid)]
    id: Uuid,
}

/// Loads the languages of a merged_countries.json (produced by country_parser) into
/// `languages` (ISO 639-3 code in `symbol`) and links them to their countries in
/// `countries_to_languages`. Idempotent: a language is matched by its code, a link that
/// exists is kept, so it can run on every deploy. Countries are matched by name and must
/// be loaded first (country_loader).
fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();

    let args: Vec<String> = env::args().collect();
    let Some(file_path) = args.get(1) else {
        eprintln!("Usage: language_loader <path/to/merged_countries.json>");
        exit(1);
    };
    let countries: Vec<MergedCountry> = serde_json::from_reader(BufReader::new(File::open(file_path)?))?;

    let db_pool = establish_connection();
    let connection = &mut db_pool.get()?;

    let (mut languages, mut links, mut missing_countries) = (0usize, 0usize, 0usize);
    for country in &countries {
        let country_id: Option<Id> = diesel::sql_query("SELECT id FROM countries WHERE name = $1 LIMIT 1")
            .bind::<Text, _>(&country.name)
            .get_result(connection)
            .optional()?;
        let Some(country_id) = country_id else {
            missing_countries += 1;
            continue;
        };
        for language in &country.languages {
            let existing: Option<Id> = diesel::sql_query("SELECT id FROM languages WHERE symbol = $1 LIMIT 1")
                .bind::<Text, _>(&language.code)
                .get_result(connection)
                .optional()?;
            let language_id = match existing {
                Some(row) => row.id,
                None => {
                    languages += 1;
                    diesel::sql_query("INSERT INTO languages (name, symbol) VALUES ($1, $2) RETURNING id")
                        .bind::<Text, _>(&language.name)
                        .bind::<Text, _>(&language.code)
                        .get_result::<Id>(connection)?
                        .id
                }
            };
            links += diesel::sql_query(
                "INSERT INTO countries_to_languages (country_id, language_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            )
            .bind::<SqlUuid, _>(country_id.id)
            .bind::<SqlUuid, _>(language_id)
            .execute(connection)?;
        }
    }

    println!(
        "Languages loaded: {languages} new languages, {links} new country links, {missing_countries} countries not in the database"
    );
    if missing_countries == countries.len() && !countries.is_empty() {
        eprintln!("No country of the file is in the database: run country_loader first");
        exit(2);
    }
    Ok(())
}
