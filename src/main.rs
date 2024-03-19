use std::io::Write;
use time::{Date, Duration, OffsetDateTime};

fn year_float(now: OffsetDateTime) -> (f64, Duration) {
    let offset = now.offset();

    let year = now.year();
    let year_start = Date::from_ordinal_date(year, 1).unwrap().with_hms(0, 0, 0).unwrap();
    let year_end = Date::from_ordinal_date(year + 1, 1).unwrap().with_hms(0, 0, 0).unwrap();
    let year_duration = year_end - year_start;

    (f64::from(year) + (now - year_start.assume_offset(offset)) / year_duration, year_duration)
}

fn day_float(now: OffsetDateTime) -> (f64, Duration) {
    let offset = now.offset();

    let day_start = now.date().with_hms(0, 0, 0).unwrap();
    let day_end = now.date().next_day().unwrap().with_hms(0, 0, 0).unwrap();
    let day_duration = day_end - day_start;

    (f64::from(now.ordinal() - 1) + (now - day_start.assume_offset(offset)) / day_duration, day_duration)
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
    let mut prev_year = -1.;
    let mut prev_day = -1.;
    let mut sample_dur = Duration::nanoseconds(-1);
    let mut year_digits = 0;
    let mut day_digits = 0;
    loop {
        let now = OffsetDateTime::now_local().unwrap();
        let (year, year_duration) = year_float(now);
        let (day, day_duration) = day_float(now);

        if year.trunc() - prev_year > 1. || day.trunc() - prev_day > 1. {
            let year_sample_time: Duration;
            let day_sample_time: Duration;
            (year_digits, year_sample_time) = second_ish_precision(year_duration);
            (day_digits, day_sample_time) = second_ish_precision(day_duration);
            println!("{year_digits} digit of year = {year_sample_time} = {} Hz", year_sample_time.as_seconds_f64().recip());
            println!("{day_digits} digit of day = {day_sample_time} = {} Hz", day_sample_time.as_seconds_f64().recip());
            // nyquist theorem: need to sample at highest signal frequency x 2
            sample_dur = year_sample_time.min(day_sample_time) / 2.;
            println!("sampling at 1/{sample_dur} = {} Hz", sample_dur.as_seconds_f64().recip());
        }

        let next = format!("{year:.year_digits$} {day:.day_digits$}");
        let space = last.len();
        print!("\r{next:<space$}");
        std::io::stdout().flush().unwrap();

        last = next;
        prev_year = year.trunc();
        prev_day = day.trunc();

        std::thread::sleep(std::time::Duration::new(0, sample_dur.whole_nanoseconds() as u32));
    }

    //   (format!("{value:.digits$}"), duration)
}
