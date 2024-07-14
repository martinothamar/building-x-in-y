const std = @import("std");
const linux = std.os.linux;

pub const BufferGroupSend = struct {
    /// Parent ring for which this group is registered.
    ring: *linux.IoUring,
    /// Pointer to the memory shared by the kernel.
    /// `buffers_count` of `io_uring_buf` structures are shared by the kernel.
    /// First `io_uring_buf` is overlaid by `io_uring_buf_ring` struct.
    br: *align(std.mem.page_size) linux.io_uring_buf_ring,
    // Number of buffers in `buffers`, number of `io_uring_buf structures` in br.
    buffers_count: u16,
    /// ID of this group, must be unique in ring.
    group_id: u16,

    buffers_added: u16,

    pub fn init(
        ring: *linux.IoUring,
        group_id: u16,
        buffers_count: u16,
    ) !BufferGroupSend {
        const br = try linux.IoUring.setup_buf_ring(ring.fd, buffers_count, group_id);
        linux.IoUring.buf_ring_init(br);

        return BufferGroupSend{
            .ring = ring,
            .group_id = group_id,
            .br = br,
            .buffers_count = buffers_count,

            .buffers_added = 0,
        };
    }

    pub fn put(self: *BufferGroupSend, buffer_id: u16, buffer: []u8) void {
        const mask = linux.IoUring.buf_ring_mask(self.buffers_count);
        linux.IoUring.buf_ring_add(self.br, buffer, buffer_id, mask, self.buffers_added);
        self.buffers_added += 1;
    }

    pub fn maybeCommitBuffers(self: *BufferGroupSend) void {
        if (self.buffers_added > 0) {
            linux.IoUring.buf_ring_advance(self.br, self.buffers_added);
            self.buffers_added = 0;
        }
    }

    pub fn deinit(self: *BufferGroupSend) void {
        linux.IoUring.free_buf_ring(self.ring.fd, self.br, self.buffers_count, self.group_id);
    }
};
