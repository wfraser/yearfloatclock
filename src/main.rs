use chrono::prelude::*;
use std::io::{stdout, Write};
use std::thread::sleep;
use std::time::Duration;

fn year_fraction<Tz: TimeZone>(time: DateTime<Tz>) -> f64 {
    let y = time.date().year();
    let soy = time.timezone().ymd(y, 1, 1).and_hms(0, 0, 0);
    let eoy = time.timezone().ymd(y+1, 1, 1).and_hms(0, 0, 0);

    let year_secs = (eoy - soy.clone()).to_std().unwrap().as_secs_f64();
    let since_soy = (time - soy).to_std().unwrap().as_secs_f64();

    let year_frac = since_soy / year_secs;

    f64::from(y) + year_frac
}

fn is_leap_year(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

fn day_fraction(year_frac: f64) -> f64 {
    year_frac.fract() * if is_leap_year(year_frac as i32) { 366. } else { 365. }
}

fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>().join(" ").to_lowercase();
    if !args.is_empty() {
        match chrono_english::parse_date_string(&args, Local::now(), chrono_english::Dialect::Us) {
            Ok(dt) => {
                let year = year_fraction(dt);
                let day = day_fraction(year);
                println!("{dt}: {year:.08} {day:.06}");
            }
            Err(e) => {
                eprintln!("invalid date/time \"{args}\": {e}");
            }
        }
        return;
    }

    // Half the time to sleep to make the 6th digit of the day move.
    let sleep_time = Duration::from_secs_f64(24. * 60. * 60. / 1e6 / 2.);

    loop {
        let now = Local::now();
        let year = year_fraction(now);
        let day = day_fraction(year);
        print!("\r{year:.08} {day:.06}");
        stdout().flush().unwrap();
        sleep(sleep_time);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    #[allow(clippy::float_cmp)]
    fn year_ends() {
        let one_sec = chrono::Duration::seconds(1);

        let mut time = Utc.ymd(2020, 12, 31).and_hms(23, 59, 59);
        assert!(2020.9999 < year_fraction(time));
        assert!(2021.0000 > year_fraction(time));

        time = time + one_sec;
        assert_eq!(2021., year_fraction(time));

        time = time + one_sec;
        assert!(2021. < year_fraction(time));
        assert!(2021.0001 > year_fraction(time));
    }

    #[test]
    fn leap_year() {
        assert!(is_leap_year(2000));
        assert!(!is_leap_year(2001));
        assert!(!is_leap_year(2002));
        assert!(!is_leap_year(2003));
        assert!(is_leap_year(2004));
        assert!(!is_leap_year(2005));
        assert!(!is_leap_year(2100));
        assert!(is_leap_year(2400));
    }

    #[test]
    fn day_frac() {
        let one_day = chrono::Duration::hours(24);
        let mut time = Utc.ymd(2020, 1, 1).and_hms(0, 0, 0);
        let d = |t| day_fraction(year_fraction(t));

        assert!(0.0001 > d(time));
        assert!(-0.0001 < d(time));

        time = time + one_day;
        assert!(1.0001 > d(time));
        assert!(0.9999 < d(time));

        time = time + (one_day * 42);
        assert!(43.0001 > d(time));
        assert!(42.9999 < d(time));

        time = Utc.ymd(2020, 1, 1).and_hms(0, 0, 0) - chrono::Duration::seconds(1);
        assert!(365. > d(time));
        assert!(364.9999 < d(time));

        time = Utc.ymd(2020, 1, 1).and_hms(12, 0, 0);
        assert!(0.4999 < d(time));
        assert!(0.5001 > d(time));

        time = Utc.ymd(2020, 5, 19).and_hms(12, 0, 0);
        assert!(0.4999 < d(time).fract());
        assert!(0.5001 > d(time).fract());
    }
}
