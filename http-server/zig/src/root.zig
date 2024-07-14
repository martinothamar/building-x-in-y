const std = @import("std");
const linuxext = @import("linuxext/linuxext.zig");
const alloc = @import("alloc.zig");
const assertion = @import("assertion.zig");
const testing = std.testing;
const assert = std.debug.assert;
const assertWith = assertion.assertWith;
const unreachableWith = assertion.unreachableWith;
const builtin = @import("builtin");
const os = std.os;
const linux = os.linux;
const posix = std.posix;
const BufferGroupRecv = linux.IoUring.BufferGroup;
const BufferGroupSend = linuxext.IoUring.BufferGroupSend;
const HttpServer = std.http.Server;

const queue_size = 1024 * 4;

const recv_bgid = 1;
const send_bgid = 2;

// Available from kernel 6.10, but is not in Zig yet
// also covers recv/send bundles. This is currently the newest kernel feature used.
const IORING_FEAT_SEND_BUF_SELECT = 1 << 14;

// Not in Zig yet
const IORING_RECVSEND_BUNDLE = 1 << 4;

comptime {
    // Want to use high perf Linux API such as IO uring,
    // so we need Linux
    assert(builtin.target.os.tag == .linux);
}

const OpType = enum(u8) {
    none = 0,
    accept = 1,
    recv = 2,
    send = 3,
    close = 4,
};

// Struct used as 'user_data' in IO uring, which is u64-sized
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
        // Operations are encoded as userdata in the IO uring submission queue entries
        // which are u64, and so we verify that here
        const UserDataField: std.builtin.Type.StructField = std.meta.fieldInfo(linux.io_uring_sqe, .user_data);
        assert(UserDataField.type == @typeInfo(Op).Struct.backing_integer orelse unreachable);
        assert(@sizeOf(Op) == @sizeOf(UserDataField.type));
        assert(@alignOf(Op) == @alignOf(UserDataField.type));
    }
};

