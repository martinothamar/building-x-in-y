pub const IoUring = @import("IoUring.zig");
const std = @import("std");
const linux = std.os.linux;
const posix = std.posix;
const assertion = @import("../assertion.zig");
const unreachableWith = assertion.unreachableWith;

// Available from kernel 6.10, but is not in Zig yet
// also covers recv/send bundles. This is currently the newest kernel feature used.
pub const IORING_FEAT_SEND_BUF_SELECT = 1 << 14;

// Not in Zig yet
pub const IORING_RECVSEND_BUNDLE = 1 << 4;

// Not in zig yet
pub const IORING_TIMEOUT_MULTISHOT = 1 << 6;

pub fn io_uring_submit_and_wait_timeout(fd: i32, to_submit: u32, wait_nr: u32, ts: *const linux.kernel_timespec, sigmask: ?*linux.sigset_t) usize {
    const arg = linux.io_uring_getevents_arg{
        .sigmask = @intFromPtr(sigmask),
        .sigmask_sz = linux.NSIG / 8,
        .ts = @intFromPtr(ts),
        .pad = undefined,
    };

    // 	struct get_data data = {
    // 	.submit		= __io_uring_flush_sq(ring),
    // 	.wait_nr	= wait_nr,
    // 	.get_flags	= IORING_ENTER_EXT_ARG,
    // 	.sz		= sizeof(arg),
    // 	.has_ts		= ts != NULL,
    // 	.arg		= &arg
    // };

    const flags = linux.IORING_ENTER_EXT_ARG | linux.IORING_ENTER_REGISTERED_RING | linux.IORING_ENTER_GETEVENTS;
    return io_uring_enter2(
        fd,
        to_submit,
        wait_nr,
        flags,
        @intFromPtr(&arg),
        @sizeOf(@TypeOf(arg)),
    );
}

pub fn io_uring_enter2(fd: i32, to_submit: u32, min_complete: u32, flags: u32, sig: u64, sz: usize) usize {
    return linux.syscall6(
        .io_uring_enter,
        @as(usize, @bitCast(@as(isize, fd))),
        to_submit,
        min_complete,
        flags,
        sig,
        sz,
    );
}

pub fn io_uring_register_files_sparse(ring: *linux.IoUring, nr: u32) !void {
    var reg = std.mem.zeroes(linux.io_uring_rsrc_register);
    reg.flags = linux.IORING_RSRC_REGISTER_SPARSE;
    reg.nr = nr;

    const ret = do_register(ring, linux.IORING_REGISTER.REGISTER_FILES2, &reg, @sizeOf(@TypeOf(reg)));
    switch (linux.E.init(ret)) {
        .SUCCESS => {},
        .MFILE => return error.ProcessFdQuotaExceeded,
        else => |errno| unreachableWith("register file error: {}", .{errno}),
    }
}

pub fn io_uring_register_ring_fd(ring: *linux.IoUring) !u32 {
    var reg = std.mem.zeroes(linux.io_uring_rsrc_update);
    reg.data = @intCast(ring.fd);
    reg.offset = std.math.maxInt(u32);

    // if (ring->int_flags & INT_FLAG_REG_RING)
    //     return -EEXIST;

    const ret = do_register(ring, linux.IORING_REGISTER.REGISTER_RING_FDS, &reg, 1);
    switch (linux.E.init(ret)) {
        .SUCCESS => {},
        else => |errno| unreachableWith("register ring fd error: {}", .{errno}),
    }

    return reg.offset;
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
