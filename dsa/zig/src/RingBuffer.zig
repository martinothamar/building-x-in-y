const std = @import("std");
const Allocator = std.mem.Allocator;
const print = std.debug.print;
const assert = std.debug.assert;
const math = std.math;

pub const Error = error{ Full, Empty };

pub fn RingBuffer(comptime T: type, comptime capacity: usize) type {
    comptime {
        assert(math.isPowerOfTwo(@sizeOf(T)));
        assert(math.isPowerOfTwo(@alignOf(T)));
        assert(math.isPowerOfTwo(capacity));
    }

    if (@sizeOf(T) * capacity > 1024) {
        return struct {
            const Self = @This();

            storage: [capacity]T,
            write_index: usize,
            read_index: usize,
            allocator: Allocator,

            pub fn init(allocator: Allocator) Allocator.Error!*Self {
                const memory = try allocator.alignedAlloc(Self, 64, 1);
                var rb = &memory[0];
                rb.storage = undefined;
                rb.write_index = 0;
                rb.read_index = 0;
                rb.allocator = allocator;
                return rb;
            }

            // Would be nice is this worked...
            // ---
            // comptime if (@sizeOf(T) * capacity > 1024) {
            //     allocator: Allocator,

            //     pub fn init(allocator: Allocator) Allocator.Error!*Self {
            //             const memory = try allocator.alignedAlloc(Self, 64, 1);
            //             var rb = &memory[0];
            //             return rb;
            //     }
            // } else {
            //     pub fn init() Self {
            //         return Self{
            //             .storage = undefined,
            //             .write_index = 0,
            //             .read_index = 0,
            //             .allocator = allocator,
            //         };
            //     }
            // }

            pub fn deinit(self: *Self) void {
                var allocator = self.allocator;
                self.* = undefined;
                allocator.destroy(self);
            }

            pub inline fn push(self: *Self, value: T) !void {
                if (self.isFull()) {
                    return error.Full;
                }

                self.storage[mask(capacity, self.write_index)] = value;
                self.write_index = mask2(capacity, self.write_index + 1);
            }

            pub inline fn pop(self: *Self) !T {
                if (self.isEmpty()) {
                    return error.Empty;
                }

                const value = self.storage[mask(capacity, self.read_index)];
                self.read_index = mask2(capacity, self.read_index + 1);
                return value;
            }

            pub inline fn len(self: Self) usize {
                const wrap_offset = 2 * capacity * intFromBool(self.write_index < self.read_index);
                const adjusted_write_index = self.write_index + wrap_offset;
                return adjusted_write_index - self.read_index;
            }

            pub inline fn isEmpty(self: Self) bool {
                return self.write_index == self.read_index;
            }

            pub inline fn isFull(self: Self) bool {
                return mask2(capacity, self.write_index + capacity) == self.read_index;
            }
        };
    } else {
        return struct {
            const Self = @This();

            storage: [capacity]T,
            write_index: usize,
            read_index: usize,

            pub fn init() Self {
                return Self{
                    .storage = undefined,
                    .write_index = 0,
                    .read_index = 0,
                };
            }

            pub inline fn push(self: *Self, value: T) !void {
                if (self.isFull()) {
                    return error.Full;
                }

                self.storage[mask(capacity, self.write_index)] = value;
                self.write_index = mask2(capacity, self.write_index + 1);
            }

            pub inline fn pop(self: *Self) !T {
                if (self.isEmpty()) {
                    return error.Empty;
                }

                const value = self.storage[mask(capacity, self.read_index)];
                self.read_index = mask2(capacity, self.read_index + 1);
                return value;
            }

            pub inline fn len(self: Self) usize {
                const wrap_offset = 2 * capacity * intFromBool(self.write_index < self.read_index);
                const adjusted_write_index = self.write_index + wrap_offset;
                return adjusted_write_index - self.read_index;
            }

            pub inline fn isEmpty(self: Self) bool {
                return self.write_index == self.read_index;
            }

            pub inline fn isFull(self: Self) bool {
                return mask2(capacity, self.write_index + capacity) == self.read_index;
            }
        };
    }
}

pub inline fn mask(comptime capacity: usize, index: usize) usize {
    return index & (capacity - 1);
}

pub inline fn mask2(comptime capacity: usize, index: usize) usize {
    return index & ((2 * capacity) - 1);
}

inline fn intFromBool(value: bool) usize {
    if (value) {
        return 1;
    } else {
        return 0;
    }
}

test "heap - produce and consume sequentially" {
    const rb = try RingBuffer(usize, 1024).init(std.testing.allocator);
    defer rb.deinit();

    var value: usize = 1;
    while (value <= 16) : (value += 1) {
        // print("value: {d}\n", .{value});

        try std.testing.expectEqual(@as(usize, 0), rb.len());
        try rb.push(value);
        try std.testing.expectEqual(@as(usize, 1), rb.len());
        try std.testing.expectEqual(value, try rb.pop());
    }
}

test "stack - produce and consume sequentially" {
    var rb = RingBuffer(usize, 8).init();

    var value: usize = 1;
    while (value <= 16) : (value += 1) {
        // print("value: {d}\n", .{value});

        try std.testing.expectEqual(@as(usize, 0), rb.len());
        try rb.push(value);
        try std.testing.expectEqual(@as(usize, 1), rb.len());
        try std.testing.expectEqual(value, try rb.pop());
    }
}