pub const Server = struct {
    // Parameters for the ring provided buffers for IO uring
    const buffer_count = 1024 * 4;
    const buffer_size = 1024 * 4;

    host: []const u8,
    port: u16,
    listener_fd: posix.socket_t,

    allocator: *const std.mem.Allocator,
    slabs: alloc.SlabAllocator,

    ring: linux.IoUring,
    buffers_recv: BufferGroupRecv,
    // buffers_send: BufferGroupSend,

    pub fn init(host: []const u8, port: ?u16, allocator: *const std.mem.Allocator) !*Server {
        var self = try allocator.create(Server);
        self.* = Server{
            .host = host,
            .port = port orelse 8080,
            .listener_fd = undefined,

            .allocator = allocator,
            .slabs = try alloc.SlabAllocator.init(allocator, buffer_size, buffer_count),

            .ring = undefined,
            .buffers_recv = undefined,
            // .buffers_send = undefined,
        };

        self.listener_fd = try self.setup_listener_socket();
        std.log.info("Setup server socket - addr={s} {} fd={}", .{ self.host, self.port, self.listener_fd });

        try self.init_ring();
        std.log.info("Initialized IOUring", .{});

        self.buffers_recv = try self.alloc_buffer_group(recv_bgid);
        // self.buffers_send = try BufferGroupSend.init(&self.ring, send_bgid, buffer_count);
        std.log.info("Setup provided buffer rings", .{});

        const op_accept: Op = .{ .type = .accept, .fd = self.listener_fd, .buffer_id = undefined, ._padding = undefined };
        _ = try self.ring.accept_multishot(op_accept.asU64(), self.listener_fd, null, null, 0);
        std.log.info("Submitted multishot accept", .{});

        return self;
    }

    pub fn deinit(self: *Server) void {
        self.slabs.deinit();
        self.allocator.free(self.buffers_recv.buffers);
        self.ring.deinit();
        self.allocator.destroy(self);
    }

    pub fn run(self: *Server) !void {
        var cqes: [256]linux.io_uring_cqe = undefined;
        while (true) {
            // std.log.debug("Waiting...", .{});
            _ = try self.ring.submit_and_wait(1);
            assert(self.ring.cq.overflow.* == 0);
            assert((self.ring.sq.flags.* & linux.IORING_SQ_CQ_OVERFLOW) == 0);

            // std.log.info("Event loop submitted {}", .{ret});

            const completed = try self.ring.copy_cqes(&cqes, 0);

            for (cqes[0..completed]) |cqe| {
                const op = Op.read(cqe.user_data);
                const op_buffer_id = op.buffer_id;
                _ = op_buffer_id;
                // std.log.debug("Op: {any}", .{op});
                switch (op.type) {
                    .accept => {
                        // std.log.info("[{}] accept", .{cqe.res});
                        assertCqe(&cqe);
                        assert(cqe.flags & linux.IORING_CQE_F_MORE == linux.IORING_CQE_F_MORE);
                        const client_fd: posix.fd_t = @intCast(cqe.res);

                        const op_recv: Op = .{
                            .type = .recv,
                            .fd = client_fd,
                            .buffer_id = 0,
                            ._padding = undefined,
                        };

                        var sqe = try self.ring.get_sqe();
                        sqe.prep_rw(.RECV, client_fd, 0, 0, 0);
                        sqe.rw_flags = 0;
                        sqe.flags |= linux.IOSQE_BUFFER_SELECT;
                        sqe.ioprio |= linux.IORING_RECV_MULTISHOT;
                        // sqe.ioprio |= IORING_RECVSEND_BUNDLE;
                        sqe.buf_index = self.buffers_recv.group_id;
                        sqe.user_data = op_recv.asU64();
                    },
                    .recv => {
                        assertCqe(&cqe);
                        assert(op.fd >= 0);
                        if (cqe.res == 0) {
                            // std.log.info("[{}] recv: len=0", .{op.fd});
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
                            const recv_buffer_id = cqe.buffer_id() catch unreachable;
                            const recv_buffer = self.buffers_recv.get(recv_buffer_id);
                            defer self.buffers_recv.put(recv_buffer_id);

                            const head = try parse_request(recv_buffer[0..len]);

                            // std.log.info("[{}] recv: len={}", .{ op.fd, len });

                            const slab = try self.slabs.alloc();
                            const send_buffer_id = slab.id;
                            var send_buffer = std.ArrayListAlignedUnmanaged(u8, std.mem.page_size).initBuffer(slab.buffer);
                            write_response(&head, &send_buffer);

                            // self.buffers_send.put(send_buffer_id, send_buffer.items);
                            const op_send: Op = .{
                                .type = .send,
                                .fd = op.fd,
                                .buffer_id = send_buffer_id,
                                ._padding = undefined,
                            };
                            var sqe = try self.ring.get_sqe();
                            sqe.prep_rw(.SEND, op.fd, @intFromPtr(send_buffer.items.ptr), send_buffer.items.len, 0);
                            sqe.rw_flags = linux.MSG.WAITALL | linux.MSG.NOSIGNAL;
                            // sqe.ioprio |= IORING_RECVSEND_BUNDLE;
                            // sqe.flags |= linux.IOSQE_BUFFER_SELECT;
                            // sqe.buf_index = self.buffers_send.group_id;
                            sqe.user_data = op_send.asU64();
                        }
                    },
                    .send => {
                        // if (cqe.err() == linux.E.NOBUFS) {
                        //     unreachableWith("No more buffers - sends={}/{}, send_buf_id={}", .{ sends_acked, sends, prev_send_buffer_id });
                        // }
                        assertCqe(&cqe);
                        // assert(cqe.flags & linux.IORING_CQE_F_BUFFER == linux.IORING_CQE_F_BUFFER);
                        // const buffer_id = cqe.buffer_id() catch unreachable;
                        // assert(buffer_id == op.buffer_id);
                        // const len: usize = @intCast(cqe.res);
                        // std.log.info("[{}] send: len={}", .{ op.fd, len });
                        self.slabs.dealloc(op.buffer_id);
                    },
                    .close => {
                        assertCqe(&cqe);
                        assertWith(cqe.res == 0, "unexpected cqe result for close: {}", .{cqe.res});
                        // std.log.info("[{}] close", .{op.fd});
                    },
                    else => unreachable,
                }
            }

            // self.buffers_send.maybeCommitBuffers();
        }
    }

    fn parse_request(buffer: []const u8) !HttpServer.Request.Head {
        const head = try HttpServer.Request.Head.parse(buffer);
        // assert(head.version == std.http.Version.@"HTTP/1.1");
        return head;
    }

    fn write_response(head: *const HttpServer.Request.Head, writer: *std.ArrayListAlignedUnmanaged(u8, std.mem.page_size)) void {
        assert(writer.items.len == 0);
        if (head.method == std.http.Method.GET and std.mem.endsWith(u8, head.target, "/plaintext")) {
            writer.appendSliceAssumeCapacity("HTTP/1.1 200 OK\r\n");
            writer.appendSliceAssumeCapacity("Server: Z\r\n");
            writer.appendSliceAssumeCapacity("Content-Length: 13\r\n");
            writer.appendSliceAssumeCapacity("Content-Type: text/plain\r\n");
            writer.appendSliceAssumeCapacity("Date: Sun, 14 Jul 2024 23:59:59 GMT\r\n");
            writer.appendSliceAssumeCapacity("\r\n");
            writer.appendSliceAssumeCapacity("Hello, World!");
        } else {
            writer.appendSliceAssumeCapacity("HTTP/1.1 404 Not Found\r\n");
            writer.appendSliceAssumeCapacity("Content-Length: 0\r\n");
            writer.appendSliceAssumeCapacity("\r\n");
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

        try posix.listen(fd, 128);
        return fd;
    }

    fn init_ring(self: *Server) !void {
        var params = std.mem.zeroes(linux.io_uring_params);
        params.flags |= linux.IORING_SETUP_SINGLE_ISSUER;
        params.flags |= linux.IORING_SETUP_CLAMP;
        params.flags |= linux.IORING_SETUP_CQSIZE;
        params.flags |= linux.IORING_SETUP_DEFER_TASKRUN;
        params.cq_entries = queue_size;

        self.ring = try linux.IoUring.init_params(queue_size, &params);
        assert(params.features & IORING_FEAT_SEND_BUF_SELECT == IORING_FEAT_SEND_BUF_SELECT);

        try linuxext.io_uring_register_files_sparse(&self.ring, 4);
        try linuxext.io_uring_register_ring_fd(
            &self.ring,
        );
    }

    fn alloc_buffer_group(self: *Server, bgid: u16) !BufferGroupRecv {
        const buffers = try self.allocator.alignedAlloc(u8, std.mem.page_size, buffer_size * buffer_count);
        return try BufferGroupRecv.init(&self.ring, bgid, buffers, buffer_size, buffer_count);
    }
};

test "init" {
    const allocator = std.testing.allocator;
    var server = try Server.init("127.0.0.1", null, &allocator);
    try server.run();
    defer server.deinit();
}
