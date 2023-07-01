const std = @import("std");
const Allocator = std.mem.Allocator;
const print = std.debug.print;
const assert = std.debug.assert;
const math = std.math;
const RingBuffer = @import("RingBuffer.zig").RingBuffer;

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    var allocator = gpa.allocator();
    var rb = try RingBuffer(usize, 1024).init(allocator);
    defer rb.deinit();

    var value: usize = 1;
    while (value <= 16) : (value += 1) {
        try rb.push(value);
        const popped = try rb.pop();

        print("value: {}, {}\n", .{ value, popped });
    }
}
