pub const IoUring = @import("IoUring.zig");
const std = @import("std");
const linux = std.os.linux;
const posix = std.posix;

pub fn io_uring_register_files_sparse(ring: *linux.IoUring, nr: u32) !void {
    var reg = std.mem.zeroes(linux.io_uring_rsrc_register);
    reg.flags = linux.IORING_RSRC_REGISTER_SPARSE;
    reg.nr = nr;

    const ret = do_register(ring, linux.IORING_REGISTER.REGISTER_FILES2, &reg, @sizeOf(@TypeOf(reg)));
    switch (linux.E.init(ret)) {
        .SUCCESS => {},
        .MFILE => return error.ProcessFdQuotaExceeded,
        else => |errno| return posix.unexpectedErrno(errno),
    }
}

pub fn io_uring_register_ring_fd(ring: *linux.IoUring) !void {
    var reg = std.mem.zeroes(linux.io_uring_rsrc_update);
    reg.data = @intCast(ring.fd);
    reg.offset = std.math.maxInt(u32);

    // if (ring->int_flags & INT_FLAG_REG_RING)
    //     return -EEXIST;

    const ret = do_register(ring, linux.IORING_REGISTER.REGISTER_RING_FDS, &reg, 1);
    switch (linux.E.init(ret)) {
        .SUCCESS => {},
        else => |errno| return posix.unexpectedErrno(errno),
    }
}

fn do_register(ring: *linux.IoUring, opcode: linux.IORING_REGISTER, arg: ?*const anyopaque, nr_args: u32) usize {
    // if (ring->int_flags & INT_FLAG_REG_REG_RING) {
    //     opcode |= IORING_REGISTER_USE_REGISTERED_RING;
    //     fd = ring->enter_ring_fd;
    // } else {
    //     fd = ring->ring_fd;
    // }
    return linux.io_uring_register(ring.fd, opcode, arg, nr_args);
}
