const std = @import("std");
const assert = std.debug.assert;

pub const SlabAllocator = struct {
    const sentinel: u16 = std.math.maxInt(u16);

    entries: std.ArrayList(InternalEntry),
    allocator: *const std.mem.Allocator,
    slab_size: usize,
    len: u16,
    next: u16,

    pub fn init(allocator: *const std.mem.Allocator, slab_size: usize, initial_capacity: u16) !SlabAllocator {
        assert(initial_capacity < sentinel - 1);
        const entries = try std.ArrayList(InternalEntry).initCapacity(allocator.*, @intCast(initial_capacity));

        var self = SlabAllocator{
            .entries = entries,
            .allocator = allocator,
            .slab_size = slab_size,
            .len = 0,
            .next = 0,
        };

        var i: u16 = 0;
        while (i < initial_capacity) : (i += 1) {
            _ = try self.alloc();
        }
        i = 0;

        while (i < initial_capacity) : (i += 1) {
            self.dealloc(i);
        }

        return self;
    }

    pub fn deinit(self: *SlabAllocator) void {
        for (self.entries.items) |entry| {
            assert(!entry.taken);
            self.allocator.free(entry.buffer);
        }

        self.entries.deinit();
        self.* = undefined;
    }

    pub fn alloc(self: *SlabAllocator) !Entry {
        assert(self.len < sentinel - 1);
        const id = self.next;
        self.len += 1;
        const idIndex = @as(usize, id);

        if (idIndex == self.entries.items.len) {
            const buffer = try self.allocator.alignedAlloc(u8, std.mem.page_size, self.slab_size);
            @memset(buffer, 0);
            const entry = InternalEntry{
                .buffer = buffer,
                .next = sentinel,
                .taken = true,
            };
            try self.entries.append(entry);
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

    pub fn get(self: *SlabAllocator, id: u16) Entry {
        const idIndex = @as(usize, id);
        assert(idIndex < self.entries.items.len);
        const entry = &self.entries.items[idIndex];
        assert(entry.taken == true);
        return Entry{
            .buffer = entry.buffer,
            .id = id,
        };
    }

    pub fn dealloc(self: *SlabAllocator, id: u16) void {
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

    pub const Entry = struct {
        buffer: []align(std.mem.page_size) u8,
        id: u16,
    };

    const InternalEntry = struct {
        buffer: []align(std.mem.page_size) u8,
        next: u16,
        taken: bool,
    };
};
