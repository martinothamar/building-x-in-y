const std = @import("std");
const testing = std.testing;
const assert = std.debug.assert;
const builtin = @import("builtin");
const os = std.os;
const linux = os.linux;
const posix = std.posix;

const queue_size = 1024;

const recv_bgid = 1;
const send_bgid = 2;

// Available from kernel 6.10 (I have 6.9 currently :()
const IORING_FEAT_SEND_BUF_SELECT = 1 << 14;

comptime {
    // Want to use high perf Linux API such as IoUring
    assert(builtin.target.os.tag == .linux);
}

const OpType = enum(u8) {
    none = 0,
    accept = 1,
    recv = 2,
    send = 3,
    close = 4,
};

// Struct used as 'user_data' in IoUring, which is u64-sized
const Op = packed struct(u64) {
    type: OpType,
    fd: posix.fd_t,
    buffer_id: u16,

    _padding: u8,

    inline fn asU64(self: Op) u64 {
        return @as(u64, @bitCast(self));
    }

    inline fn read(value: u64) Op {
        return @as(Op, @bitCast(value));
    }

    comptime {
        // Operations are encoded as userdata in the IO Uring submission queue entries
        // which are u64, and so we verify that here
        const UserDataField: std.builtin.Type.StructField = std.meta.fieldInfo(linux.io_uring_sqe, .user_data);
        assert(UserDataField.type == @typeInfo(Op).Struct.backing_integer orelse unreachable);
        assert(@sizeOf(Op) == @sizeOf(UserDataField.type));
        assert(@alignOf(Op) == @alignOf(UserDataField.type));
    }
};

