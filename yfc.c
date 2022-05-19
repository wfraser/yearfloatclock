#include <stdbool.h>
#include <stdio.h>
#include <sys/time.h>
#include <time.h>
#include <math.h>

double year_fraction(struct timeval *now)
{
    struct tm *now_tm = localtime(&now->tv_sec);
    int y = now_tm->tm_year;
    struct tm jan1 = {
        .tm_sec = 0,
        .tm_min = 0,
        .tm_hour = 0,
        .tm_mday = 1, // 1-based
        .tm_mon = 0, // 0-based
        .tm_year = y,
        0
    };
    time_t soy = mktime(&jan1);
    jan1.tm_year += 1;
    time_t eoy = mktime(&jan1);

    double sec_frac = (double)now->tv_usec / 1e6;

    double year_secs = (double)eoy - (double)soy;
    double since_soy = (double)now->tv_sec - (double)soy + sec_frac;
    double year_frac = since_soy / year_secs;

    return 1900. + (double)y + year_frac;
}

bool is_leap_year(int year)
{
    return (year % 4 == 0 && (year % 100 != 0 || year % 400 == 0));
}

double day_fraction(double year_fraction)
{
    double year_part = year_fraction - floor(year_fraction);
    return year_part * (is_leap_year((int)year_fraction) ? 366. : 365.);
}

int main(int argc, char **argv)
{
    struct timeval now;
    // half the time to move the 6th digit of the day
    // i.e. 5/1e7 of a day, in nanoseconds
    struct timespec sleep = {
        .tv_sec = 0,
        .tv_nsec = (long)(24. * 60. * 60. * 1e9 / 1e6 / 2),
    };

    for (;;) {
        int result = gettimeofday(&now, NULL);
        if (result != 0) {
            perror("gettimeofday");
            return -1;
        }
        double frac = year_fraction(&now);
        printf("\r%.08f %.06f", frac, day_fraction(frac));
        fflush(stdout);
        nanosleep(&sleep, NULL);
    }
}
