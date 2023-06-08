#include <stdio.h>
#include <stdalign.h>
#include <stdatomic.h>
#include <stdint.h>
#include <stddef.h>
#include <string.h>
#include <pthread.h>

// Must be power of 2
#define SPMC_QUEUE_SIZE 64

// Get's the next and the previous index in the queue ringbuffer
#define NEXT_IDX(index) (((index) + 1) & (SPMC_QUEUE_SIZE - 1))
#define PREV_IDX(index) (((index) - 1) & (SPMC_QUEUE_SIZE - 1))

typedef struct {
    uint64_t version;
    uint64_t data;
} block;

typedef struct {
    alignas(64) uint64_t index;
    alignas(64) block data[SPMC_QUEUE_SIZE];
} spmc;

typedef struct {
    uint64_t index;
    uint64_t wraps;
    spmc * q;
} spmc_reader;

void push(spmc *q, uint64_t v) {
    uint64_t index = atomic_load_explicit(&q->index, memory_order_relaxed);

    atomic_fetch_add_explicit(&q->data[index].version, 1, memory_order_relaxed);

    q->data[index].data = v;

    atomic_fetch_add_explicit(&q->data[index].version, 1, memory_order_relaxed);

    uint64_t new_index = NEXT_IDX(index);
    atomic_store_explicit(&q->index, new_index, memory_order_relaxed);
}

uint64_t pop(spmc_reader *qr) {
    uint64_t index = qr->index;

    uint64_t version = atomic_load_explicit(&qr->q->data[index].version, memory_order_acquire);

    if (version == qr->wraps * 2 || (version & 1) != 0)
        return 0;

    uint64_t value;
    memcpy(&value, &qr->q->data[index].data, sizeof(value));

    uint64_t new_index = NEXT_IDX(index);
    qr->index = new_index;
    if (new_index < index) {
        qr->wraps++;
    }

    return version == atomic_load_explicit(&qr->q->data[index].version, memory_order_acquire) ? value : 0;
}

typedef struct {
    int thread;
    spmc *q;
} reader_args;

void printq(spmc *q) {
    uint64_t index = PREV_IDX(q->index);
    printf("Q - index: %lu, data: %lu\n", index, q->data[index].data);
}

void printqr(spmc_reader *qr, uint64_t r) {
    uint64_t index = PREV_IDX(qr->index);
    printf("QR - index: %lu, data: %lu, r: %lu\n", index, qr->q->data[index].data, r);
}

const int NUM_MSGS = 64;

void *reader_thread(void *args)
{
    reader_args* ra = (reader_args*)args;
    spmc_reader spmcr = { .index = 0, .wraps = 0, .q = ra->q };

    uint64_t messages_read = 0;
    while (messages_read < NUM_MSGS) {
        uint64_t r = pop(&spmcr);
        if (r == 0) {
            sched_yield();
            continue;
        }
        messages_read++;
        printqr(&spmcr, r);
    }

    printf("Exit reader thread\n");
    return NULL;
}

int main() {
    spmc spmc = { .index = 0 };

    const int num_threads = 1;
    pthread_t threads[num_threads];
    reader_args thread_args[num_threads];

    for (int t = 0; t < num_threads; t++) {
        thread_args[t] = (reader_args) { .thread = t, .q = &spmc };
        pthread_create(&threads[t], NULL, reader_thread, (void*)&thread_args[t]);
    }

    for (int i = 0; i < NUM_MSGS; i++) {
        push(&spmc, i);
        printq(&spmc);
    }

    for (int t = 0; t < num_threads; t++) {
        pthread_join(threads[t], NULL);
    }

    return 0;
}

