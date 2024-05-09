use crate::DistanceUnit;

pub fn format_meters(meters: f64, unit: DistanceUnit) -> String {
    match unit {
        DistanceUnit::Kilometers => {
            if meters < 1.5 {
                "1 meter".to_string()
            } else if meters < 10.0 {
                format!("{:.0} meters", meters.round())
            } else if meters < 500.0 {
                // the nearest 10 meters
                format!("{:.0} meters", (meters / 10.0).round() * 10.0)
            } else if meters < 950.0 {
                // nearest 100 meters
                format!("{:.0} meters", (meters / 100.0).round() * 100.0)
            } else if meters < 1050.0 {
                "1 kilometer".to_string()
            } else {
                let kilometers = meters / 1000.0;
                if meters < 9950.0 {
                    format!("{kilometers:.1} kilometers")
                } else {
                    format!("{kilometers:.0} kilometers")
                }
            }
        }
        DistanceUnit::Miles => {
            const METERS_PER_MILE: f64 = 1609.34;
            const METERS_PER_FOOT: f64 = 0.3048;
            if meters < METERS_PER_MILE * 0.125 {
                // round to the nearest 10 feet
                let mut feet = (meters / METERS_PER_FOOT / 10.0).round() * 10.0;
                if feet > 200.0 {
                    // round to the nearest 100 feet
                    feet = (feet / 100.0).round() * 100.0;
                }
                format!("{:.0} feet", (10.0f64).max(feet))
            } else if meters < METERS_PER_MILE * 0.375 {
                "a quarter mile".to_string()
            } else if meters < METERS_PER_MILE * 0.625 {
                "a half mile".to_string()
            } else {
                let miles = meters / METERS_PER_MILE;
                if (0.95..1.1).contains(&miles) {
                    "1 mile".to_string()
                } else if miles < 9.95 {
                    format!("{miles:.1} miles")
                } else {
                    format!("{:.0} miles", miles.round())
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meter_formatting() {
        assert_eq!(format_meters(1.0, DistanceUnit::Kilometers), "1 meter");
        assert_eq!(format_meters(2.6, DistanceUnit::Kilometers), "3 meters");
        assert_eq!(format_meters(99.0, DistanceUnit::Kilometers), "100 meters");
        assert_eq!(format_meters(599.0, DistanceUnit::Kilometers), "600 meters");
        assert_eq!(format_meters(600.0, DistanceUnit::Kilometers), "600 meters");
        assert_eq!(format_meters(900.0, DistanceUnit::Kilometers), "900 meters");
        assert_eq!(
            format_meters(960.0, DistanceUnit::Kilometers),
            "1 kilometer"
        );
        assert_eq!(
            format_meters(1049.0, DistanceUnit::Kilometers),
            "1 kilometer"
        );
        assert_eq!(
            format_meters(1100.0, DistanceUnit::Kilometers),
            "1.1 kilometers"
        );
        assert_eq!(
            format_meters(9940.0, DistanceUnit::Kilometers),
            "9.9 kilometers"
        );
        assert_eq!(
            format_meters(9999.0, DistanceUnit::Kilometers),
            "10 kilometers"
        );
        assert_eq!(
            format_meters(10000.0, DistanceUnit::Kilometers),
            "10 kilometers"
        );
        assert_eq!(
            format_meters(100000.0, DistanceUnit::Kilometers),
            "100 kilometers"
        );
    }

    #[test]
    fn format_miles_from_meters() {
        assert_eq!(format_meters(1.0, DistanceUnit::Miles), "10 feet");
        assert_eq!(format_meters(10.0, DistanceUnit::Miles), "30 feet");
        assert_eq!(format_meters(50.0, DistanceUnit::Miles), "160 feet");
        assert_eq!(format_meters(100.0, DistanceUnit::Miles), "300 feet");
        assert_eq!(format_meters(500.0, DistanceUnit::Miles), "a quarter mile");
        assert_eq!(format_meters(1000.0, DistanceUnit::Miles), "a half mile");
        assert_eq!(format_meters(1100.0, DistanceUnit::Miles), "0.7 miles");
        assert_eq!(format_meters(1300.0, DistanceUnit::Miles), "0.8 miles");
        assert_eq!(format_meters(1700.0, DistanceUnit::Miles), "1 mile");
        assert_eq!(format_meters(1800.0, DistanceUnit::Miles), "1.1 miles");
        assert_eq!(format_meters(2000.0, DistanceUnit::Miles), "1.2 miles");
        assert_eq!(format_meters(16000.0, DistanceUnit::Miles), "9.9 miles");
        assert_eq!(format_meters(16500.0, DistanceUnit::Miles), "10 miles");
        assert_eq!(format_meters(20000.0, DistanceUnit::Miles), "12 miles");
    }
}
