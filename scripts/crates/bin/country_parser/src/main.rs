use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{from_reader, to_writer_pretty, Value};

#[derive(Debug, Deserialize)]
pub struct GeoJson {
    #[serde(rename = "type")]
    pub r#type: String,
    pub name: String,
    pub crs: Crs,
    pub features: Vec<GeoFeature>,
}

#[derive(Debug, Deserialize)]
pub struct Crs {
    #[serde(rename = "type")]
    pub r#type: String,
    pub properties: CrsProperties,
}

#[derive(Debug, Deserialize)]
pub struct CrsProperties {  
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct GeoFeature {
    #[serde(rename = "type")]
    pub r#type: String,
    pub properties: GeoFeatureProperties,
    pub geometry: Geometry,
}

#[derive(Debug, Deserialize)]
pub struct GeoFeatureProperties {
    pub name: String,
    #[serde(rename = "ISO3166-1-Alpha-2")]
    pub iso_alpha2: String,
    #[serde(rename = "ISO3166-1-Alpha-3")]
    pub iso_alpha3: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Geometry {
    #[serde(rename = "type")]
    pub r#type: String,
    pub coordinates: Value, // Univeral type for coordinates.
}

#[derive(Debug, Deserialize)]
struct CountryInfo {
    pub cca2: String,
    pub cca3: String,
    pub name: Name,
    pub capital: Option<Vec<String>>,
    pub flag: String,
    pub maps: HashMap<String, String>,
    pub languages: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct Name {
    pub common: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MergedCountry {
    pub name: String,
    pub capital: Option<String>,
    pub flag: String,
    pub geo_json: Geometry,
}

fn main() -> Result<()> {
    // 1. Reading countries.json
    let countries_file = File::open("./data/countries.json")?;
    let countries: Vec<CountryInfo> = from_reader(BufReader::new(countries_file))?;

    // 2. Reading countries.geojson
    let geo_file = File::open("./data/countries.geojson")?;
    let geo: GeoJson = from_reader(BufReader::new(geo_file))?;

    // 3. Indexing geo features by ISO codes
    let mut geo_map: HashMap<String, &GeoFeature> = HashMap::new();
    for feature in &geo.features {
        geo_map.insert(feature.properties.iso_alpha2.to_uppercase(), feature);
        geo_map.insert(feature.properties.iso_alpha3.to_uppercase(), feature);
    }

    // 4. Merging data
    let mut merged: Vec<MergedCountry> = Vec::with_capacity(countries.len());
    for country in countries {
        let key2 = country.cca2.to_uppercase();
        let key3 = country.cca3.to_uppercase();

        if let Some(geo) = geo_map.get(&key2).or_else(|| geo_map.get(&key3)) {
            merged.push(MergedCountry {
                name: country.name.common,
                capital: country.capital.and_then(|v| v.into_iter().next()),
                flag: country.flag,
                geo_json: geo.geometry.clone(),
            });
        }
    }

    // 5. Saving merged data to JSON
    let output = File::create("./data/merged_countries.json")?;
    to_writer_pretty(BufWriter::new(output), &merged)?;
    println!("Merged {} countries", merged.len());

    Ok(())
}
