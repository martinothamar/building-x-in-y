const std = @import("std");
const posix = std.posix;
const time = std.time;
const assert = std.debug.assert;

// Based on: https://github.com/frmdstryr/zig-datetime/blob/70aebf28fb3e137cd84123a9349d157a74708721/src/datetime.zig

// Number of days before Jan 1st of year
fn daysBeforeYear(year: u32) u32 {
    const y: u32 = year - 1;
    return y * 365 + @divFloor(y, 4) - @divFloor(y, 100) + @divFloor(y, 400);
}

// Days before 1 Jan 1970
const EPOCH = daysBeforeYear(1970) + 1;

const DAYS_IN_MONTH = [12]u8{ 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31 };
const DAYS_BEFORE_MONTH = [12]u16{ 0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334 };

const MAX_ORDINAL: u32 = 3652059;

pub const DateTime = struct {
    day_name: []const u8,
    day: u8,
    month_name: []const u8,
    month: u4,
    year: u16,
    hour: u8,
    minute: u8,
    second: u8,

    pub fn init(ts: *const posix.timespec) DateTime {
        const ts_ns: i128 = (@as(i128, ts.tv_sec) * time.ns_per_s) + ts.tv_nsec;
        const timestamp: i64 = @as(i64, @intCast(@divFloor(ts_ns, time.ns_per_ms)));

        const days = @divFloor(timestamp, time.ms_per_day) + @as(i64, EPOCH);
        assert(days >= 0 and days <= MAX_ORDINAL);

        const remainder = @mod(timestamp, time.ms_per_day);
        var t: u64 = @abs(remainder);
        // t is now only the time part of the day
        const h: u32 = @intCast(@divFloor(t, time.ms_per_hour));
        t -= h * time.ms_per_hour;
        const m: u32 = @intCast(@divFloor(t, time.ms_per_min));
        t -= m * time.ms_per_min;
        const s: u32 = @intCast(@divFloor(t, time.ms_per_s));
        // t -= s * time.ms_per_s;
        // const ns: u32 = @intCast(t * time.ns_per_ms);

        const ordinal: u32 = @intCast(days);
        const d = date(ordinal);

        const dow: u3 = @intCast(ordinal % 7);
        const weekday: Weekday = @enumFromInt(if (dow == 0) 7 else dow);
        const weekday_name = @tagName(weekday);

        const month_name = @tagName(@as(Month, @enumFromInt(d.month)));

        return DateTime{
            .day_name = weekday_name,
            .day = d.day,
            .month_name = month_name,
            .month = d.month,
            .year = d.year,
            .hour = @intCast(h),
            .minute = @intCast(m),
            .second = @intCast(s),
        };
    }

    fn date(ordinal: u32) Date {
        // n is a 1-based index, starting at 1-Jan-1.  The pattern of leap years
        // repeats exactly every 400 years.  The basic strategy is to find the
        // closest 400-year boundary at or before n, then work with the offset
        // from that boundary to n.  Life is much clearer if we subtract 1 from
        // n first -- then the values of n at 400-year boundaries are exactly
        // those divisible by DI400Y:
        //
        //     D  M   Y            n              n-1
        //     -- --- ----        ----------     ----------------
        //     31 Dec -400        -DI400Y        -DI400Y -1
        //      1 Jan -399        -DI400Y +1     -DI400Y       400-year boundary
        //     ...
        //     30 Dec  000        -1             -2
        //     31 Dec  000         0             -1
        //      1 Jan  001         1              0            400-year boundary
        //      2 Jan  001         2              1
        //      3 Jan  001         3              2
        //     ...
        //     31 Dec  400         DI400Y        DI400Y -1
        //      1 Jan  401         DI400Y +1     DI400Y        400-year boundary
        assert(ordinal >= 1 and ordinal <= MAX_ORDINAL);

        var n = ordinal - 1;
        const DI400Y = comptime daysBeforeYear(401); // Num of days in 400 years
        const DI100Y = comptime daysBeforeYear(101); // Num of days in 100 years
        const DI4Y = comptime daysBeforeYear(5); // Num of days in 4   years
        const n400 = @divFloor(n, DI400Y);
        n = @mod(n, DI400Y);
        var year = n400 * 400 + 1; //  ..., -399, 1, 401, ...

        // Now n is the (non-negative) offset, in days, from January 1 of year, to
        // the desired date.  Now compute how many 100-year cycles precede n.
        // Note that it's possible for n100 to equal 4!  In that case 4 full
        // 100-year cycles precede the desired day, which implies the desired
        // day is December 31 at the end of a 400-year cycle.
        const n100 = @divFloor(n, DI100Y);
        n = @mod(n, DI100Y);

        // Now compute how many 4-year cycles precede it.
        const n4 = @divFloor(n, DI4Y);
        n = @mod(n, DI4Y);

        // And now how many single years.  Again n1 can be 4, and again meaning
        // that the desired day is December 31 at the end of the 4-year cycle.
        const n1 = @divFloor(n, 365);
        n = @mod(n, 365);

        year += n100 * 100 + n4 * 4 + n1;

        if (n1 == 4 or n100 == 4) {
            assert(n == 0);
            return Date{ .year = @intCast(year - 1), .month = @intCast(12), .day = @intCast(31) };
        }

        // Now the year is correct, and n is the offset from January 1.  We find
        // the month via an estimate that's either exact or one too large.
        const leapyear = (n1 == 3) and (n4 != 24 or n100 == 3);
        assert(leapyear == isLeapYear(year));
        var month = (n + 50) >> 5;
        if (month == 0) month = 12; // Loop around
        var preceding = daysBeforeMonth(year, month);

        if (preceding > n) { // estimate is too large
            month -= 1;
            if (month == 0) month = 12; // Loop around
            preceding -= daysInMonth(year, month);
        }
        n -= preceding;
        // assert(n > 0 and n < daysInMonth(year, month));

        // Now the year and month are correct, and n is the offset from the
        // start of that month:  we're done!
        return Date{ .year = @intCast(year), .month = @intCast(month), .day = @intCast(n + 1) };
    }

    fn isLeapYear(year: u32) bool {
        return year % 4 == 0 and (year % 100 != 0 or year % 400 == 0);
    }

    fn daysBeforeMonth(year: u32, month: u32) u32 {
        assert(month >= 1 and month <= 12);
        var d = DAYS_BEFORE_MONTH[month - 1];
        if (month > 2 and isLeapYear(year)) d += 1;
        return d;
    }

    fn daysInMonth(year: u32, month: u32) u8 {
        assert(1 <= month and month <= 12);
        if (month == 2 and isLeapYear(year)) return 29;
        return DAYS_IN_MONTH[month - 1];
    }

    const Date = struct {
        year: u16,
        month: u4 = 1, // Month of year
        day: u8 = 1, // Day of month
    };
};

const Weekday = enum(u3) {
    Monday = 1,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
};

const Month = enum(u4) {
    January = 1,
    February,
    March,
    April,
    May,
    June,
    July,
    August,
    September,
    October,
    November,
    December,
};
