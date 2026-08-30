// Challenger Async Runtime — Performance Benchmarks
// Phase 35: Scheduler, Timer, TCP, Channel, Memory benchmarks

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifdef _WIN32
#pragma comment(lib, "ws2_32.lib")
#include <winsock2.h>
#include <ws2tcpip.h>
#include <windows.h>
#else
#include <unistd.h>
#include <pthread.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <arpa/inet.h>
#include <time.h>
#endif

#include "runtime.h"

// ============================================================
// Benchmark helpers
// ============================================================

static double g_freq_mhz = 0;

static void bench_init(void) {
    #ifdef _WIN32
    LARGE_INTEGER freq;
    QueryPerformanceFrequency(&freq);
    g_freq_mhz = (double)freq.QuadPart / 1000000.0;
    #else
    g_freq_mhz = 2400.0; // assume ~2.4GHz
    #endif
}

static int64_t now_us(void) {
    return challenger_time_now_us();
}

#define BENCH_START(name, iterations) \
    do { \
        printf("  %-45s ", name); \
        int64_t _bench_start = now_us(); \
        int64_t _bench_iters = iterations;

#define BENCH_END() \
        int64_t _bench_end = now_us(); \
        double _bench_us = (double)(_bench_end - _bench_start); \
        double _bench_per = _bench_us / (double)_bench_iters; \
        printf("%10.1f us total  %8.2f ns/op\n", _bench_us, _bench_per * 1000.0); \
    } while(0)

// ============================================================
// Benchmark 1: Timer creation throughput
// ============================================================

static Poll bench_pending_fn(ChallengerFuture* f, ChallengerWaker* w) {
    (void)f; (void)w;
    return challenger_poll_pending();
}

static void bench_timer_creation(void) {
    int N = 10000;
    BENCH_START("Timer creation (10K)", N);
    for (int i = 0; i < N; i++) {
        ChallengerExecutor* exec = challenger_executor_new();
        ChallengerTimerWheel tw;
        challenger_timer_init(&tw);
        challenger_set_global_executor(exec);
        exec->current_task_id = 1;
        uint64_t id = challenger_timer_sleep(exec, &tw, 1000000);
        (void)id;
        challenger_set_global_executor(NULL);
        challenger_executor_free(exec);
    }
    BENCH_END();
}

// ============================================================
// Benchmark 2: Task spawn + immediate completion
// ============================================================

static Poll bench_ready_fn(ChallengerFuture* f, ChallengerWaker* w) {
    (void)f; (void)w;
    return challenger_poll_ready(1);
}

static void bench_task_spawn_completion(void) {
    int N = 10000;
    BENCH_START("Task spawn+complete (10K)", N);
    for (int i = 0; i < N; i++) {
        ChallengerExecutor* exec = challenger_executor_new();
        ChallengerFuture* fut = challenger_future_new(bench_ready_fn, NULL);
        challenger_executor_spawn(exec, fut);
        challenger_executor_run(exec);
        challenger_executor_free(exec);
    }
    BENCH_END();
}

// ============================================================
// Benchmark 3: Executor throughput (batch)
// ============================================================

static void bench_executor_batch(void) {
    int N = 10000;
    ChallengerExecutor* exec = challenger_executor_new();
    for (int i = 0; i < N; i++) {
        ChallengerFuture* fut = challenger_future_new(bench_ready_fn, NULL);
        challenger_executor_spawn(exec, fut);
    }
    BENCH_START("Executor batch 10K tasks", N);
    challenger_executor_run(exec);
    BENCH_END();
    challenger_executor_free(exec);
}

// ============================================================
// Benchmark 4: Channel throughput
// ============================================================

static void bench_channel_throughput(void) {
    int N = 100000;
    ChallengerChannel* ch = challenger_channel_new(0);
    BENCH_START("Channel send+recv (100K)", N);
    for (int i = 0; i < N; i++) {
        challenger_channel_send(NULL, ch, i);
        int64_t out = 0;
        challenger_channel_receive(NULL, ch, &out);
    }
    BENCH_END();
    challenger_channel_free(ch);
}

// ============================================================
// Benchmark 5: Mutex lock/unlock throughput
// ============================================================

static void bench_mutex_throughput(void) {
    int N = 100000;
    ChallengerMutex* m = challenger_mutex_new();
    BENCH_START("Mutex lock+unlock (100K)", N);
    for (int i = 0; i < N; i++) {
        challenger_mutex_try_lock(m, 1);
        challenger_mutex_unlock(NULL, m, 1);
    }
    BENCH_END();
    challenger_mutex_free(m);
}

// ============================================================
// Benchmark 6: Waker creation + wake
// ============================================================

