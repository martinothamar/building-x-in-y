#define _POSIX_C_SOURCE 199309L
#include <stdio.h>
#include <stdalign.h>
#include <stdatomic.h>
#include <stdint.h>
#include <stddef.h>
#include <string.h>
#include <pthread.h>
#include <stdbool.h>
#include <time.h>

// Must be power of 2
#define SPMC_QUEUE_SIZE 64

#define NUM_MSGS 64

// Get's the next and the previous index in the queue ringbuffer
#define NEXT_IDX(index) (((index) + 1) & (SPMC_QUEUE_SIZE - 1))
#define PREV_IDX(index) (((index) - 1) & (SPMC_QUEUE_SIZE - 1))

#define likely(x)    __builtin_expect (!!(x), 1)
#define unlikely(x)  __builtin_expect (!!(x), 0)

// Represents a single element in the queue
typedef struct {
    uint64_t version;
    uint64_t data;
} block;

// The queue itself
typedef struct {
    alignas(64) uint64_t index;
    alignas(64) block data[SPMC_QUEUE_SIZE];
} spmc;

// The reader of the queue, _not_ threadsafe
typedef struct {
    uint64_t index;
    uint64_t wraps;
    spmc * q;
} spmc_reader;

void push(spmc *q, uint64_t v) {
    uint64_t index = q->index;

    uint64_t seq0 = atomic_load_explicit(&q->data[index].version, memory_order_relaxed);
    atomic_store_explicit(&q->data[index].version, seq0 + 1, memory_order_release);
    atomic_signal_fence(memory_order_acq_rel);

    q->data[index].data = v;

    atomic_signal_fence(memory_order_acq_rel);
    atomic_store_explicit(&q->data[index].version, seq0 + 2, memory_order_release);

    q->index = NEXT_IDX(index);
}

bool pop(spmc_reader *qr, uint64_t *value) {
    uint64_t index = qr->index;

    uint64_t seq0, seq1;

    seq0 = atomic_load_explicit(&qr->q->data[index].version, memory_order_acquire);
    atomic_signal_fence(memory_order_acq_rel);

    *value = qr->q->data[index].data;

    atomic_signal_fence(memory_order_acq_rel);
    seq1 = atomic_load_explicit(&qr->q->data[index].version, memory_order_acquire);

    if (likely(seq0 == ((qr->wraps + 1) * 2) && seq0 == seq1)) {
        uint64_t new_index = NEXT_IDX(index);
        qr->index = new_index;
        if (new_index < index) {
            qr->wraps++;
        }
        // printf("Got element: %lu, seq0: %lu, seq1: %lu\n", *value, seq0, seq1);
        return true;
    } else {
        return false;
    }
}

typedef struct {
    int thread;
    spmc *q;
} reader_args;

__thread size_t log_index = 0;
__thread char log_data[NUM_MSGS * 2][64];
char * log_push() {
    return (char *)&log_data[log_index++];
}

void printq(spmc *q) {
    uint64_t index = PREV_IDX(q->index);
    char *log = log_push();
    sprintf(log, "Q - index: %lu, data: %lu\n", index, q->data[index].data);
}

void printqr(spmc_reader *qr, uint64_t r) {
    uint64_t index = PREV_IDX(qr->index);
    char *log = log_push();
    sprintf(log, "QR - index: %lu, data: %lu\n", index, r);
}

void *reader_thread(void *args)
{
    reader_args* ra = (reader_args*)args;
    spmc_reader spmcr = { .index = 0, .wraps = 0, .q = ra->q };

    uint64_t messages_read = 0;
    while (messages_read < NUM_MSGS) {
        uint64_t r;
        uint64_t success = pop(&spmcr, &r);
        if (!success) {
            printf("Yielding thread\n");
            sched_yield();
            continue;
        }
        messages_read++;
        printqr(&spmcr, r);
    }

    for (size_t i = 0; i < log_index; i++) {
        printf("%s", log_data[i]);
    }

    printf("Exit reader thread\n");
    return NULL;
}

void ussleep(unsigned long us)
{
    struct timespec ts;
    ts.tv_sec = 0;
    ts.tv_nsec = us * 1000;
    nanosleep(&ts, NULL);
}

int main() {
    spmc spmc = { .index = 0, .data = {{ 0 }} };

    const int num_threads = 1;
    pthread_t threads[num_threads];
    reader_args thread_args[num_threads];

    for (int t = 0; t < num_threads; t++) {
        thread_args[t] = (reader_args) { .thread = t, .q = &spmc };
        pthread_create(&threads[t], NULL, reader_thread, (void*)&thread_args[t]);
    }

    ussleep(1);

    for (int i = 1; i <= NUM_MSGS; i++) {
        push(&spmc, i);
        printq(&spmc);
    }

    for (size_t i = 0; i < log_index; i++) {
        printf("%s", log_data[i]);
    }

    for (int t = 0; t < num_threads; t++) {
        pthread_join(threads[t], NULL);
    }

    return 0;
}