pub const Server = struct {
    const buffer_count = 128;
    const buffer_size = 1024 * 32;

    host: []const u8,
    port: u16,

    allocator: std.mem.Allocator,

    ring: linux.IoUring,
    buffers_recv: BufferGroup,
    buffers_send: BufferGroup,

    pub fn init(host: []const u8, port: ?u16, allocator: std.mem.Allocator) Server {
        return Server{
            .host = host,
            .port = port orelse 6379,

            .allocator = allocator,

            .ring = undefined,
            .buffers_recv = undefined,
            .buffers_send = undefined,
        };
    }

    pub fn deinit(self: *Server) void {
        self.ring.deinit();

        switch (self.buffers_recv.buffers_allocation) {
            BufferGroupAllocation.buffers => |buffers| self.allocator.free(buffers),
            else => unreachable,
        }
    }

    pub fn run(self: *Server) !void {
        const fd = try self.setup_listener_socket();
        std.log.info("Setup server socket - addr={s} {} fd={}", .{ self.host, self.port, fd });

        try self.init_ring();
        std.log.info("Initialized IOUring", .{});

        self.buffers_recv = try self.alloc_recv_buffer_group();
        self.buffers_send = try self.alloc_send_buffer_group();
        defer self.buffers_recv.deinit();
        defer self.buffers_send.deinit();

        var slabs = try SlabAllocator.init(self.allocator);
        defer slabs.deinit();

        const op_accept: Op = .{ .type = .accept, .fd = fd, .buffer_id = undefined, ._padding = undefined };
        _ = try self.ring.accept_multishot(op_accept.asU64(), fd, null, null, 0);
        std.log.info("Submitted multishot accept", .{});

        var cqes: [256]linux.io_uring_cqe = undefined;
        while (true) {
            std.log.info("Waiting...", .{});
            const ret = try self.ring.submit_and_wait(1);
            assert(self.ring.cq.overflow.* == 0);
            assert((self.ring.sq.flags.* & linux.IORING_SQ_CQ_OVERFLOW) == 0);

            std.log.info("Event loop submitted {}", .{ret});

            const completed = try self.ring.copy_cqes(&cqes, 0);

            for (cqes[0..completed]) |cqe| {
                const op = Op.read(cqe.user_data);
                // std.log.debug("Op: {any}", .{op});
                switch (op.type) {
                    .accept => {
                        std.log.info("[{}] accept", .{cqe.res});
                        assertCqe(&cqe);
                        assert(cqe.flags & linux.IORING_CQE_F_MORE == linux.IORING_CQE_F_MORE);
                        const client_fd: posix.fd_t = @intCast(cqe.res);

                        const op_recv: Op = .{
                            .type = .recv,
                            .fd = client_fd,
                            .buffer_id = 0,
                            ._padding = undefined,
                        };
                        _ = try self.buffers_recv.recv_multishot(op_recv.asU64(), client_fd, 0);
                    },
                    .recv => {
                        assertCqe(&cqe);
                        assert(op.fd >= 0);
                        if (cqe.res == 0) {
                            std.log.info("[{}] recv: len=0", .{op.fd});
                            const op_close: Op = .{
                                .type = .close,
                                .fd = op.fd,
                                .buffer_id = undefined,
                                ._padding = undefined,
                            };
                            _ = try self.ring.close(op_close.asU64(), op.fd);
                        } else {
                            const len: usize = @intCast(cqe.res);
                            assert(cqe.flags & linux.IORING_CQE_F_BUFFER == linux.IORING_CQE_F_BUFFER);
                            const buffer_id = cqe.buffer_id() catch unreachable;
                            const source_buffer = self.buffers_recv.get(buffer_id)[0..len];
                            defer self.buffers_recv.put(buffer_id);
                            std.log.info("[{}] recv: len={}, payload='{s}'", .{ op.fd, len, source_buffer });
                            const slab = slabs.alloc();
                            const send_buffer = slab.buffer[0..len];
                            @memcpy(send_buffer, source_buffer);
                            const op_send: Op = .{
                                .type = .send,
                                .fd = op.fd,
                                .buffer_id = slab.id,
                                ._padding = undefined,
                            };
                            _ = try self.ring.send(op_send.asU64(), op.fd, send_buffer, 0);

                            // self.buffers_send.putBuffer(buffer_id, buffer);
                            // const op_send: Op = .{
                            //     .type = .SEND,
                            //     .fd = op.fd,
                            //     .buffer_id = buffer_id,
                            //     ._padding = undefined,
                            // };
                            // var sqe = try self.ring.get_sqe();
                            // sqe.prep_rw(.SEND, op.fd, 0, len, 0);
                            // sqe.rw_flags = linux.MSG.WAITALL | linux.MSG.NOSIGNAL;
                            // sqe.flags = 0;
                            // sqe.flags |= linux.IOSQE_BUFFER_SELECT;
                            // sqe.buf_index = self.buffers_send.group_id;
                            // sqe.user_data = op_send.asU64();
                        }
                    },
                    .send => {
                        assertCqe(&cqe);
                        // assert(cqe.flags & linux.IORING_CQE_F_BUFFER == linux.IORING_CQE_F_BUFFER);
                        // const buffer_id = try cqe.buffer_id();
                        // assert(op.buffer_id < self.buffers_recv.buffers_count);
                        // assert(buffer_id == op.buffer_id);
                        slabs.dealloc(op.buffer_id);
                        const len: usize = @intCast(cqe.res);
                        std.log.info("[{}] send: len={}", .{ op.fd, len });
                        // self.buffers_recv.put(buffer_id);
                    },
                    .close => {
                        assertCqe(&cqe);
                        assertWith(cqe.res == 0, "unexpected cqe result for close: {}", .{cqe.res});
                        std.log.info("[{}] close", .{op.fd});
                    },
                    else => unreachable,
                }

                // Register client fd to ring
            }
        }
    }

    fn assertCqe(cqe: *const linux.io_uring_cqe) void {
        switch (cqe.err()) {
            .SUCCESS => {},
            else => |errno| unreachableWith("cqe error: {}", .{errno}),
        }
        assertWith(cqe.res >= 0, "unexpected cqe result value: {}", .{cqe.res});
    }

    fn setup_listener_socket(self: *Server) !posix.socket_t {
        const fd = try posix.socket(posix.AF.INET, posix.SOCK.STREAM, 0);

        try posix.setsockopt(fd, posix.SOL.SOCKET, posix.SO.REUSEPORT, &std.mem.toBytes(@as(c_int, 1)));

        const address = try std.net.Address.parseIp4(self.host, self.port);
        try posix.bind(fd, &address.any, address.getOsSockLen());

        try posix.listen(fd, queue_size);
        return fd;
    }

    fn init_ring(self: *Server) !void {
        var params = std.mem.zeroes(linux.io_uring_params);
        params.flags |= linux.IORING_SETUP_SINGLE_ISSUER | linux.IORING_SETUP_CLAMP;
        params.flags |= linux.IORING_SETUP_CQSIZE;
        params.flags |= linux.IORING_SETUP_DEFER_TASKRUN;
        params.cq_entries = queue_size;

        self.ring = try linux.IoUring.init_params(queue_size, &params);
        // Need 6.10 kernel
        // assert(params.features & IORING_FEAT_SEND_BUF_SELECT == IORING_FEAT_SEND_BUF_SELECT);

        try self.io_uring_register_files_sparse(4);
        try self.io_uring_register_ring_fd();
    }

    fn alloc_recv_buffer_group(self: *Server) !BufferGroup {
        const buffers = try self.allocator.alloc(u8, buffer_size * buffer_count);
        return try BufferGroup.init(&self.ring, recv_bgid, .{
            .buffers = buffers,
        }, buffer_size, buffer_count);
    }

    fn alloc_send_buffer_group(self: *Server) !BufferGroup {
        return try BufferGroup.init(&self.ring, send_bgid, .{ .buffer_group = &self.buffers_recv }, buffer_size, buffer_count);
    }

    fn io_uring_register_files_sparse(self: *const Server, nr: u32) !void {
        var reg = std.mem.zeroes(linux.io_uring_rsrc_register);
        reg.flags = linux.IORING_RSRC_REGISTER_SPARSE;
        reg.nr = nr;

        const ret = self.do_register(linux.IORING_REGISTER.REGISTER_FILES2, &reg, @sizeOf(@TypeOf(reg)));
        switch (linux.E.init(ret)) {
            .SUCCESS => {},
            .MFILE => return error.ProcessFdQuotaExceeded,
            else => |errno| return posix.unexpectedErrno(errno),
        }
    }

    fn io_uring_register_ring_fd(self: *const Server) !void {
        var reg = std.mem.zeroes(linux.io_uring_rsrc_update);
        reg.data = @intCast(self.ring.fd);
        reg.offset = std.math.maxInt(u32);

        // if (ring->int_flags & INT_FLAG_REG_RING)
        //     return -EEXIST;

        const ret = self.do_register(linux.IORING_REGISTER.REGISTER_RING_FDS, &reg, 1);
        switch (linux.E.init(ret)) {
            .SUCCESS => {},
            else => |errno| return posix.unexpectedErrno(errno),
        }
    }

    fn do_register(self: *const Server, opcode: linux.IORING_REGISTER, arg: ?*const anyopaque, nr_args: u32) usize {
        // if (ring->int_flags & INT_FLAG_REG_REG_RING) {
        //     opcode |= IORING_REGISTER_USE_REGISTERED_RING;
        //     fd = ring->enter_ring_fd;
        // } else {
        //     fd = ring->ring_fd;
        // }
        return linux.io_uring_register(self.ring.fd, opcode, arg, nr_args);
    }
};

