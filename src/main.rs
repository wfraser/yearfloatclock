use std::io::Write;
use time::{Date, Duration, OffsetDateTime};

// We're ignoring leap seconds.
const DAY_DURATION: Duration = Duration::hours(24);

struct Clock {
    year: f64,
    year_start: OffsetDateTime,
    year_duration: Duration,

    day: f64,
    day_start: OffsetDateTime,

    year_digits: usize,
    day_digits: usize,
    day_sample_duration: Duration,

    sample_delay: Duration,
}

impl Clock {
    pub fn new() -> Self {
        let (day_digits, day_sample_duration) = second_ish_precision(DAY_DURATION);
        Self {
            year: -1.,
            year_start: OffsetDateTime::from_unix_timestamp(0).unwrap(),
            year_duration: Duration::seconds(-1),
            day: -1.,
            day_start: OffsetDateTime::from_unix_timestamp(0).unwrap(),
            year_digits: 0,
            day_digits,
            day_sample_duration,
            sample_delay: Duration::seconds(-1),
        }
    }

    fn recalculate(&mut self, now: OffsetDateTime) {
        let offset = now.offset();

        let year = now.year();
        let year_start = Date::from_ordinal_date(year, 1).unwrap().with_hms(0, 0, 0).unwrap();
        let year_end = Date::from_ordinal_date(year + 1, 1).unwrap().with_hms(0, 0, 0).unwrap();

        self.year = f64::from(year);
        self.year_duration = year_end - year_start;
        self.year_start = year_start.assume_offset(offset);

        self.day = f64::from(now.ordinal() - 1);
        self.day_start = now.date().with_hms(0, 0, 0).unwrap().assume_offset(offset);

        let (year_digits, year_sample_duration) = second_ish_precision(self.year_duration);
        self.year_digits = year_digits;

        self.sample_delay = year_sample_duration.min(self.day_sample_duration) / 2.;

        println!("{year_digits} digit of year = {year_sample_duration} = {} Hz", year_sample_duration.as_seconds_f64().recip());
        println!("{} digit of day = {} = {} Hz", self.day_digits, self.day_sample_duration, self.day_sample_duration.as_seconds_f64().recip());
        println!("sampling at 1/{} = {} Hz", self.sample_delay, self.sample_delay.as_seconds_f64().recip());
    }

    pub fn year_float(&mut self, now: OffsetDateTime) -> f64 {
        let year = f64::from(now.year());
        if year != self.year {
            self.recalculate(now);
        }
        year + (now - self.year_start) / self.year_duration
    }

    pub fn day_float(&mut self, now: OffsetDateTime) -> f64 {
        let day = f64::from(now.ordinal() - 1);
        if day != self.day {
            self.recalculate(now);
        }
        day + (now - self.day_start) / DAY_DURATION
    }

    pub fn format(&mut self, now: OffsetDateTime) -> String {
        let year = self.year_float(now);
        let day = self.day_float(now);
        let year_digits = self.year_digits;
        let day_digits = self.day_digits;
        format!("{year:.year_digits$} {day:.day_digits$}")
    }

    pub fn sample_delay(&self) -> std::time::Duration {
        std::time::Duration::new(0, self.sample_delay.subsec_nanoseconds() as u32)
    }
}

fn second_ish_precision(mut duration: Duration) -> (usize, Duration) {
    let mut digits = 0;
    while duration > Duration::seconds(1) {
        duration /= 10;
        digits += 1;
    }
    (digits, duration)
}

fn main() {
    let mut last = String::new();
    let mut clock = Clock::new();
    loop {
        let now = OffsetDateTime::now_local().unwrap();
        let next = clock.format(now);
        let space = last.len();
        print!("\r{next:<space$}");
        std::io::stdout().flush().unwrap();

        last = next;
        std::thread::sleep(clock.sample_delay());
    }
}

#[cfg(test)]
mod tests {
    use time::Month;
    use super::*;

    #[test]
    fn year_ends() {
        let mut clock = Clock::new();

        let one_sec = Duration::seconds(1);

        let mut time = Date::from_calendar_date(2020, Month::December, 31).unwrap().with_hms(23, 59, 59).unwrap().assume_utc();
        assert!(2020.9999 < clock.year_float(time));
        assert!(2021.0000 > clock.year_float(time));

        time += one_sec;
        assert_eq!(2021., clock.year_float(time));

        time += one_sec;
        assert!(2021. < clock.year_float(time));
        assert!(2021.0001 > clock.year_float(time));
    }

    #[test]
    fn day_frac() {
        let mut clock = Clock::new();
        let mut time = Date::from_calendar_date(2021, Month::January, 1).unwrap().with_hms(0, 0, 0).unwrap().assume_utc();
        assert!(0.0001 > clock.day_float(time));
        assert!(-0.0001 < clock.day_float(time));

        time += DAY_DURATION;
        assert!(0.9999 < clock.day_float(time));
        assert!(1.0001 > clock.day_float(time));
    }

    #[test]
    fn leap_year() {
        let mut clock = Clock::new();

        // 2000 was a leap year
        let mut time = Date::from_calendar_date(2000, Month::December, 31).unwrap().with_hms(23, 59, 59).unwrap().assume_utc();
        assert!(2001. > clock.year_float(time));
        let leap_delay = clock.sample_delay();

        // happy new year
        time += Duration::seconds(1);
        assert_eq!(2001., clock.year_float(time));
        let non_leap_delay = clock.sample_delay();

        // leap year is slightly longer, and should have a longer delay
        assert!(leap_delay > non_leap_delay);
    }
}