static void bench_waker_noop(void* data) { (void)data; }

static void bench_waker_throughput(void) {
    int N = 100000;
    BENCH_START("Waker create+wake (100K)", N);
    for (int i = 0; i < N; i++) {
        ChallengerWaker* w = challenger_waker_new(bench_waker_noop, NULL);
        challenger_waker_wake(w);
        challenger_waker_free(w);
    }
    BENCH_END();
}

// ============================================================
// Benchmark 7: Future poll throughput
// ============================================================

static void bench_future_poll(void) {
    int N = 100000;
    ChallengerFuture* fut = challenger_future_new(bench_ready_fn, NULL);
    ChallengerWaker* w = challenger_waker_new(NULL, NULL);
    BENCH_START("Future poll (100K)", N);
    for (int i = 0; i < N; i++) {
        // Reset completed state for repeated polling
        fut->completed = 0;
        fut->output = 0;
        challenger_future_poll(fut, w);
    }
    BENCH_END();
    challenger_waker_free(w);
    challenger_future_free(fut);
}

// ============================================================
// Benchmark 8: TCP echo throughput (single client)
// ============================================================

#define BENCH_TCP_PORT 19900

static void bench_tcp_echo(void) {
    int server_fd = challenger_tcp_socket();
    challenger_tcp_bind(server_fd, "127.0.0.1", BENCH_TCP_PORT);
    challenger_tcp_listen(server_fd, 1);

    int client_fd = challenger_tcp_socket();
    challenger_tcp_connect(client_fd, "127.0.0.1", BENCH_TCP_PORT);

    int accepted_fd = challenger_tcp_accept(server_fd);

    int N = 1000;
    char msg[64] = "benchmark-ping";
    char buf[256];
    BENCH_START("TCP echo 1K round-trips", N);
    for (int i = 0; i < N; i++) {
        challenger_tcp_write(client_fd, msg, (int)strlen(msg));
        challenger_tcp_read(accepted_fd, buf, sizeof(buf));
    }
    BENCH_END();

    challenger_tcp_close(client_fd);
    challenger_tcp_close(accepted_fd);
    challenger_tcp_close(server_fd);
}

// ============================================================
// Benchmark 9: Select combinator throughput
// ============================================================

static void bench_select_throughput(void) {
    int N = 10000;
    ChallengerFuture* futures[3];
    ChallengerWaker* w = challenger_waker_new(NULL, NULL);

    BENCH_START("Select over 3 futures (10K)", N);
    for (int i = 0; i < N; i++) {
        ChallengerFuture* f0 = challenger_future_new(bench_ready_fn, NULL);
        ChallengerFuture* f1 = challenger_future_new(bench_ready_fn, NULL);
        ChallengerFuture* f2 = challenger_future_new(bench_ready_fn, NULL);
        futures[0] = f0; futures[1] = f1; futures[2] = f2;
        int64_t val = 0;
        challenger_select_poll(futures, 3, w, &val);
        challenger_future_free(f0);
        challenger_future_free(f1);
        challenger_future_free(f2);
    }
    BENCH_END();
    challenger_waker_free(w);
}

// ============================================================
// Benchmark 10: Memory allocation throughput
// ============================================================

static void bench_alloc_throughput(void) {
    int N = 50000;
    BENCH_START("Executor alloc/free (50K)", N);
    for (int i = 0; i < N; i++) {
        ChallengerExecutor* exec = challenger_executor_new();
        challenger_executor_free(exec);
    }
    BENCH_END();
}

// ============================================================
// Main
// ============================================================

int main(void) {
    setvbuf(stdout, NULL, _IONBF, 0);
    setvbuf(stderr, NULL, _IONBF, 0);

    #ifdef _WIN32
    WSADATA wsa;
    WSAStartup(MAKEWORD(2, 2), &wsa);
    #endif

    bench_init();

    printf("=== Challenger Performance Benchmarks ===\n\n");

    printf("--- Scheduler ---\n");
    bench_task_spawn_completion();
    bench_executor_batch();
    bench_future_poll();
    bench_waker_throughput();
    bench_select_throughput();

    printf("\n--- Timer ---\n");
    bench_timer_creation();

    printf("\n--- Channel ---\n");
    bench_channel_throughput();

    printf("\n--- Mutex ---\n");
    bench_mutex_throughput();

    printf("\n--- TCP ---\n");
    bench_tcp_echo();

    printf("\n--- Memory ---\n");
    bench_alloc_throughput();

    printf("\n=== Benchmarks Complete ===\n");

    #ifdef _WIN32
    WSACleanup();
    #endif
    return 0;
}
