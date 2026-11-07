//! The local wall clock, for dates and times as Ami Pro shows them and for
//! its time base (`docs/research/FIELDS_NOTES.md`, "The time base").

use amipro_document::WallClock;

/// The local wall clock now, from the system's time zone.
pub fn local_now() -> WallClock {
    let now = jiff::Zoned::now();
    WallClock {
        year: i32::from(now.year()),
        month: now.month() as u8,
        day: now.day() as u8,
        hour: now.hour() as u8,
        minute: now.minute() as u8,
        second: now.second() as u8,
    }
}

/// Now in Ami Pro's time base: the seconds it would store for this wall
/// clock (`[desc]`'s save and creation times).
pub fn ami_pro_now() -> u32 {
    local_now().to_ami_pro()
}

/// Ami Pro's minutes of editing for a session `elapsed` long: rounded to the
/// nearest minute (`11c8:342b`).
pub fn editing_minutes(elapsed: std::time::Duration) -> u32 {
    u32::try_from((elapsed.as_secs() + 30) / 60).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_clock_reads_a_plausible_local_time() {
        let now = local_now();
        assert!((2020..2200).contains(&now.year));
        assert!((1..=12).contains(&now.month) && (1..=31).contains(&now.day));
        assert!(now.hour < 24 && now.minute < 60 && now.second < 61);
        // Ami Pro's time base is within a day of the system's UTC seconds.
        let utc = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        assert!((i64::from(ami_pro_now()) - utc).abs() < 86_400);
    }

    #[test]
    fn editing_time_rounds_to_the_nearest_minute() {
        use std::time::Duration;
        assert_eq!(editing_minutes(Duration::from_secs(29)), 0);
        assert_eq!(editing_minutes(Duration::from_secs(30)), 1);
        assert_eq!(editing_minutes(Duration::from_secs(149)), 2);
        assert_eq!(editing_minutes(Duration::from_secs(150)), 3);
    }
}
