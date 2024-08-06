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
const datetime = @import("datetime.zig");

const queue_size = 1024;

const recv_bgid = 1;
const send_bgid = 2;

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
    host: []const u8 align(std.atomic.cache_line),
    port: u16,

    threads: []*ServerThread,
    allocator: *const std.mem.Allocator,

    start_signal: std.Thread.ResetEvent,
    shutting_down: bool,

    current_time_str_buffer_1: [64]u8,
    current_time_str_buffer_2: [64]u8,
    current_time_str_buffer_3: [64]u8,
    current_time_str_buffer_4: [64]u8,
    current_time_str: []const u8,
    current_time_str_ref: ?*[]const u8,

    comptime {
        assert(@alignOf(@This()) == std.atomic.cache_line);
    }

    pub fn init(host: []const u8, port: ?u16, allocator: *const std.mem.Allocator) !*Server {
        const self = try allocator.create(Server);
        errdefer allocator.destroy(self);

        var cpus = try std.Thread.getCpuCount();
        cpus /= 1;

        const threads = try allocator.alloc(*ServerThread, cpus);
        errdefer allocator.free(threads);

        const port_to_use = port orelse 8080;

        self.* = Server{
            .host = host,
            .port = port_to_use,

            .threads = threads,
            .allocator = allocator,

            .start_signal = std.Thread.ResetEvent{},
            .shutting_down = false,

            .current_time_str_buffer_1 = undefined,
            .current_time_str_buffer_2 = undefined,
            .current_time_str_buffer_3 = undefined,
            .current_time_str_buffer_4 = undefined,
            .current_time_str = undefined,
            .current_time_str_ref = null,
        };

        try self.handle_signal();

        for (0..cpus) |cpu| {
            const thread = try ServerThread.init(cpu, self);
            threads[cpu] = thread;
        }

        return self;
    }

    pub fn deinit(self: *Server) void {
        if (SignalHandler.shutting_down) |shutdown| {
            if (shutdown == &self.shutting_down) {
                SignalHandler.shutting_down = null;
            }
        }
        for (self.threads) |thread| {
            thread.deinit();
        }
        self.allocator.free(self.threads);
        const allocator = self.allocator.*;
        self.* = undefined;
        allocator.destroy(self);
    }

    pub fn run(self: *Server) !void {
        std.time.sleep(std.time.ns_per_s * 1);

        try self.update_timestamp();

        self.start_signal.set();

        while (!self.shutting_down) {
            std.time.sleep(std.time.ns_per_s * 1);
            try self.update_timestamp();
        }

        std.log.info("Shutting down, waiting for threads to exit", .{});

        for (self.threads) |thread| {
            assert(thread.thread != null);
            thread.thread.?.join();
        }

        std.log.info("Done!", .{});
    }

    fn handle_signal(self: *Server) !void {
        SignalHandler.shutting_down = &self.shutting_down;
        const act = posix.Sigaction{
            .handler = .{ .sigaction = SignalHandler.handle },
            .mask = posix.empty_sigset,
            .flags = 0,
        };
        var oact: posix.Sigaction = undefined;
        try posix.sigaction(posix.SIG.INT, &act, &oact);
        SignalHandler.original_handler = oact.handler.sigaction;
    }

    const SignalHandler = struct {
        const Self = @This();
        var shutting_down: ?*bool = null;

        var original_handler: ?*const fn (
            i32,
            *const posix.siginfo_t,
            ?*anyopaque,
        ) callconv(.C) void = null;

        fn handle(sig: i32, info: *const posix.siginfo_t, o: ?*anyopaque) callconv(.C) void {
            assert(sig == posix.SIG.INT);
            std.log.info("Received SIGINT", .{});
            if (Self.shutting_down) |shutdown| {
                std.log.info("Signaled exit", .{});
                shutdown.* = true;
            }

            if (Self.original_handler) |orig_handler| {
                orig_handler(sig, info, o);
            }
        }
    };

    fn update_timestamp(self: *Server) !void {
        var ts: posix.timespec = undefined;
        try posix.clock_gettime(posix.CLOCK.REALTIME, &ts);
        const dt = datetime.DateTime.init(&ts);
        // Date: <day-name>, <day> <month> <year> <hour>:<minute>:<second> GMT
        // Date: Wed, 21 Oct 2015 07:28:00 GMT

        var buffer: *[64]u8 = undefined;
        if (self.current_time_str_ref) |current_time_str_ref| {
            if (current_time_str_ref.ptr == &self.current_time_str_buffer_1) {
                buffer = &self.current_time_str_buffer_2;
            } else if (current_time_str_ref.ptr == &self.current_time_str_buffer_2) {
                buffer = &self.current_time_str_buffer_3;
            } else if (current_time_str_ref.ptr == &self.current_time_str_buffer_3) {
                buffer = &self.current_time_str_buffer_4;
            } else {
                buffer = &self.current_time_str_buffer_1;
            }
        } else {
            buffer = &self.current_time_str_buffer_1;
        }

        self.current_time_str = try std.fmt.bufPrint(
            buffer[0..],
            "Date: {s}, {d:0>2} {s} {} {d:0>2}:{d:0>2}:{d:0>2} GMT\r\n",
            .{
                dt.day_name[0..3],
                dt.day,
                dt.month_name[0..3],
                dt.year,
                dt.hour,
                dt.minute,
                dt.second,
            },
        );
        self.current_time_str_ref = &self.current_time_str;
    }
};

