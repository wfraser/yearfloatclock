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

    y as f64 + year_frac
}

fn is_leap_year(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

fn main() {
    // How long to sleep to make the 8th digit move?
    let per_day = 24. * 60. * 60. / 1e8;
    let sleep_leap = Duration::from_secs_f64(366. * per_day);
    let sleep_non_leap = Duration::from_secs_f64(365. * per_day);

    loop {
        let now = Local::now();
        print!("\r{:.8}", year_fraction(now));
        stdout().flush().unwrap();
        sleep(if is_leap_year(now.year()) { sleep_leap } else { sleep_non_leap });
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
}
