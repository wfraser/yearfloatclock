use std::io::Write;
use time::format_description::well_known::Iso8601;
use time::{Date, Duration, OffsetDateTime, PrimitiveDateTime, UtcOffset};

// We're ignoring leap seconds.
const DAY_DURATION: Duration = Duration::hours(24);

struct Clock {
    basis_year: f64,
    basis_day: f64,
    basis_tz: Option<time::UtcOffset>,

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
        // The day is always the same length, so these values stay fixed.
        // Ideally these could be consts like DAY_DURATION, but floating-point division isn't
        // allowed in const rust.
        let (day_digits, day_sample_duration) = second_ish_precision(DAY_DURATION);
        Self {
            basis_year: 0.,
            basis_day: 0.,
            basis_tz: None,
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

    /// Recalculate cached internal variables for the given date+time. Needs to be called if the
    /// date changed since the last time any other methods were called.
    fn recalculate(&mut self, now: OffsetDateTime) {
        let offset = self.basis_tz.unwrap_or(now.offset());

        let year = now.year();
        let year_start = Date::from_ordinal_date(year, 1)
            .unwrap()
            .with_hms(0, 0, 0)
            .unwrap();
        let year_end = Date::from_ordinal_date(year + 1, 1)
            .unwrap()
            .with_hms(0, 0, 0)
            .unwrap();

        self.year = f64::from(year);
        self.year_duration = year_end - year_start;
        self.year_start = year_start.assume_offset(offset);

        self.day = f64::from(now.ordinal() - 1);
        self.day_start = now.date().with_hms(0, 0, 0).unwrap().assume_offset(offset);

        let (year_digits, year_sample_duration) = second_ish_precision(self.year_duration);
        self.year_digits = year_digits;

        // nyquist theorem: the required sample rate is 2x the highest frequency signal
        self.sample_delay = year_sample_duration.min(self.day_sample_duration) / 2;

        /*
        println!("{year_digits} digit of year = {year_sample_duration} = {} Hz", year_sample_duration.as_seconds_f64().recip());
        println!("{} digit of day = {} = {} Hz", self.day_digits, self.day_sample_duration, self.day_sample_duration.as_seconds_f64().recip());
        println!("sampling at 1/{} = {} Hz", self.sample_delay, self.sample_delay.as_seconds_f64().recip());
         */
    }

    /// The year and the fraction of the way through the year.
    pub fn year_float(&mut self, now: OffsetDateTime) -> f64 {
        let year = f64::from(now.year());
        if year != self.year {
            self.recalculate(now);
        }
        year + (now - self.year_start) / self.year_duration - self.basis_year
    }

    /// The day of the year (0-based) and the fraction of the way through the day.
    pub fn day_float(&mut self, now: OffsetDateTime) -> f64 {
        let day = f64::from(now.ordinal() - 1);
        if day != self.day || f64::from(now.year()) != self.year {
            self.recalculate(now);
        }
        let f = day + (now - self.day_start) / DAY_DURATION;
        if f >= self.basis_day {
            f - self.basis_day
        } else {
            f - self.basis_day + (self.year_duration / DAY_DURATION)
        }
    }

    /// Format the year and day fractions into a string with the right number of digits.
    pub fn format(&mut self, now: OffsetDateTime) -> String {
        let year = self.year_float(now);
        let day = self.day_float(now);
        let year_digits = self.year_digits;
        let day_digits = self.day_digits;
        format!("{year:.year_digits$} {day:.day_digits$}")
    }

    /// How long to delay before taking another time sample?
    pub fn sample_delay(&self) -> std::time::Duration {
        std::time::Duration::new(0, self.sample_delay.subsec_nanoseconds() as u32)
    }

    pub fn set_basis(&mut self, time: OffsetDateTime) {
        self.basis_year = self.year_float(time);
        self.basis_day = self.day_float(time);
        self.basis_tz = Some(time.offset());
    }
}

/// For a decimal number of a given duration, how many digits need to be shown for it to update
/// faster than once per second, and how often does that last digit update exactly?
fn second_ish_precision(mut duration: Duration) -> (usize, Duration) {
    let mut digits = 0;
    while duration > Duration::seconds(1) {
        duration /= 10;
        digits += 1;
    }
    (digits, duration)
}

#[derive(Debug, Default)]
struct Args {
    basis: Option<OffsetDateTime>,
    at: Option<OffsetDateTime>,
}

impl Args {
    pub fn parse() -> Self {
        let mut ret = Self::default();
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--version" | "-V" => {
                    eprintln!(
                        "{} v{} {}",
                        env!("CARGO_PKG_NAME"),
                        env!("CARGO_PKG_VERSION"),
                        env!("CARGO_PKG_AUTHORS")
                    );
                    std::process::exit(1);
                }
                "--at" | "--basis" => {
                    let dest = match arg.as_str() {
                        "--at" => &mut ret.at,
                        "--basis" => &mut ret.basis,
                        _ => unreachable!(),
                    };
                    let time = {
                        let bail = || -> ! {
                            eprintln!("{arg} must be followed by a date/time in YYYY-MM-DD[THH:MM:SS[+ZZZZ]] format");
                            std::process::exit(2);
                        };
                        let input = args.next().unwrap_or_else(|| bail());
                        if let Ok(dt) = OffsetDateTime::parse(&input, &Iso8601::PARSING) {
                            dt
                        } else if let Ok(dt) = PrimitiveDateTime::parse(&input, &Iso8601::PARSING) {
                            dt.assume_offset(UtcOffset::current_local_offset().unwrap())
                        } else if let Ok(d) = Date::parse(&input, &Iso8601::PARSING) {
                            d.with_hms(0, 0, 0)
                                .unwrap()
                                .assume_offset(UtcOffset::current_local_offset().unwrap())
                        } else {
                            bail()
                        }
                    };
                    *dest = Some(time);
                }
                other => {
                    if other != "-h" && other != "--help" {
                        eprintln!("unrecognized argument {other:?}");
                    }
                    eprintln!("usage: {} [--basis datetime] [--at datetime]", env!("CARGO_PKG_NAME"));
                    std::process::exit(1);
                }
            }
        }
        ret
    }
}

fn main() {
    let mut last = String::new();
    let mut clock = Clock::new();
    let args = Args::parse();
    if let Some(basis) = args.basis {
        clock.set_basis(basis);
    }
    if let Some(t) = args.at {
        println!("{}", clock.format(t));
        return;
    }
    loop {
        let now = OffsetDateTime::now_local().unwrap();
        let next = clock.format(now);
        if next != last {
            let space = last.len();
            print!("\r{next:<space$}");
            std::io::stdout().flush().unwrap();
            last = next;
        }
        std::thread::sleep(clock.sample_delay());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Month;

    #[test]
    fn year_ends() {
        let mut clock = Clock::new();

        let one_sec = Duration::seconds(1);

        let mut time = Date::from_calendar_date(2020, Month::December, 31)
            .unwrap()
            .with_hms(23, 59, 59)
            .unwrap()
            .assume_utc();
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
        let mut time = Date::from_calendar_date(2021, Month::January, 1)
            .unwrap()
            .with_hms(0, 0, 0)
            .unwrap()
            .assume_utc();
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
        let mut time = Date::from_calendar_date(2000, Month::December, 31)
            .unwrap()
            .with_hms(23, 59, 59)
            .unwrap()
            .assume_utc();
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
