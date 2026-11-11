//! IANA time zone of a point, offline (`tzf-rs` with its bundled boundary data): no
//! billed Google call, and it works for points set by hand as well as Google places.

use std::sync::LazyLock;
use tzf_rs::DefaultFinder;

/// Built on first use (it parses the bundled boundaries) and shared by every worker.
static FINDER: LazyLock<DefaultFinder> = LazyLock::new(DefaultFinder::new);

/// The time zone the point lies in, e.g. "Europe/Belgrade"; `None` when the data has none
/// or names one chrono-tz does not know (so `openNow` could not use it).
pub fn time_zone_at(latitude: f64, longitude: f64) -> Option<String> {
    if !latitude.is_finite() || !longitude.is_finite() {
        return None;
    }
    let name = FINDER.get_tz_name(longitude, latitude);
    name.parse::<chrono_tz::Tz>().ok().map(|_| name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_zone_of_cities() {
        assert_eq!(
            time_zone_at(52.52, 13.405).as_deref(),
            Some("Europe/Berlin")
        );
        assert_eq!(
            time_zone_at(44.787, 20.457).as_deref(),
            Some("Europe/Belgrade")
        );
        assert_eq!(
            time_zone_at(55.751, 37.618).as_deref(),
            Some("Europe/Moscow")
        );
        assert_eq!(time_zone_at(f64::NAN, 0.0), None);
    }
}