const BufferGroupAllocation = union(enum) {
    buffers: []u8,
    buffer_group: *BufferGroup,
};

const BufferGroup = struct {
    /// Parent ring for which this group is registered.
    ring: *linux.IoUring,
    /// Pointer to the memory shared by the kernel.
    /// `buffers_count` of `io_uring_buf` structures are shared by the kernel.
    /// First `io_uring_buf` is overlaid by `io_uring_buf_ring` struct.
    br: *align(std.mem.page_size) linux.io_uring_buf_ring,
    /// Contiguous block of memory of size (buffers_count * buffer_size).
    buffers_allocation: BufferGroupAllocation,
    /// Size of each buffer in buffers.
    buffer_size: u32,
    // Number of buffers in `buffers`, number of `io_uring_buf structures` in br.
    buffers_count: u16,
    /// ID of this group, must be unique in ring.
    group_id: u16,

    pub fn init(
        ring: *linux.IoUring,
        group_id: u16,
        buffers_allocation: BufferGroupAllocation,
        buffer_size: u32,
        buffers_count: u16,
    ) !BufferGroup {
        const br = try linux.IoUring.setup_buf_ring(ring.fd, buffers_count, group_id);
        linux.IoUring.buf_ring_init(br);

        switch (buffers_allocation) {
            BufferGroupAllocation.buffers => |buffers| {
                assert(buffers.len == buffers_count * buffer_size);
                const mask = linux.IoUring.buf_ring_mask(buffers_count);
                var i: u16 = 0;
                while (i < buffers_count) : (i += 1) {
                    const start = buffer_size * i;
                    const buf = buffers[start .. start + buffer_size];
                    linux.IoUring.buf_ring_add(br, buf, i, mask, i);
                }
                linux.IoUring.buf_ring_advance(br, buffers_count);
            },
            else => {},
        }

        return BufferGroup{
            .ring = ring,
            .group_id = group_id,
            .br = br,
            .buffers_allocation = buffers_allocation,
            .buffer_size = buffer_size,
            .buffers_count = buffers_count,
        };
    }

    // Get buffer by id.
    pub fn get(self: *BufferGroup, buffer_id: u16) []u8 {
        switch (self.buffers_allocation) {
            BufferGroupAllocation.buffers => |buffers| {
                const head = self.buffer_size * buffer_id;
                return buffers[head .. head + self.buffer_size];
            },
            BufferGroupAllocation.buffer_group => |*bufferGroup| {
                return bufferGroup.*.get(buffer_id);
            },
        }
    }

    pub fn put(self: *BufferGroup, buffer_id: u16) void {
        assert(std.meta.activeTag(self.buffers_allocation) == BufferGroupAllocation.buffers);
        const mask = linux.IoUring.buf_ring_mask(self.buffers_count);
        const buffer = self.get(buffer_id);
        linux.IoUring.buf_ring_add(self.br, buffer, buffer_id, mask, 0);
        linux.IoUring.buf_ring_advance(self.br, 1);
    }

    pub fn putBuffer(self: *BufferGroup, buffer_id: u16, buffer: []u8) void {
        assert(std.meta.activeTag(self.buffers_allocation) == BufferGroupAllocation.buffer_group);
        const mask = linux.IoUring.buf_ring_mask(self.buffers_count);
        linux.IoUring.buf_ring_add(self.br, buffer, buffer_id, mask, 0);
        linux.IoUring.buf_ring_advance(self.br, 1);
    }

    pub fn deinit(self: *BufferGroup) void {
        linux.IoUring.free_buf_ring(self.ring.fd, self.br, self.buffers_count, self.group_id);
    }

    // Prepare recv operation which will select buffer from this group.
    pub fn recv(self: *BufferGroup, user_data: u64, fd: posix.fd_t, flags: u32) !*linux.io_uring_sqe {
        var sqe = try self.ring.get_sqe();
        sqe.prep_rw(.RECV, fd, 0, 0, 0);
        sqe.rw_flags = flags;
        sqe.flags |= linux.IOSQE_BUFFER_SELECT;
        sqe.buf_index = self.group_id;
        sqe.user_data = user_data;
        return sqe;
    }

    // Prepare multishot recv operation which will select buffer from this group.
    pub fn recv_multishot(self: *BufferGroup, user_data: u64, fd: posix.fd_t, flags: u32) !*linux.io_uring_sqe {
        var sqe = try self.recv(user_data, fd, flags);
        sqe.ioprio |= linux.IORING_RECV_MULTISHOT;
        return sqe;
    }
};

