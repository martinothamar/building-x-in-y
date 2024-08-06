const std = @import("std");
const lib = @import("root.zig");
const Server = lib.server.Server;
const assert = std.debug.assert;

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{
        .thread_safe = false,
    }){};
    defer {
        const alloc_check = gpa.deinit();
        assert(alloc_check == .ok);
    }

    const allocator = gpa.allocator();
    var server = try Server.init("127.0.0.1", null, &allocator);
    defer server.deinit();
    try server.run();
}