const ServerThreadAllocator = std.heap.GeneralPurposeAllocator(.{
    .thread_safe = false,
});

const ServerThread = struct {
    // Parameters for the ring provided buffers for IO uring
    const buffer_count = 1024;
    const buffer_size = 1024 * 4;

    thread: ?std.Thread align(std.atomic.cache_line),
    thread_name: []const u8,
    host: []const u8,
    port: u16,
    listener_fd: posix.socket_t,
    server: *Server,

    root_allocator: *const std.mem.Allocator,
    thread_allocator: ServerThreadAllocator,
    allocator: std.mem.Allocator,
    slabs: alloc.SlabAllocator,

    ring: linux.IoUring,
    ring_fd_offset: u32,
    buffers_recv: BufferGroupRecv,

    comptime {
        assert(@alignOf(@This()) == std.atomic.cache_line);
    }

    fn init(cpu: usize, server: *Server) !*ServerThread {
        const root_allocator = server.allocator;
        var self = try root_allocator.create(ServerThread);
        errdefer root_allocator.destroy(self);

        self.* = ServerThread{
            .thread = undefined,
            .thread_name = undefined,
            .host = server.host,
            .port = server.port,
            .listener_fd = undefined,
            .server = server,

            .root_allocator = root_allocator,
            .thread_allocator = ServerThreadAllocator{},
            .allocator = undefined,
            .slabs = undefined,

            .ring = undefined,
            .ring_fd_offset = undefined,
            .buffers_recv = undefined,
        };
        errdefer {
            const alloc_check = self.thread_allocator.deinit();
            assert(alloc_check == .ok);
        }
        self.allocator = self.thread_allocator.allocator();

        self.slabs = try alloc.SlabAllocator.init(&self.allocator, buffer_size, buffer_count);
        errdefer self.slabs.deinit();

        const thread_config = std.Thread.SpawnConfig{
            .allocator = self.allocator,
        };
        self.thread = try std.Thread.spawn(thread_config, run, .{self});
        const thread_name = try std.fmt.allocPrint(self.allocator, "zserv-{}", .{cpu});
        errdefer self.allocator.free(thread_name);
        try self.thread.?.setName(thread_name);
        self.thread_name = thread_name;

        return self;
    }

    fn deinit(self: *ServerThread) void {
        self.allocator.free(self.thread_name);
        self.allocator.free(self.buffers_recv.buffers);
        self.ring.deinit();
        self.slabs.deinit();
        const alloc_check = self.thread_allocator.deinit();
        assert(alloc_check == .ok);
        const root_allocator = self.root_allocator.*;
        self.* = undefined;
        root_allocator.destroy(self);
    }

    fn run(self: *ServerThread) !void {
        std.log.debug("[{s}] Waiting for startup signal", .{self.thread_name});

        self.server.start_signal.wait();

        std.log.debug("[{s}] Starting", .{self.thread_name});

        self.listener_fd = try self.setup_listener_socket();
        std.log.info("[{s}] Setup server socket - addr={s} {} fd={}", .{ self.thread_name, self.host, self.port, self.listener_fd });

        try self.init_ring();
        std.log.info("[{s}] Initialized IOUring", .{self.thread_name});

        self.buffers_recv = try self.alloc_buffer_group(recv_bgid);
        std.log.info("[{s}] Setup provided buffer rings", .{self.thread_name});

        const op_accept: Op = .{ .type = .accept, .fd = self.listener_fd, .buffer_id = undefined, ._padding = undefined };
        _ = try self.ring.accept_multishot(op_accept.asU64(), self.listener_fd, null, null, 0);
        std.log.info("[{s}] Submitted multishot accept", .{self.thread_name});

        const wait = linux.kernel_timespec{
            .tv_sec = 1,
            .tv_nsec = 0,
        };

        var cqes: [256]linux.io_uring_cqe = undefined;
        loop: while (!self.server.shutting_down) {
            // std.log.debug("Waiting...", .{});
            // _ = self.ring.submit_and_wait(1) catch |err| switch (err) {
            //     error.SignalInterrupt => break :loop,
            //     error.RingShuttingDown => break :loop,
            //     else => unreachableWith("Unhandled error from IO Uring enter: {}", .{err}),
            // };
            const submitted = self.ring.flush_sq();
            // const res = linux.io_uring_enter(
            //     @intCast(self.ring_fd_offset),
            //     submitted,
            //     1,
            //     linux.IORING_ENTER_GETEVENTS | linux.IORING_ENTER_REGISTERED_RING,
            //     null,
            // );
            const res = linuxext.io_uring_submit_and_wait_timeout(
                @intCast(self.ring_fd_offset),
                submitted,
                1,
                &wait,
                null,
            );

            switch (linux.E.init(res)) {
                .SUCCESS => {},
                .INTR => break :loop,
                .NXIO => break :loop,
                else => |err| unreachableWith("Unhandled error from IO Uring enter: {}", .{err}),
            }
            assert(self.ring.cq.overflow.* == 0);
            assert((self.ring.sq.flags.* & linux.IORING_SQ_CQ_OVERFLOW) == 0);

            // std.log.info("Event loop submitted {}", .{ret});

            const completed = try self.ring.copy_cqes(&cqes, 0);

            for (cqes[0..completed]) |cqe| {
                const op = Op.read(cqe.user_data);
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
                            .buffer_id = undefined,
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
                        assert(op.fd >= 0);
                        if (cqe.res == 0 or cqe.err() == linux.E.CONNRESET) {
                            // std.log.info("[{}] recv: len=0", .{op.fd});
                            const op_close: Op = .{
                                .type = .close,
                                .fd = op.fd,
                                .buffer_id = undefined,
                                ._padding = undefined,
                            };
                            _ = try self.ring.close(op_close.asU64(), op.fd);
                        } else {
                            assertCqe(&cqe);
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
                            self.write_response(&head, &send_buffer);

                            const op_send: Op = .{
                                .type = .send,
                                .fd = op.fd,
                                .buffer_id = send_buffer_id,
                                ._padding = undefined,
                            };
                            var sqe = try self.ring.get_sqe();
                            sqe.prep_rw(.SEND, op.fd, @intFromPtr(send_buffer.items.ptr), send_buffer.items.len, 0);
                            sqe.rw_flags = linux.MSG.WAITALL | linux.MSG.NOSIGNAL;
                            sqe.user_data = op_send.asU64();
                        }
                    },
                    .send => {
                        assertCqe(&cqe);
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
        }

        std.log.info("[{s}] Exiting thread...", .{self.thread_name});
    }

    fn parse_request(buffer: []const u8) !HttpServer.Request.Head {
        const head = try HttpServer.Request.Head.parse(buffer);
        // assert(head.version == std.http.Version.@"HTTP/1.1");
        return head;
    }

    fn write_response(
        self: *ServerThread,
        head: *const HttpServer.Request.Head,
        writer: *std.ArrayListAlignedUnmanaged(u8, std.mem.page_size),
    ) void {
        assert(writer.items.len == 0);
        if (head.method == std.http.Method.GET and std.mem.endsWith(u8, head.target, "/plaintext")) {
            writer.appendSliceAssumeCapacity("HTTP/1.1 200 OK\r\n");
            writer.appendSliceAssumeCapacity("Server: Z\r\n");
            const current_time = (self.server.current_time_str_ref orelse unreachable).*;
            writer.appendSliceAssumeCapacity(current_time);
            assert(std.mem.endsWith(u8, writer.items, "GMT\r\n"));
            writer.appendSliceAssumeCapacity("Content-Length: 13\r\n");
            writer.appendSliceAssumeCapacity("Content-Type: text/plain\r\n");
            writer.appendSliceAssumeCapacity("\r\n");
            writer.appendSliceAssumeCapacity("Hello, World!");
        } else {
            writer.appendSliceAssumeCapacity("HTTP/1.1 404 Not Found\r\n");
            writer.appendSliceAssumeCapacity("Server: Z\r\n");
            const current_time = (self.server.current_time_str_ref orelse unreachable).*;
            writer.appendSliceAssumeCapacity(current_time);
            assert(std.mem.endsWith(u8, writer.items, "GMT\r\n"));
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

    fn setup_listener_socket(self: *ServerThread) !posix.socket_t {
        const fd = try posix.socket(posix.AF.INET, posix.SOCK.STREAM, 0);

        try posix.setsockopt(fd, posix.SOL.SOCKET, posix.SO.REUSEPORT, &std.mem.toBytes(@as(c_int, 1)));

        const address = try std.net.Address.parseIp4(self.host, self.port);
        try posix.bind(fd, &address.any, address.getOsSockLen());

        try posix.listen(fd, 128);
        return fd;
    }

    fn init_ring(self: *ServerThread) !void {
        var params = std.mem.zeroes(linux.io_uring_params);
        params.flags |= linux.IORING_SETUP_SINGLE_ISSUER;
        params.flags |= linux.IORING_SETUP_CLAMP;
        params.flags |= linux.IORING_SETUP_CQSIZE;
        params.flags |= linux.IORING_SETUP_DEFER_TASKRUN;
        params.cq_entries = queue_size;

        self.ring = try linux.IoUring.init_params(queue_size, &params);
        assert(params.features & linuxext.IORING_FEAT_SEND_BUF_SELECT == linuxext.IORING_FEAT_SEND_BUF_SELECT);

        try linuxext.io_uring_register_files_sparse(&self.ring, 4);
        self.ring_fd_offset = try linuxext.io_uring_register_ring_fd(&self.ring);
    }

    fn alloc_buffer_group(self: *ServerThread, bgid: u16) !BufferGroupRecv {
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
