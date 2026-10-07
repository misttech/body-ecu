// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

/* The types of time.h: time_t and clock_t (from sys/types.h), struct tm, and
 * struct timespec. No time function is provided: a firmware's clock is its
 * own. struct timespec is guarded by __timespec_defined, which Zephyr's POSIX
 * time.h checks before it defines its own. */

#ifndef TIME_H
#define TIME_H

#include <stddef.h>
#include <sys/types.h>

struct tm {
  int tm_sec;   /* Seconds after the minute, 0 to 60. */
  int tm_min;   /* Minutes after the hour, 0 to 59. */
  int tm_hour;  /* Hours since midnight, 0 to 23. */
  int tm_mday;  /* Day of the month, 1 to 31. */
  int tm_mon;   /* Months since January, 0 to 11. */
  int tm_year;  /* Years since 1900. */
  int tm_wday;  /* Days since Sunday, 0 to 6. */
  int tm_yday;  /* Days since January 1, 0 to 365. */
  int tm_isdst; /* Positive in daylight saving time, 0 outside it, negative if unknown. */
};

#ifndef __timespec_defined
#define __timespec_defined
struct timespec {
  time_t tv_sec; /* Whole seconds. */
  long tv_nsec;  /* Nanoseconds, 0 to 999999999. */
};
#endif

#endif /* TIME_H */
