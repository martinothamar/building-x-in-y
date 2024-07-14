const std = @import("std");

pub fn assertWith(
    ok: bool,
    comptime format: []const u8,
    args: anytype,
) void {
    if (!ok) {
        std.log.err(format, args);
        unreachable;
    }
}

pub fn unreachableWith(comptime format: []const u8, args: anytype) void {
    std.log.err(format, args);
    unreachable;
}
