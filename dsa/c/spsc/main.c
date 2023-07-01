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
#include <assert.h>

// Must be power of 2
#define SPSC_QUEUE_SIZE 1024

#define NUM_MSGS 64 * 64

// Get's the next and the previous index in the queue ringbuffer
#define NEXT_IDX(index) (((index) + 1) & (SPSC_QUEUE_SIZE - 1))
#define PREV_IDX(index) (((index) - 1) & (SPSC_QUEUE_SIZE - 1))

#define likely(x)    __builtin_expect (!!(x), 1)
#define unlikely(x)  __builtin_expect (!!(x), 0)

// Represents a single element in the queue
typedef struct {
    uint64_t version;
    uint64_t data;
} block;

typedef struct {
    uint64_t index;
    uint64_t wraps;
} spsc_reader;

typedef struct {
    alignas(64) uint64_t write_index;
    alignas(64) spsc_reader reader;
    alignas(64) block data[SPSC_QUEUE_SIZE];
} spsc;

void push(spsc *q, uint64_t v) {
    uint64_t index = q->write_index;

    uint64_t seq0 = atomic_load_explicit(&q->data[index].version, memory_order_relaxed);

    q->data[index].data = v;

    atomic_store_explicit(&q->data[index].version, seq0 + 2, memory_order_release);

    q->write_index = NEXT_IDX(index);
}

bool pop(spsc *q, uint64_t *v) {
    uint64_t index = q->reader.index;

    uint64_t seq0, seq1;

    seq0 = atomic_load_explicit(&q->data[index].version, memory_order_acquire);

    *v = q->data[index].data;

    seq1 = atomic_load_explicit(&q->data[index].version, memory_order_acquire);

    if (unlikely(seq0 != ((q->reader.wraps + 1) * 2) || seq0 != seq1)) {
        return false;
    }

    uint64_t new_index = NEXT_IDX(index);
    q->reader.index = new_index;
    if (new_index < index) {
        q->reader.wraps++;
    }
    // printf("Got element: %lu, seq0: %lu, seq1: %lu\n", *value, seq0, seq1);
    return true;
}

typedef struct {
    int thread;
    spsc *q;
} reader_args;

typedef struct {
    uint64_t index;
    uint64_t data;
    uint64_t latency;
} log_message;

__thread size_t log_index = 0;
__thread log_message log_data[NUM_MSGS];
log_message * log_push() {
    return (log_message *)&log_data[log_index++];
}

uint64_t start;
uint64_t end;

void log_writer(spsc *q, uint64_t v) {
    uint64_t write_index = PREV_IDX(q->write_index);
    log_message *log = log_push();
    log->index = write_index;
    log->data = v;
    log->latency = 0;
}

void log_reader(spsc *q, uint64_t i, uint64_t l) {
    uint64_t read_index = PREV_IDX(q->reader.index);
    log_message *log = log_push();
    log->index = read_index;
    log->data = i;
    log->latency = l;
}

uint64_t current_nanosec() {
    struct timespec t;
    int r = clock_gettime(CLOCK_MONOTONIC, &t);
    assert(r == 0);
    return (uint64_t)t.tv_sec * (uint64_t)1000000000 + (uint64_t)t.tv_nsec;
}

void ussleep(unsigned long us)
{
    struct timespec ts;
    ts.tv_sec = 0;
    ts.tv_nsec = us * 1000;
    nanosleep(&ts, NULL);
}

void *reader_thread(void *args)
{
    reader_args* ra = (reader_args*)args;
    spsc *q = ra->q;

    uint64_t messages_read = 0;
    uint64_t v;
    while (messages_read < NUM_MSGS) {
        uint64_t success = pop(q, &v);
        if (!success) {
            // printf("Yielding thread\n");
            // sched_yield();
            continue;
        }
        uint64_t l = current_nanosec() - v;
        messages_read++;
        log_reader(q, messages_read, l);
    }

    end = current_nanosec();

    ussleep(10000);

    for (size_t i = 0; i < log_index; i++) {
        log_message *log = &log_data[i];
        printf("Q - reader: %lu, data: %lu, latency: %lu\n", log->index, log->data, log->latency);
    }

    uint64_t nanoseconds = end - start;
    double msgs_per_second = (double)NUM_MSGS / ((double)nanoseconds / 1000000000.0);
    printf("%u messages in %lu nanoseconds, %f msgs/s\n", NUM_MSGS, nanoseconds, msgs_per_second);

    printf("Exit reader thread\n");
    return NULL;
}

int main() {
    spsc spsc = { .write_index = 0, .reader = {0}, .data = {{ 0 }} };

    const int num_threads = 1;
    pthread_t threads[num_threads];
    reader_args thread_args[num_threads];

    for (int t = 0; t < num_threads; t++) {
        thread_args[t] = (reader_args) { .thread = t, .q = &spsc };
        pthread_create(&threads[t], NULL, reader_thread, (void*)&thread_args[t]);
    }

    ussleep(1);

    start = current_nanosec();
    for (int i = 1; i <= NUM_MSGS; i++) {
        push(&spsc, current_nanosec());
        log_writer(&spsc, i);
    }

    ussleep(5);

    for (size_t i = 0; i < log_index; i++) {
        log_message *log = &log_data[i];
        printf("Q - writer: %lu, data: %lu\n", log->index, log->data);
    }

    for (int t = 0; t < num_threads; t++) {
        pthread_join(threads[t], NULL);
    }

    return 0;
}