const SlabAllocator = struct {
    const sentinel: u16 = std.math.maxInt(u16);

    entries: std.ArrayList(InternalEntry),
    allocator: std.mem.Allocator,
    len: usize,
    next: u16,

    fn init(allocator: std.mem.Allocator) !SlabAllocator {
        return SlabAllocator{
            .entries = try std.ArrayList(InternalEntry).initCapacity(allocator, 128),
            .allocator = allocator,
            .len = 0,
            .next = 0,
        };
    }

    fn deinit(self: *SlabAllocator) void {
        self.entries.deinit();
        self.* = undefined;
    }

    fn alloc(self: *SlabAllocator) Entry {
        assert(self.len < sentinel - 1);
        const id = self.next;
        self.len += 1;
        const idIndex = @as(usize, id);

        if (idIndex == self.entries.items.len) {
            const buffer = self.allocator.alignedAlloc(u8, std.mem.page_size, std.mem.page_size) catch unreachable;
            const entry = InternalEntry{
                .buffer = buffer,
                .next = sentinel,
                .taken = true,
            };
            self.entries.append(entry) catch unreachable;
            self.next = id + 1;
            return Entry{
                .buffer = entry.buffer,
                .id = id,
            };
        } else {
            assert(idIndex <= self.entries.items.len);
            const entry = &self.entries.items[idIndex];
            assert(entry.taken == false);
            assert(entry.next != sentinel);
            entry.taken = true;
            self.next = entry.next;
            entry.next = sentinel;
            return Entry{
                .buffer = entry.buffer,
                .id = id,
            };
        }
    }

    fn get(self: *SlabAllocator, id: u16) Entry {
        const idIndex = @as(usize, id);
        assert(idIndex < self.entries.items.len);
        const entry = &self.entries.items[idIndex];
        assert(entry.taken == true);
        return Entry{
            .buffer = entry.buffer,
            .id = id,
        };
    }

    fn dealloc(self: *SlabAllocator, id: u16) void {
        const idIndex = @as(usize, id);
        assert(idIndex < self.entries.items.len);
        const entry = &self.entries.items[idIndex];
        assert(entry.taken == true);
        assert(entry.next == sentinel);
        entry.taken = false;
        entry.next = self.next;
        self.len -= 1;
        self.next = id;
    }

    const Entry = struct {
        buffer: []align(std.mem.page_size) u8,
        id: u16,
    };

    const InternalEntry = struct {
        buffer: []align(std.mem.page_size) u8,
        next: u16,
        taken: bool,
    };
};

fn assertWith(
    ok: bool,
    comptime format: []const u8,
    args: anytype,
) void {
    if (!ok) {
        std.log.err(format, args);
        unreachable;
    }
}

fn unreachableWith(comptime format: []const u8, args: anytype) void {
    std.log.err(format, args);
    unreachable;
}

test "init" {
    var server = Server.init("127.0.0.1", null);
    try server.run();
    defer server.deinit();
}
