use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{from_reader, to_writer_pretty, Value};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter};

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
    pub coordinates: Value, // Universal type for coordinates.
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
    /// Official languages, sorted by code. Missing in files written before languages were kept.
    #[serde(default)]
    pub languages: Vec<MergedLanguage>,
}

/// A language of a country: ISO 639-3 code (`languages.symbol`) and English name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergedLanguage {
    pub code: String,
    pub name: String,
}

/// The languages of `countries.json` (`{"rus": "Russian"}`) as a list sorted by code.
pub fn languages_of(languages: &HashMap<String, String>) -> Vec<MergedLanguage> {
    let mut list: Vec<MergedLanguage> = languages
        .iter()
        .map(|(code, name)| MergedLanguage {
            code: code.clone(),
            name: name.clone(),
        })
        .collect();
    list.sort_by(|a, b| a.code.cmp(&b.code));
    list
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
                languages: languages_of(&country.languages),
            });
        }
    }

    // 5. Saving merged data to JSON
    let output = File::create("./data/merged_countries.json")?;
    to_writer_pretty(BufWriter::new(output), &merged)?;
    println!("Merged {} countries", merged.len());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn languages_are_listed_by_code() {
        let source = HashMap::from([
            ("srp".to_string(), "Serbian".to_string()),
            ("hun".to_string(), "Hungarian".to_string()),
        ]);
        assert_eq!(
            languages_of(&source),
            vec![
                MergedLanguage { code: "hun".into(), name: "Hungarian".into() },
                MergedLanguage { code: "srp".into(), name: "Serbian".into() },
            ]
        );
    }

    #[test]
    fn a_merged_file_without_languages_still_reads() {
        let old = r#"{"name":"Togo","capital":"Lomé","flag":"🇹🇬","geo_json":{"type":"Polygon","coordinates":[]}}"#;
        let country: MergedCountry = serde_json::from_str(old).unwrap();
        assert!(country.languages.is_empty());
    }
}
