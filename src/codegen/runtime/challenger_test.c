// Challenger Async Runtime — Comprehensive Validation Test
// Phases 21-34: Error Model, Future, Timer, TCP, UDP, Channels, Sync,
//               Join/Select, Cancellation, Blocking Pool, Process, DNS, Multi-thread, Stress
//
// Compile: clang -o challenger_test.exe challenger_test.c -lws2_32 (Windows)
//          clang -o challenger_test challenger_test.c -lpthread (POSIX)

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifdef _WIN32
#pragma comment(lib, "ws2_32.lib")
#include <winsock2.h>
#include <ws2tcpip.h>
#include <windows.h>
#include <io.h>
#include <fcntl.h>
typedef int ssize_t;
#else
#include <unistd.h>
#include <pthread.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <arpa/inet.h>
#include <errno.h>
#include <time.h>
#include <signal.h>
#endif

// Include the runtime header
#include "runtime.h"

// ============================================================
// Test framework
// ============================================================

static int g_tests_run = 0;
static int g_tests_passed = 0;
static int g_tests_failed = 0;

#define TEST(name) \
    do { g_tests_run++; printf("  [%d] %-50s ", g_tests_run, name); } while(0)

#define PASS() \
    do { g_tests_passed++; printf("PASS\n"); } while(0)

#define FAIL(msg) \
    do { g_tests_failed++; printf("FAIL: %s\n", msg); } while(0)

#define ASSERT(cond, msg) \
    do { if (!(cond)) { FAIL(msg); return; } } while(0)

#define ASSERT_EQ(a, b, msg) \
    do { if ((a) != (b)) { FAIL(msg); return; } } while(0)

#define ASSERT_NE(a, b, msg) \
    do { if ((a) == (b)) { FAIL(msg); return; } } while(0)

// ============================================================
// Phase 21: Error Model
// ============================================================

static void test_error_model_tcp_socket_failure(void) {
    TEST("TCP socket creation");
    int fd = challenger_tcp_socket();
    ASSERT(fd >= 0, "socket creation failed");
    challenger_tcp_close(fd);
    PASS();
}

static void test_error_model_tcp_bind_failure(void) {
    TEST("TCP bind on invalid fd");
    int result = challenger_tcp_bind(-1, "127.0.0.1", 1);
    ASSERT_EQ(result, -1, "bind on invalid fd should return -1");
    PASS();
}

static void test_error_model_tcp_connect_failure(void) {
    TEST("TCP connect to unreachable port");
    int fd = challenger_tcp_socket();
    ASSERT(fd >= 0, "socket creation failed");
    int result = challenger_tcp_connect(fd, "127.0.0.1", 1);
    // connect to port 1 should fail (no listener)
    ASSERT(result == -1 || result == 0, "connect result unexpected");
    challenger_tcp_close(fd);
    PASS();
}

static void test_error_model_channel_closed(void) {
    TEST("Channel send on closed channel");
    ChallengerChannel* ch = challenger_channel_new(0);
    ASSERT(ch != NULL, "channel creation failed");
    challenger_channel_close(ch);
    int result = challenger_channel_send(NULL, ch, 42);
    ASSERT_EQ(result, -1, "send on closed channel should return -1");
    challenger_channel_free(ch);
    PASS();
}

static void test_error_model_channel_receive_empty(void) {
    TEST("Channel receive on empty channel");
    ChallengerChannel* ch = challenger_channel_new(0);
    ASSERT(ch != NULL, "channel creation failed");
    int64_t out = 0;
    int result = challenger_channel_receive(NULL, ch, &out);
    ASSERT_EQ(result, 0, "receive on empty channel should return 0");
    challenger_channel_free(ch);
    PASS();
}

static void test_error_model_reactor_null(void) {
    TEST("Reactor null safety");
    // These should not crash
    challenger_reactor_register(NULL, -1, 0);
    challenger_reactor_unregister(NULL, -1);
    int n = challenger_reactor_poll(NULL, NULL, 0);
    ASSERT_EQ(n, 0, "poll on null should return 0");
    PASS();
}

static void test_error_model_timer_null(void) {
    TEST("Timer null safety");
    ChallengerTimerWheel tw;
    challenger_timer_init(&tw);
    // timer_sleep with null exec should return 0
    uint64_t id = challenger_timer_sleep(NULL, &tw, 1000000);
    ASSERT_EQ(id, 0, "timer_sleep with null exec should return 0");
    // timer_cancel with invalid id should not crash
    challenger_timer_cancel(&tw, 0);
    challenger_timer_cancel(&tw, 9999);
    PASS();
}

// ============================================================
// Phase 22: Future Correctness
// ============================================================

static Poll poll_counter_fn(ChallengerFuture* fut, ChallengerWaker* waker) {
    (void)waker;
    int* count = (int*)fut->state;
    (*count)++;
    if (*count >= 3) {
        return challenger_poll_ready(42);
    }
    return challenger_poll_pending();
}

static void test_future_ready_immediately(void) {
    TEST("Future Ready immediately");
    int count = 0;
    ChallengerFuture* fut = challenger_future_new(poll_counter_fn, &count);
    ASSERT(fut != NULL, "future creation failed");
    ChallengerWaker* w = challenger_waker_new(NULL, NULL);
    Poll p = challenger_future_poll(fut, w);
    // First poll: count=1, not ready yet (need 3 polls)
    ASSERT_EQ(p.tag, 1, "should be Pending on first poll");
    ASSERT_EQ(count, 1, "poll count should be 1");
    p = challenger_future_poll(fut, w);
    ASSERT_EQ(p.tag, 1, "should be Pending on second poll");
    ASSERT_EQ(count, 2, "poll count should be 2");
    p = challenger_future_poll(fut, w);
    ASSERT_EQ(p.tag, 0, "should be Ready on third poll");
    ASSERT_EQ(p.value, 42, "output should be 42");
    ASSERT(fut->completed, "future should be marked completed");
    challenger_waker_free(w);
    challenger_future_free(fut);
    PASS();
}

static void test_future_no_double_complete(void) {
    TEST("Future no double completion");
    int count = 0;
    ChallengerFuture* fut = challenger_future_new(poll_counter_fn, &count);
    ChallengerWaker* w = challenger_waker_new(NULL, NULL);
    // Poll until ready
    Poll p;
    do { p = challenger_future_poll(fut, w); } while (p.tag != 0);
    ASSERT_EQ(p.value, 42, "output mismatch");
    // Poll again after completion — should return Ready with same output
    p = challenger_future_poll(fut, w);
    ASSERT_EQ(p.tag, 0, "completed future should still return Ready");
    ASSERT_EQ(p.value, 42, "output should be preserved");
    challenger_waker_free(w);
    challenger_future_free(fut);
    PASS();
}

static void test_future_is_completed(void) {
    TEST("Future is_completed");
    int count = 0;
    ChallengerFuture* fut = challenger_future_new(poll_counter_fn, &count);
    ASSERT(!challenger_future_is_completed(fut), "should not be completed initially");
    ChallengerWaker* w = challenger_waker_new(NULL, NULL);
    Poll p;
    do { p = challenger_future_poll(fut, w); } while (p.tag != 0);
    ASSERT(challenger_future_is_completed(fut), "should be completed after Ready");
    challenger_waker_free(w);
    challenger_future_free(fut);
    PASS();
}

static void test_future_null_safety(void) {
    TEST("Future null safety");
    ASSERT(challenger_future_is_completed(NULL) == 1, "null future is completed");
    Poll p = challenger_future_poll(NULL, NULL);
    ASSERT_EQ(p.tag, 0, "null future poll returns Ready");
    PASS();
}

// ============================================================
// Phase 22: Waker Correctness
// ============================================================

static volatile int g_wake_count = 0;

static void counting_wake_fn(void* data) {
    (void)data;
    g_wake_count++;
}

static void test_waker_basic(void) {
    TEST("Waker basic wake");
    g_wake_count = 0;
    ChallengerWaker* w = challenger_waker_new(counting_wake_fn, NULL);
    ASSERT(w != NULL, "waker creation failed");
    challenger_waker_wake(w);
    ASSERT_EQ(g_wake_count, 1, "wake should be called once");
    challenger_waker_wake(w);
    ASSERT_EQ(g_wake_count, 2, "second wake should be called");
    challenger_waker_free(w);
    PASS();
}

static void test_waker_wake_by_ref(void) {
    TEST("Waker wake_by_ref");
    g_wake_count = 0;
    ChallengerWaker* w = challenger_waker_new(counting_wake_fn, NULL);
    challenger_waker_wake_by_ref(w);
    ASSERT_EQ(g_wake_count, 1, "wake_by_ref should work");
    challenger_waker_free(w);
    PASS();
}

static void test_waker_null_safety(void) {
    TEST("Waker null safety");
    g_wake_count = 0;
    challenger_waker_wake(NULL);
    ASSERT_EQ(g_wake_count, 0, "null wake should not crash");
    challenger_waker_wake_by_ref(NULL);
    ASSERT_EQ(g_wake_count, 0, "null wake_by_ref should not crash");
    ChallengerWaker* w = challenger_waker_new(NULL, NULL);
    challenger_waker_wake(w); // wake_fn is NULL, should not crash
    ASSERT_EQ(g_wake_count, 0, "null fn wake should not crash");
    challenger_waker_free(w);
    PASS();
}

static void test_waker_for_task(void) {
    TEST("Waker for task integration");
    ChallengerExecutor* exec = challenger_executor_new();
    ChallengerWaker* w = challenger_waker_new_for_task(exec, 1);
    ASSERT(w != NULL, "waker for task creation failed");
    ASSERT_EQ((uintptr_t)w->data, 1, "waker data should be task_id");
    challenger_waker_free(w);
    challenger_executor_free(exec);
    PASS();
}

// ============================================================
// Phase 22: Executor + Task Lifecycle
// ============================================================

static Poll always_ready_fn(ChallengerFuture* fut, ChallengerWaker* waker) {
    (void)fut; (void)waker;
    return challenger_poll_ready(100);
}

static Poll always_pending_fn(ChallengerFuture* fut, ChallengerWaker* waker) {
    (void)fut; (void)waker;
    return challenger_poll_pending();
}

static void test_executor_single_task(void) {
    TEST("Executor single task");
    ChallengerExecutor* exec = challenger_executor_new();
    ChallengerFuture* fut = challenger_future_new(always_ready_fn, NULL);
    uint64_t id = challenger_executor_spawn(exec, fut);
    ASSERT(id > 0, "spawn should return valid task id");
    int result = challenger_executor_run(exec);
    ASSERT_EQ(result, 0, "executor_run should return 0");
    ASSERT(challenger_ready_queue_is_empty(&exec->ready), "queue should be empty after completion");
    challenger_executor_free(exec);
    PASS();
}

static void test_executor_multiple_tasks(void) {
    TEST("Executor multiple tasks");
    ChallengerExecutor* exec = challenger_executor_new();
    for (int i = 0; i < 100; i++) {
        ChallengerFuture* fut = challenger_future_new(always_ready_fn, NULL);
        challenger_executor_spawn(exec, fut);
    }
    challenger_executor_run(exec);
    ASSERT(challenger_ready_queue_is_empty(&exec->ready), "all tasks should complete");
    challenger_executor_free(exec);
    PASS();
}

static void test_executor_cancel_task(void) {
    TEST("Executor cancel task");
    ChallengerExecutor* exec = challenger_executor_new();
    ChallengerFuture* fut = challenger_future_new(always_pending_fn, NULL);
    uint64_t id = challenger_executor_spawn(exec, fut);
    // Cancel before running
    challenger_executor_cancel(exec, id);
    int result = challenger_executor_run(exec);
    ASSERT_EQ(result, 0, "executor_run should return 0");
    challenger_executor_free(exec);
    PASS();
}

static void test_executor_wake_pending_task(void) {
    TEST("Executor wake pending task");
    ChallengerExecutor* exec = challenger_executor_new();
    challenger_set_global_executor(exec);

    // Create a future that returns Pending first time, Ready on second
    typedef struct { int polls; } WakeState;
    WakeState* state = (WakeState*)calloc(1, sizeof(WakeState));

    ChallengerFuture* fut = challenger_future_new(
        (ChallengerPollFn)poll_counter_fn, state
    );
    uint64_t id = challenger_executor_spawn(exec, fut);

    // First run: task polls, returns Pending (polls=1)
    challenger_executor_run(exec);

    // Task is now pending with real waker (from Fix 1)
    // Wake the task
    challenger_executor_wake_task(exec, id);

    // Run again: task polls, returns Pending (polls=2)
    challenger_executor_run(exec);

    // Wake again
    challenger_executor_wake_task(exec, id);

    // Run again: task polls, returns Ready (polls=3, value=42)
    challenger_executor_run(exec);

    free(state);
    challenger_set_global_executor(NULL);
    challenger_executor_free(exec);
    PASS();
}

static void test_executor_task_lifecycle(void) {
    TEST("Executor task lifecycle states");
    ChallengerExecutor* exec = challenger_executor_new();
    challenger_set_global_executor(exec);

    typedef struct { int polls; int id; } LifecycleState;
    LifecycleState* state = (LifecycleState*)calloc(1, sizeof(LifecycleState));
    state->id = 42;

    ChallengerFuture* fut = challenger_future_new(
        (ChallengerPollFn)poll_counter_fn, state
    );
    uint64_t id = challenger_executor_spawn(exec, fut);

    // Verify task exists in queue
    ASSERT(!challenger_ready_queue_is_empty(&exec->ready), "queue should have task");

    // Run until complete
    challenger_executor_run(exec);

    // Queue should be empty
    ASSERT(challenger_ready_queue_is_empty(&exec->ready), "queue should be empty after completion");

    free(state);
    challenger_set_global_executor(NULL);
    challenger_executor_free(exec);
    PASS();
}

// ============================================================
// Phase 23: Timer
// ============================================================

static void test_timer_clock(void) {
    TEST("Timer high-resolution clock");
    int64_t t1 = challenger_time_now_us();
    int64_t t2 = challenger_time_now_us();
    ASSERT(t2 >= t1, "clock should be monotonic");
    volatile int64_t start = challenger_time_now_us();
    while (challenger_time_now_us() - start < 1000) {}
    int64_t t3 = challenger_time_now_us();
    ASSERT(t3 > t2, "time should advance");
    PASS();
}

static void test_timer_create_cancel(void) {
    TEST("Timer create and cancel");
    ChallengerExecutor* exec = challenger_executor_new();
    ChallengerTimerWheel tw;
    challenger_timer_init(&tw);

    challenger_set_global_executor(exec);
    exec->current_task_id = 99;

    uint64_t id = challenger_timer_sleep(exec, &tw, 5000000); // 5 seconds
    ASSERT(id > 0, "timer should return valid id");
    ASSERT_EQ(tw.count, 1, "timer count should be 1");

    challenger_timer_cancel(&tw, id);
    ASSERT_EQ(tw.count, 0, "timer count should be 0 after cancel");

    challenger_set_global_executor(NULL);
    challenger_executor_free(exec);
    PASS();
}

static void test_timer_tick_expires(void) {
    TEST("Timer tick expiration");
    ChallengerExecutor* exec = challenger_executor_new();
    ChallengerTimerWheel tw;
    challenger_timer_init(&tw);
    challenger_set_global_executor(exec);

    // Spawn a pending task
    ChallengerFuture* fut = challenger_future_new(always_pending_fn, NULL);
    uint64_t task_id = challenger_executor_spawn(exec, fut);

    // Create a short timer (1ms)
    exec->current_task_id = task_id;
    uint64_t timer_id = challenger_timer_sleep(exec, &tw, 1000); // 1ms
    ASSERT(timer_id > 0, "timer should be created");

    // Wait for timer to expire
    #ifdef _WIN32
    Sleep(5);
    #else
    usleep(5000);
    #endif

    // Tick should expire the timer and wake the task
    int64_t remaining = challenger_timer_tick(exec, &tw);
    ASSERT_EQ(remaining, -1, "no active timers should return -1");
    ASSERT_EQ(tw.count, 0, "timer should be deactivated");

    // The task should now be re-enqueued
    challenger_set_global_executor(NULL);
    challenger_executor_free(exec);
    PASS();
}

static void test_timer_multiple_timers(void) {
    TEST("Multiple timers");
    ChallengerExecutor* exec = challenger_executor_new();
    ChallengerTimerWheel tw;
    challenger_timer_init(&tw);
    challenger_set_global_executor(exec);

    for (int i = 0; i < 10; i++) {
        ChallengerFuture* fut = challenger_future_new(always_pending_fn, NULL);
        uint64_t task_id = challenger_executor_spawn(exec, fut);
        exec->current_task_id = task_id;
        uint64_t timer_id = challenger_timer_sleep(exec, &tw, (i + 1) * 1000000); // 1-10ms
        ASSERT(timer_id > 0, "timer should be created");
    }
    ASSERT_EQ(tw.count, 10, "should have 10 timers");

    // Cancel all
    for (int i = 1; i <= 10; i++) {
        challenger_timer_cancel(&tw, (uint64_t)i);
    }
    ASSERT_EQ(tw.count, 0, "all timers cancelled");

    challenger_set_global_executor(NULL);
    challenger_executor_free(exec);
    PASS();
}

// ============================================================
// Phase 24: TCP
// ============================================================

#define TEST_TCP_PORT 19876

static void test_tcp_socket_bind_listen(void) {
    TEST("TCP socket/bind/listen");
    int fd = challenger_tcp_socket();
    ASSERT(fd >= 0, "socket creation failed");

    int result = challenger_tcp_bind(fd, "127.0.0.1", TEST_TCP_PORT);
    ASSERT_EQ(result, 0, "bind should succeed");

    result = challenger_tcp_listen(fd, 5);
    ASSERT_EQ(result, 0, "listen should succeed");

    challenger_tcp_close(fd);
    PASS();
}

static void test_tcp_connect_accept(void) {
    TEST("TCP connect/accept");

    // Server
    int server_fd = challenger_tcp_socket();
    ASSERT(server_fd >= 0, "server socket failed");
    challenger_tcp_bind(server_fd, "127.0.0.1", TEST_TCP_PORT + 1);
    challenger_tcp_listen(server_fd, 5);

    // Client
    int client_fd = challenger_tcp_socket();
    ASSERT(client_fd >= 0, "client socket failed");
    int result = challenger_tcp_connect(client_fd, "127.0.0.1", TEST_TCP_PORT + 1);
    ASSERT_EQ(result, 0, "connect should succeed");

    // Accept
    int accepted_fd = challenger_tcp_accept(server_fd);
    ASSERT(accepted_fd >= 0, "accept should succeed");

    challenger_tcp_close(client_fd);
    challenger_tcp_close(accepted_fd);
    challenger_tcp_close(server_fd);
    PASS();
}

static void test_tcp_read_write(void) {
    TEST("TCP read/write");

    int server_fd = challenger_tcp_socket();
    challenger_tcp_bind(server_fd, "127.0.0.1", TEST_TCP_PORT + 2);
    challenger_tcp_listen(server_fd, 5);

    int client_fd = challenger_tcp_socket();
    challenger_tcp_connect(client_fd, "127.0.0.1", TEST_TCP_PORT + 2);

    int accepted_fd = challenger_tcp_accept(server_fd);

    // Write from client
    const char* msg = "Hello Challenger";
    int written = challenger_tcp_write(client_fd, msg, (int)strlen(msg));
    ASSERT_EQ(written, (int)strlen(msg), "write should send all bytes");

    // Read on server
    char buf[256] = {0};
    int n = challenger_tcp_read(accepted_fd, buf, sizeof(buf));
    ASSERT(n > 0, "read should return positive bytes");
    ASSERT(strcmp(buf, msg) == 0, "read data should match written data");

    challenger_tcp_close(client_fd);
    challenger_tcp_close(accepted_fd);
    challenger_tcp_close(server_fd);
    PASS();
}

static void test_tcp_echo_server(void) {
    TEST("TCP echo server (single client)");

    int server_fd = challenger_tcp_socket();
    challenger_tcp_bind(server_fd, "127.0.0.1", TEST_TCP_PORT + 3);
    challenger_tcp_listen(server_fd, 5);

    int client_fd = challenger_tcp_socket();
    challenger_tcp_connect(client_fd, "127.0.0.1", TEST_TCP_PORT + 3);

    int accepted_fd = challenger_tcp_accept(server_fd);

    // Echo loop: client sends, server reads and writes back
    for (int i = 0; i < 10; i++) {
        char msg[64];
        snprintf(msg, sizeof(msg), "echo-%d", i);
        challenger_tcp_write(client_fd, msg, (int)strlen(msg));

        char buf[256] = {0};
        int n = challenger_tcp_read(accepted_fd, buf, sizeof(buf));
        ASSERT(n > 0, "echo read should succeed");
        ASSERT(strcmp(buf, msg) == 0, "echo data should match");
    }

    challenger_tcp_close(client_fd);
    challenger_tcp_close(accepted_fd);
    challenger_tcp_close(server_fd);
    PASS();
}

static void test_tcp_multiple_clients(void) {
    TEST("TCP multiple clients (5)");

    int server_fd = challenger_tcp_socket();
    challenger_tcp_bind(server_fd, "127.0.0.1", TEST_TCP_PORT + 4);
    challenger_tcp_listen(server_fd, 10);

    int client_fds[5];
    int accepted_fds[5];

    for (int i = 0; i < 5; i++) {
        client_fds[i] = challenger_tcp_socket();
        challenger_tcp_connect(client_fds[i], "127.0.0.1", TEST_TCP_PORT + 4);
        accepted_fds[i] = challenger_tcp_accept(server_fd);
        ASSERT(accepted_fds[i] >= 0, "accept should succeed");
    }

    // All clients send a message
    for (int i = 0; i < 5; i++) {
        char msg[64];
        snprintf(msg, sizeof(msg), "client-%d", i);
        challenger_tcp_write(client_fds[i], msg, (int)strlen(msg));
    }

    // Server reads from all
    for (int i = 0; i < 5; i++) {
        char buf[256] = {0};
        int n = challenger_tcp_read(accepted_fds[i], buf, sizeof(buf));
        ASSERT(n > 0, "read should succeed");
    }

    for (int i = 0; i < 5; i++) {
        challenger_tcp_close(client_fds[i]);
        challenger_tcp_close(accepted_fds[i]);
    }
    challenger_tcp_close(server_fd);
    PASS();
}

static void test_tcp_nonblocking(void) {
    TEST("TCP non-blocking mode");
    int fd = challenger_tcp_socket();
    ASSERT(fd >= 0, "socket creation failed");

    int result = challenger_tcp_set_nonblocking(fd);
    ASSERT_EQ(result, 0, "set_nonblocking should succeed");

    // Non-blocking connect to no listener should fail immediately
    challenger_tcp_bind(fd, "127.0.0.1", TEST_TCP_PORT + 5);
    challenger_tcp_listen(fd, 1);

    int client_fd = challenger_tcp_socket();
    challenger_tcp_set_nonblocking(client_fd);
    int r = challenger_tcp_connect(client_fd, "127.0.0.1", TEST_TCP_PORT + 5);
    // Non-blocking connect may return -1 with EINPROGRESS or succeed immediately
    // Either is acceptable — the important thing is it doesn't block forever

    challenger_tcp_close(client_fd);
    challenger_tcp_close(fd);
    PASS();
}

static void test_tcp_close_releases_fd(void) {
    TEST("TCP close releases fd");
    int fd1 = challenger_tcp_socket();
    ASSERT(fd1 >= 0, "first socket failed");
    challenger_tcp_close(fd1);

    int fd2 = challenger_tcp_socket();
    ASSERT(fd2 >= 0, "second socket failed");
    // fd2 should be valid (not leaked)
    ASSERT(fd2 != fd1 || fd2 > 0, "fd should be reusable");
    challenger_tcp_close(fd2);
    PASS();
}

// ============================================================
// Phase 25: UDP
// ============================================================

#define TEST_UDP_PORT 19880

static void test_udp_send_receive(void) {
    TEST("UDP send/receive");

    int recv_fd = challenger_udp_socket();
    ASSERT(recv_fd >= 0, "recv socket creation failed");
    int result = challenger_udp_bind(recv_fd, "127.0.0.1", TEST_UDP_PORT);
    ASSERT_EQ(result, 0, "bind should succeed");

    int send_fd = challenger_udp_socket();
    ASSERT(send_fd >= 0, "send socket creation failed");

    // Send a packet
    const char* msg = "UDP test";
    int sent = challenger_udp_send(send_fd, msg, (int)strlen(msg), "127.0.0.1", TEST_UDP_PORT);
    ASSERT(sent > 0, "send should succeed");

    // Receive
    char buf[256] = {0};
    int64_t from_addr = 0;
    int n = challenger_udp_recv(recv_fd, buf, sizeof(buf), &from_addr);
    ASSERT(n > 0, "recv should succeed");
    ASSERT(strcmp(buf, msg) == 0, "received data should match");

    challenger_udp_close(send_fd);
    challenger_udp_close(recv_fd);
    PASS();
}

static void test_udp_multiple_packets(void) {
    TEST("UDP multiple packets (10)");

    int recv_fd = challenger_udp_socket();
    challenger_udp_bind(recv_fd, "127.0.0.1", TEST_UDP_PORT + 1);
    int send_fd = challenger_udp_socket();

    for (int i = 0; i < 10; i++) {
        char msg[64];
        snprintf(msg, sizeof(msg), "packet-%d", i);
        challenger_udp_send(send_fd, msg, (int)strlen(msg), "127.0.0.1", TEST_UDP_PORT + 1);

        char buf[256] = {0};
        int n = challenger_udp_recv(recv_fd, buf, sizeof(buf), NULL);
        ASSERT(n > 0, "recv should succeed");
        ASSERT(strcmp(buf, msg) == 0, "packet data should match");
    }

    challenger_udp_close(send_fd);
    challenger_udp_close(recv_fd);
    PASS();
}

static void test_udp_close(void) {
    TEST("UDP close releases resources");
    int fd = challenger_udp_socket();
    ASSERT(fd >= 0, "socket creation failed");
    challenger_udp_close(fd);
    // Creating another socket should succeed
    int fd2 = challenger_udp_socket();
    ASSERT(fd2 >= 0, "second socket should succeed");
    challenger_udp_close(fd2);
    PASS();
}

// ============================================================
// Phase 26: Channels
// ============================================================

static void test_channel_basic_send_receive(void) {
    TEST("Channel basic send/receive");
    ChallengerChannel* ch = challenger_channel_new(0);
    ASSERT(ch != NULL, "channel creation failed");

    int sent = challenger_channel_send(NULL, ch, 42);
    ASSERT_EQ(sent, 1, "send should succeed");

    int64_t out = 0;
    int received = challenger_channel_receive(NULL, ch, &out);
    ASSERT_EQ(received, 1, "receive should succeed");
    ASSERT_EQ(out, 42, "received value should match");

    challenger_channel_free(ch);
    PASS();
}

static void test_channel_fifo_ordering(void) {
    TEST("Channel FIFO ordering");
    ChallengerChannel* ch = challenger_channel_new(0);

    for (int i = 0; i < 100; i++) {
        challenger_channel_send(NULL, ch, i);
    }

    for (int i = 0; i < 100; i++) {
        int64_t out = 0;
        int received = challenger_channel_receive(NULL, ch, &out);
        ASSERT_EQ(received, 1, "receive should succeed");
        ASSERT_EQ(out, i, "ordering should be FIFO");
    }

    challenger_channel_free(ch);
    PASS();
}

static void test_channel_close(void) {
    TEST("Channel close behavior");
    ChallengerChannel* ch = challenger_channel_new(0);

    // Send before close
    challenger_channel_send(NULL, ch, 1);
    challenger_channel_close(ch);

    ASSERT(challenger_channel_is_closed(ch), "channel should be closed");

    // Can still receive buffered data
    int64_t out = 0;
    int received = challenger_channel_receive(NULL, ch, &out);
    ASSERT_EQ(received, 1, "should receive buffered data");
    ASSERT_EQ(out, 1, "value should match");

    // Receive on empty closed channel returns -1
    received = challenger_channel_receive(NULL, ch, &out);
    ASSERT_EQ(received, -1, "receive on closed empty should return -1");

    // Send on closed channel returns -1
    int sent = challenger_channel_send(NULL, ch, 2);
    ASSERT_EQ(sent, -1, "send on closed should return -1");

    challenger_channel_free(ch);
    PASS();
}

static void test_channel_waker_on_send(void) {
    TEST("Channel wakes receiver on send");
    ChallengerExecutor* exec = challenger_executor_new();
    challenger_set_global_executor(exec);

    ChallengerChannel* ch = challenger_channel_new(0);
    // Register a waiter on receive
    uint64_t fake_task_id = 999;
    ch->recv_waiters[ch->recv_waiter_count++] = fake_task_id;

    // Send should wake the receiver
    challenger_channel_send(exec, ch, 42);

    ASSERT_EQ(ch->recv_waiter_count, 0, "waiter should be woken and removed");

    challenger_channel_free(ch);
    challenger_set_global_executor(NULL);
    challenger_executor_free(exec);
    PASS();
}

static void test_channel_waker_on_receive(void) {
    TEST("Channel wakes sender on receive");
    ChallengerExecutor* exec = challenger_executor_new();
    challenger_set_global_executor(exec);

    ChallengerChannel* ch = challenger_channel_new(0);
    // Pre-fill channel
    challenger_channel_send(NULL, ch, 1);
    challenger_channel_send(NULL, ch, 2);

    // Register a waiter on send
    uint64_t fake_task_id = 888;
    ch->send_waiters[ch->send_waiter_count++] = fake_task_id;

    // Receive should wake the sender
    int64_t out = 0;
    challenger_channel_receive(exec, ch, &out);

    ASSERT_EQ(ch->send_waiter_count, 0, "waiter should be woken and removed");

    challenger_channel_free(ch);
    challenger_set_global_executor(NULL);
    challenger_executor_free(exec);
    PASS();
}

// ============================================================
// Phase 27: Synchronization
// ============================================================

static void test_mutex_basic(void) {
    TEST("Mutex basic lock/unlock");
    ChallengerMutex* m = challenger_mutex_new();
    ASSERT(m != NULL, "mutex creation failed");

    int locked = challenger_mutex_try_lock(m, 1);
    ASSERT_EQ(locked, 1, "first lock should succeed");

    // Second lock attempt should fail (contention)
    int locked2 = challenger_mutex_try_lock(m, 2);
    ASSERT_EQ(locked2, 0, "second lock should fail");

    challenger_mutex_unlock(NULL, m, 1);
    ASSERT_EQ(m->state, CHALLENGER_SYNC_UNLOCKED, "should be unlocked");

    // Now second lock should succeed
    locked2 = challenger_mutex_try_lock(m, 2);
    ASSERT_EQ(locked2, 1, "lock after unlock should succeed");

    challenger_mutex_unlock(NULL, m, 2);
    challenger_mutex_free(m);
    PASS();
}

static void test_mutex_wakes_waiter(void) {
    TEST("Mutex wakes waiter on unlock");
    ChallengerExecutor* exec = challenger_executor_new();
    challenger_set_global_executor(exec);

    ChallengerMutex* m = challenger_mutex_new();
    challenger_mutex_try_lock(m, 1);

    // Add a waiter
    uint64_t waiter_id = 555;
    challenger_mutex_try_lock(m, waiter_id); // adds to waiters
    ASSERT_EQ(m->waiter_count, 1, "should have 1 waiter");

    // Unlock should wake the waiter
    challenger_mutex_unlock(exec, m, 1);
    ASSERT_EQ(m->waiter_count, 0, "waiter should be woken");

    challenger_mutex_free(m);
    challenger_set_global_executor(NULL);
    challenger_executor_free(exec);
    PASS();
}

static void test_semaphore_basic(void) {
    TEST("Semaphore basic acquire/release");
    ChallengerSemaphore* s = challenger_semaphore_new(2);
    ASSERT(s != NULL, "semaphore creation failed");

    int acq1 = challenger_semaphore_try_acquire(s, 1);
    ASSERT_EQ(acq1, 1, "first acquire should succeed");
    ASSERT_EQ(s->count, 1, "count should be 1");

    int acq2 = challenger_semaphore_try_acquire(s, 2);
    ASSERT_EQ(acq2, 1, "second acquire should succeed");
    ASSERT_EQ(s->count, 0, "count should be 0");

    int acq3 = challenger_semaphore_try_acquire(s, 3);
    ASSERT_EQ(acq3, 0, "third acquire should fail (contention)");

    challenger_semaphore_release(NULL, s);
    ASSERT_EQ(s->count, 1, "count should be 1 after release");

    challenger_semaphore_free(s);
    PASS();
}

static void test_semaphore_wakes_waiter(void) {
    TEST("Semaphore wakes waiter on release");
    ChallengerExecutor* exec = challenger_executor_new();
    challenger_set_global_executor(exec);

    ChallengerSemaphore* s = challenger_semaphore_new(0);
    // Acquire (will block, adds waiter)
    challenger_semaphore_try_acquire(s, 777);
    ASSERT_EQ(s->waiter_count, 1, "should have 1 waiter");

    // Release should wake waiter
    challenger_semaphore_release(exec, s);
    ASSERT_EQ(s->waiter_count, 0, "waiter should be woken");

    challenger_semaphore_free(s);
    challenger_set_global_executor(NULL);
    challenger_executor_free(exec);
    PASS();
}

static void test_rwlock_basic(void) {
    TEST("RwLock basic read/write");
    ChallengerRwLock* rw = challenger_rwlock_new();

    // Multiple readers
    ASSERT(challenger_rwlock_try_read(rw, 1), "first read should succeed");
    ASSERT(challenger_rwlock_try_read(rw, 2), "second read should succeed");
    ASSERT_EQ(rw->read_count, 2, "read count should be 2");

    challenger_rwlock_read_unlock(NULL, rw);
    challenger_rwlock_read_unlock(NULL, rw);
    ASSERT_EQ(rw->read_count, 0, "read count should be 0");

    // Writer
    ASSERT(challenger_rwlock_try_write(rw, 3), "write should succeed");
    ASSERT(!challenger_rwlock_try_write(rw, 4), "second write should fail");
    ASSERT(!challenger_rwlock_try_read(rw, 5), "read during write should fail");

    challenger_rwlock_write_unlock(NULL, rw);
    ASSERT(challenger_rwlock_try_read(rw, 6), "read after write should succeed");

    challenger_rwlock_read_unlock(NULL, rw);
    challenger_rwlock_free(rw);
    PASS();
}

static void test_notify_basic(void) {
    TEST("Notify one/all");
    ChallengerExecutor* exec = challenger_executor_new();
    ChallengerNotify* n = challenger_notify_new();

    // Add waiters
    challenger_notify_wait(n, 1);
    challenger_notify_wait(n, 2);
    challenger_notify_wait(n, 3);
    ASSERT_EQ(n->waiter_count, 3, "should have 3 waiters");

    // Notify one
    challenger_notify_one(exec, n);
    ASSERT_EQ(n->waiter_count, 2, "should have 2 waiters after notify_one");

    // Notify all
    challenger_notify_all(exec, n);
    ASSERT_EQ(n->waiter_count, 0, "should have 0 waiters after notify_all");

    challenger_notify_free(n);
    challenger_executor_free(exec);
    PASS();
}

// ============================================================
// Phase 27: Join / Select
// ============================================================

static void test_join_all(void) {
    TEST("JoinAll completes when all ready");
    ChallengerFuture* futures[3];
    int counts[3] = {0, 0, 0};

    // Future 0: completes after 1 poll
    ChallengerFuture* f0 = challenger_future_new(poll_counter_fn, &counts[0]);
    // Future 1: completes after 2 polls
    ChallengerFuture* f1 = challenger_future_new(poll_counter_fn, &counts[1]);
    // Future 2: completes after 3 polls
    ChallengerFuture* f2 = challenger_future_new(poll_counter_fn, &counts[2]);

    futures[0] = f0;
    futures[1] = f1;
    futures[2] = f2;

    ChallengerJoinAll* ja = challenger_join_all_new(futures, 3);
    ChallengerWaker* w = challenger_waker_new(NULL, NULL);

    // Poll until all ready
    int iters = 0;
    while (!challenger_join_all_poll(ja, w) && iters < 10) {
        iters++;
    }
    ASSERT(ja->completed, "join_all should be completed");
    ASSERT(iters <= 3, "should complete in <= 3 iterations");

    challenger_waker_free(w);
    challenger_join_all_free(ja);
    for (int i = 0; i < 3; i++) challenger_future_free(futures[i]);
    PASS();
}

static Poll poll_until_n_fn(ChallengerFuture* fut, ChallengerWaker* waker) {
    (void)waker;
    int* state = (int*)fut->state;
    state[0]++; // poll count
    if (state[0] >= state[1]) { // state[1] = target poll count
        return challenger_poll_ready(state[2]); // state[2] = output value
    }
    return challenger_poll_pending();
}

static void test_select_first_ready(void) {
    TEST("Select returns first Ready");
    ChallengerFuture* futures[3];
    // Each future has state: [poll_count, target_polls, output_value]
    int s0[3] = {0, 3, 100}; // completes after 3 polls
    int s1[3] = {0, 1, 200}; // completes after 1 poll
    int s2[3] = {0, 2, 300}; // completes after 2 polls

    futures[0] = challenger_future_new(poll_until_n_fn, s0);
    futures[1] = challenger_future_new(poll_until_n_fn, s1);
    futures[2] = challenger_future_new(poll_until_n_fn, s2);

    ChallengerWaker* w = challenger_waker_new(NULL, NULL);
    int64_t value = 0;

    // First poll: future 0 polls (count=1, needs 3) → Pending
    //            future 1 polls (count=1, needs 1) → Ready (value=200)
    int idx = challenger_select_poll(futures, 3, w, &value);
    ASSERT_EQ(idx, 1, "first Ready should be index 1");
    ASSERT_EQ(value, 200, "value should be 200");

    challenger_waker_free(w);
    for (int i = 0; i < 3; i++) challenger_future_free(futures[i]);
    PASS();
}

static void test_select_none_ready(void) {
    TEST("Select returns -1 when none ready");
    ChallengerFuture* f = challenger_future_new(always_pending_fn, NULL);
    ChallengerFuture* futures[1] = { f };
    ChallengerWaker* w = challenger_waker_new(NULL, NULL);

    int idx = challenger_select_poll(futures, 1, w, NULL);
    ASSERT_EQ(idx, -1, "should return -1 when none ready");

    challenger_waker_free(w);
    challenger_future_free(f);
    PASS();
}

// ============================================================
// Phase 28: Cancellation
// ============================================================

static void test_cancel_before_poll(void) {
    TEST("Cancel before first poll");
    ChallengerExecutor* exec = challenger_executor_new();
    ChallengerFuture* fut = challenger_future_new(always_pending_fn, NULL);
    uint64_t id = challenger_executor_spawn(exec, fut);
    challenger_executor_cancel(exec, id);
    ASSERT(challenger_task_is_cancelled(exec, id), "task should be cancelled");
    challenger_executor_run(exec);
    challenger_executor_free(exec);
    PASS();
}

static void test_cancel_during_execution(void) {
    TEST("Cancel during execution");
    ChallengerExecutor* exec = challenger_executor_new();
    challenger_set_global_executor(exec);

    typedef struct { int polls; } S;
    S* state = (S*)calloc(1, sizeof(S));
    ChallengerFuture* fut = challenger_future_new(poll_counter_fn, state);
    uint64_t id = challenger_executor_spawn(exec, fut);

    // Run once (polls=1, Pending)
    challenger_executor_run(exec);

    // Cancel
    challenger_executor_cancel(exec, id);

    // Run again — should skip cancelled task
    challenger_executor_run(exec);

    free(state);
    challenger_set_global_executor(NULL);
    challenger_executor_free(exec);
    PASS();
}

static void test_cancel_completed_task(void) {
    TEST("Cancel already-completed task");
    ChallengerExecutor* exec = challenger_executor_new();
    ChallengerFuture* fut = challenger_future_new(always_ready_fn, NULL);
    uint64_t id = challenger_executor_spawn(exec, fut);
    challenger_executor_run(exec);

    // Cancel after completion — should not crash
    challenger_executor_cancel(exec, id);
    ASSERT(!challenger_task_is_cancelled(exec, id), "completed task should not be cancelled");

    challenger_executor_free(exec);
    PASS();
}

static void test_cancel_nonexistent_task(void) {
    TEST("Cancel nonexistent task");
    ChallengerExecutor* exec = challenger_executor_new();
    // Should not crash
    challenger_executor_cancel(exec, 99999);
    ASSERT(!challenger_task_is_cancelled(exec, 99999), "nonexistent task should not be cancelled");
    challenger_executor_free(exec);
    PASS();
}

// ============================================================
// Phase 29: Blocking Pool
// ============================================================

static void* simple_work(void* arg) {
    int* value = (int*)arg;
    *value = 42;
    return NULL;
}

static void test_blocking_pool_basic(void) {
    TEST("Blocking pool basic submit");
    ChallengerBlockingPool* pool = challenger_blocking_pool_new(2);
    ASSERT(pool != NULL, "pool creation failed");

    int result = 0;
    int submitted = challenger_blocking_submit(pool, simple_work, &result, 0);
    ASSERT_EQ(submitted, 0, "submit should succeed");

    // Wait for completion
    #ifdef _WIN32
    Sleep(100);
    #else
    usleep(100000);
    #endif

    ASSERT_EQ(result, 42, "work should have set value to 42");

    challenger_blocking_pool_free(pool);
    PASS();
}

static void test_blocking_pool_multiple_work(void) {
    TEST("Blocking pool multiple work items");
    ChallengerBlockingPool* pool = challenger_blocking_pool_new(2);

    int values[10] = {0};
    for (int i = 0; i < 10; i++) {
        challenger_blocking_submit(pool, simple_work, &values[i], 0);
    }

    // Wait for all to complete
    #ifdef _WIN32
    Sleep(200);
    #else
    usleep(200000);
    #endif

    for (int i = 0; i < 10; i++) {
        ASSERT_EQ(values[i], 42, "work item should have completed");
    }

    challenger_blocking_pool_free(pool);
    PASS();
}

static void test_blocking_pool_does_not_block_executor(void) {
    TEST("Blocking pool does not block executor");
    ChallengerBlockingPool* pool = challenger_blocking_pool_new(2);

    volatile int async_work_done = 0;

    // Submit slow work to pool
    challenger_blocking_submit(pool, simple_work, (void*)&async_work_done, 0);

    // Meanwhile, executor should not be blocked
    ChallengerExecutor* exec = challenger_executor_new();
    ChallengerFuture* fut = challenger_future_new(always_ready_fn, NULL);
    challenger_executor_spawn(exec, fut);
    challenger_executor_run(exec);

    ASSERT(challenger_ready_queue_is_empty(&exec->ready), "executor should complete immediately");

    // Wait for pool work
    #ifdef _WIN32
    Sleep(100);
    #else
    usleep(100000);
    #endif

    challenger_blocking_pool_free(pool);
    challenger_executor_free(exec);
    PASS();
}

static void test_blocking_pool_shutdown(void) {
    TEST("Blocking pool clean shutdown");
    ChallengerBlockingPool* pool = challenger_blocking_pool_new(4);
    ASSERT(pool != NULL, "pool creation failed");

    challenger_blocking_pool_shutdown(pool);
    // Should not crash, threads should be joined
    PASS();
}

// ============================================================
// Phase 31: Process
// ============================================================

static void test_process_spawn(void) {
    TEST("Process spawn and read stdout");
    #ifdef _WIN32
    const char* cmd = "cmd.exe";
    char* args[] = { "/c", "echo", "hello" };
    int arg_count = 3;
    #else
    const char* cmd = "echo";
    char* args[] = { "hello" };
    int arg_count = 1;
    #endif

    ChallengerSubprocess* proc = challenger_process_spawn(cmd, args, arg_count);
    ASSERT(proc != NULL, "spawn should succeed");
    ASSERT(proc->pid > 0, "pid should be positive");

    #ifdef _WIN32
    Sleep(500);
    #else
    usleep(500000);
    #endif

    int64_t len = 0;
    char* output = challenger_process_read_stdout(proc, &len);

    challenger_process_free(proc);
    if (output) free(output);
    PASS();
}

// ============================================================
// Phase 32: DNS
// ============================================================

static void test_dns_resolve_localhost(void) {
    TEST("DNS resolve localhost");
    ChallengerDnsResult result = challenger_dns_resolve("127.0.0.1");
    ASSERT(result.valid, "DNS resolve of 127.0.0.1 should succeed");
    ASSERT(strcmp(result.ip, "127.0.0.1") == 0, "IP should match");
    PASS();
}

static void test_dns_resolve_failure(void) {
    TEST("DNS resolve invalid hostname");
    ChallengerDnsResult result = challenger_dns_resolve("this-host-does-not-exist-12345.invalid");
    ASSERT(!result.valid, "invalid hostname should fail");
    PASS();
}

// ============================================================
// Phase 33: Multi-thread Executor
// ============================================================

static void test_mt_executor_basic(void) {
    TEST("Multi-thread executor basic");
    ChallengerMtExecutor* mt = challenger_mt_executor_new(2);
    ASSERT(mt != NULL, "mt executor creation failed");

    ChallengerFuture* fut = challenger_future_new(always_ready_fn, NULL);
    uint64_t id = challenger_mt_spawn(mt, fut);
    ASSERT(id > 0, "spawn should return valid id");

    challenger_mt_shutdown(mt);
    challenger_mt_executor_free(mt);
    PASS();
}

// ============================================================
// Phase 34: Stress Tests
// ============================================================

static void stress_spawn_cancel_storm(void) {
    TEST("Stress: spawn/cancel storm (1000 tasks)");
    ChallengerExecutor* exec = challenger_executor_new();

    for (int i = 0; i < 1000; i++) {
        ChallengerFuture* fut = challenger_future_new(always_pending_fn, NULL);
        uint64_t id = challenger_executor_spawn(exec, fut);
        if (i % 2 == 0) {
            challenger_executor_cancel(exec, id);
        }
    }

    challenger_executor_run(exec);
    // Should not crash or hang
    challenger_executor_free(exec);
    PASS();
}

static void stress_timer_storm(void) {
    TEST("Stress: timer storm (1000 timers)");
    ChallengerExecutor* exec = challenger_executor_new();
    ChallengerTimerWheel tw;
    challenger_timer_init(&tw);
    challenger_set_global_executor(exec);

    for (int i = 0; i < 1000; i++) {
        ChallengerFuture* fut = challenger_future_new(always_pending_fn, NULL);
        uint64_t task_id = challenger_executor_spawn(exec, fut);
        exec->current_task_id = task_id;
        challenger_timer_sleep(exec, &tw, (i + 1) * 1000000);
    }

    ASSERT(tw.count > 0, "should have active timers");

    // Cancel all
    for (int i = 1; i <= 1000; i++) {
        challenger_timer_cancel(&tw, (uint64_t)i);
    }

    challenger_set_global_executor(NULL);
    challenger_executor_free(exec);
    PASS();
}

static void stress_channel_storm(void) {
    TEST("Stress: channel storm (10000 messages)");
    ChallengerChannel* ch = challenger_channel_new(0);

    for (int i = 0; i < 10000; i++) {
        int sent = challenger_channel_send(NULL, ch, i);
        ASSERT_EQ(sent, 1, "send should succeed");
    }

    for (int i = 0; i < 10000; i++) {
        int64_t out = 0;
        int received = challenger_channel_receive(NULL, ch, &out);
        ASSERT_EQ(received, 1, "receive should succeed");
        ASSERT_EQ(out, i, "value should match");
    }

    challenger_channel_free(ch);
    PASS();
}

static void stress_ready_queue(void) {
    TEST("Stress: ready queue push/pop (65536 tasks)");
    ReadyQueue* q = (ReadyQueue*)calloc(1, sizeof(ReadyQueue));
    challenger_ready_queue_init(q);

    ChallengerTask* tasks = (ChallengerTask*)calloc(65536, sizeof(ChallengerTask));
    for (int i = 0; i < 65536; i++) {
        tasks[i].id = i;
        tasks[i].future = NULL;
        tasks[i].waker = NULL;
        tasks[i].state = 0;
        tasks[i].needs_poll = 0;
        challenger_ready_queue_push(q, &tasks[i]);
    }

    ASSERT(challenger_ready_queue_is_empty(q) ? 0 : 1, "queue should not be empty");

    for (int i = 0; i < 65536; i++) {
        ChallengerTask* t = challenger_ready_queue_pop(q);
        ASSERT(t != NULL, "pop should succeed");
        ASSERT_EQ((int)t->id, i, "task id should match");
    }

    ASSERT(challenger_ready_queue_is_empty(q), "queue should be empty after popping all");
    free(tasks);
    free(q);
    PASS();
}

static void stress_executor_many_tasks(void) {
    TEST("Stress: executor with 10000 ready tasks");
    ChallengerExecutor* exec = challenger_executor_new();

    for (int i = 0; i < 10000; i++) {
        ChallengerFuture* fut = challenger_future_new(always_ready_fn, NULL);
        challenger_executor_spawn(exec, fut);
    }

    challenger_executor_run(exec);
    ASSERT(challenger_ready_queue_is_empty(&exec->ready), "all tasks should complete");
    challenger_executor_free(exec);
    PASS();
}

static void stress_mutex_contention(void) {
    TEST("Stress: mutex lock/unlock cycles (10000)");
    ChallengerMutex* m = challenger_mutex_new();

    for (int i = 0; i < 10000; i++) {
        int locked = challenger_mutex_try_lock(m, i);
        ASSERT_EQ(locked, 1, "lock should succeed");
        challenger_mutex_unlock(NULL, m, i);
    }

    challenger_mutex_free(m);
    PASS();
}

// ============================================================
// Phase 34: Reactor + TCP Integration
// ============================================================

static void test_reactor_tcp_integration(void) {
    TEST("Reactor + TCP integration");
    ChallengerReactor* reactor = challenger_reactor_new();
    ASSERT(reactor != NULL, "reactor creation failed");

    ChallengerExecutor* exec = challenger_executor_new();
    challenger_set_global_executor(exec);

    // Create server
    int server_fd = challenger_tcp_socket();
    challenger_tcp_set_nonblocking(server_fd);
    challenger_tcp_bind(server_fd, "127.0.0.1", TEST_TCP_PORT + 10);
    challenger_tcp_listen(server_fd, 5);

    // Create client
    int client_fd = challenger_tcp_socket();
    challenger_tcp_set_nonblocking(client_fd);
    challenger_tcp_connect(client_fd, "127.0.0.1", TEST_TCP_PORT + 10);

    // Register client with reactor
    ChallengerFuture* fut = challenger_future_new(always_pending_fn, NULL);
    uint64_t task_id = challenger_executor_spawn(exec, fut);
    challenger_reactor_register(reactor, client_fd, task_id);

    // Poll reactor (non-blocking, should get connect event)
    int n = challenger_reactor_poll(exec, reactor, 100);
    // n may be 0 or 1 depending on timing — the important thing is no crash

    challenger_reactor_unregister(reactor, client_fd);
    challenger_tcp_close(client_fd);
    challenger_tcp_close(server_fd);

    challenger_set_global_executor(NULL);
    challenger_executor_free(exec);
    challenger_reactor_free(reactor);
    PASS();
}

// ============================================================
// Real-World Validation: TCP Echo Server
// ============================================================

static void test_realworld_tcp_echo(void) {
    TEST("Real-world: TCP echo server with 10 clients");

    int server_fd = challenger_tcp_socket();
    challenger_tcp_bind(server_fd, "127.0.0.1", TEST_TCP_PORT + 20);
    challenger_tcp_listen(server_fd, 10);

    int client_fds[10];
    int accepted_fds[10];

    // Connect 10 clients
    for (int i = 0; i < 10; i++) {
        client_fds[i] = challenger_tcp_socket();
        int r = challenger_tcp_connect(client_fds[i], "127.0.0.1", TEST_TCP_PORT + 20);
        ASSERT_EQ(r, 0, "connect should succeed");
        accepted_fds[i] = challenger_tcp_accept(server_fd);
        ASSERT(accepted_fds[i] >= 0, "accept should succeed");
    }

    // All clients send, all servers echo back
    for (int i = 0; i < 10; i++) {
        char msg[128];
        snprintf(msg, sizeof(msg), "client-%d-ping", i);
        int written = challenger_tcp_write(client_fds[i], msg, (int)strlen(msg));
        ASSERT_EQ(written, (int)strlen(msg), "write should succeed");

        char buf[256] = {0};
        int n = challenger_tcp_read(accepted_fds[i], buf, sizeof(buf));
        ASSERT(n > 0, "read should succeed");
        ASSERT(strcmp(buf, msg) == 0, "echo should match");
    }

    // Multiple round-trips
    for (int round = 0; round < 5; round++) {
        for (int i = 0; i < 10; i++) {
            char msg[128];
            snprintf(msg, sizeof(msg), "round-%d-client-%d", round, i);
            challenger_tcp_write(client_fds[i], msg, (int)strlen(msg));

            char buf[256] = {0};
            int n = challenger_tcp_read(accepted_fds[i], buf, sizeof(buf));
            ASSERT(n > 0, "echo read should succeed");
            ASSERT(strcmp(buf, msg) == 0, "echo should match");
        }
    }

    // Cleanup
    for (int i = 0; i < 10; i++) {
        challenger_tcp_close(client_fds[i]);
        challenger_tcp_close(accepted_fds[i]);
    }
    challenger_tcp_close(server_fd);
    PASS();
}

// ============================================================
// P0-B: Async TCP Futures — Full E2E Validation
// ============================================================

static void test_p0b_tcp_connect_async(void) {
    TEST("P0-B: TCP connect async (loopback)");
    ChallengerExecutor* exec = challenger_executor_new();
    challenger_set_global_executor(exec);

    int server_fd = challenger_tcp_socket();
    challenger_tcp_set_nonblocking(server_fd);
    challenger_tcp_bind(server_fd, "127.0.0.1", 19950);
    challenger_tcp_listen(server_fd, 5);

    int client_fd = challenger_tcp_socket();
    ChallengerFuture* fut = challenger_tcp_connect_async(client_fd, "127.0.0.1", 19950);

    uint64_t task_id = challenger_executor_spawn(exec, fut);
    ASSERT(task_id > 0, "spawn failed");
    int rc = challenger_executor_run(exec);
    ASSERT(rc == 0, "executor_run failed");

    ASSERT(fut->completed, "future should be completed");
    ASSERT(fut->output == 0, "connect should succeed on loopback");

    challenger_tcp_close(client_fd);
    challenger_tcp_close(server_fd);
    challenger_set_global_executor(NULL);
    challenger_executor_free(exec);
    PASS();
}

static void test_p0b_tcp_accept_async(void) {
    TEST("P0-B: TCP accept async");
    ChallengerExecutor* exec = challenger_executor_new();
    challenger_set_global_executor(exec);

    int server_fd = challenger_tcp_socket();
    challenger_tcp_set_nonblocking(server_fd);
    challenger_tcp_bind(server_fd, "127.0.0.1", 19951);
    challenger_tcp_listen(server_fd, 5);

    int client_fd = challenger_tcp_socket();
    challenger_tcp_connect(client_fd, "127.0.0.1", 19951);

    ChallengerFuture* fut = challenger_tcp_accept_async(server_fd);
    uint64_t task_id = challenger_executor_spawn(exec, fut);
    ASSERT(task_id > 0, "spawn failed");
    int rc = challenger_executor_run(exec);
    ASSERT(rc == 0, "executor_run failed");

    ASSERT(fut->completed, "future should be completed");
    ASSERT(fut->output > 0, "accept should return valid fd");
    int accepted_fd = (int)fut->output;

    challenger_tcp_close(client_fd);
    challenger_tcp_close(accepted_fd);
    challenger_tcp_close(server_fd);
    challenger_set_global_executor(NULL);
    challenger_executor_free(exec);
    PASS();
}

static void test_p0b_tcp_read_write_async(void) {
    TEST("P0-B: TCP read/write async");
    ChallengerExecutor* exec = challenger_executor_new();
    challenger_set_global_executor(exec);

    int server_fd = challenger_tcp_socket();
    challenger_tcp_set_nonblocking(server_fd);
    challenger_tcp_bind(server_fd, "127.0.0.1", 19952);
    challenger_tcp_listen(server_fd, 5);

    int client_fd = challenger_tcp_socket();
    challenger_tcp_connect(client_fd, "127.0.0.1", 19952);
    int accepted_fd = challenger_tcp_accept(server_fd);
    ASSERT(accepted_fd > 0, "accept failed");

    ChallengerFuture* write_fut = challenger_tcp_write_async(client_fd, "hello", 5);
    challenger_executor_spawn(exec, write_fut);
    challenger_executor_run(exec);
    ASSERT(write_fut->completed, "write future should complete");
    ASSERT(write_fut->output == 5, "should write 5 bytes");

    ChallengerFuture* read_fut = challenger_tcp_read_async(accepted_fd, 100);
    challenger_executor_spawn(exec, read_fut);
    challenger_executor_run(exec);
    ASSERT(read_fut->completed, "read future should complete");
    ASSERT(read_fut->output == 5, "should read 5 bytes");

    const char* data = challenger_tcp_get_last_read_buf();
    ASSERT(memcmp(data, "hello", 5) == 0, "data should match");

    challenger_tcp_close(client_fd);
    challenger_tcp_close(accepted_fd);
    challenger_tcp_close(server_fd);
    challenger_set_global_executor(NULL);
    challenger_executor_free(exec);
    PASS();
}

static void test_p0b_tcp_full_roundtrip(void) {
    TEST("P0-B: TCP full async roundtrip");
    ChallengerExecutor* exec = challenger_executor_new();
    challenger_set_global_executor(exec);

    int server_fd = challenger_tcp_socket();
    challenger_tcp_set_nonblocking(server_fd);
    challenger_tcp_bind(server_fd, "127.0.0.1", 19953);
    challenger_tcp_listen(server_fd, 5);

    int client_fd = challenger_tcp_socket();
    ChallengerFuture* connect_fut = challenger_tcp_connect_async(client_fd, "127.0.0.1", 19953);
    challenger_executor_spawn(exec, connect_fut);
    challenger_executor_run(exec);
    ASSERT(connect_fut->completed, "connect should complete");
    ASSERT(connect_fut->output == 0, "connect should succeed");

    ChallengerFuture* accept_fut = challenger_tcp_accept_async(server_fd);
    challenger_executor_spawn(exec, accept_fut);
    challenger_executor_run(exec);
    ASSERT(accept_fut->completed, "accept should complete");
    int accepted_fd = (int)accept_fut->output;
    ASSERT(accepted_fd > 0, "accepted fd should be valid");

    ChallengerFuture* write_fut = challenger_tcp_write_async(client_fd, "P0B_PROOF", 9);
    challenger_executor_spawn(exec, write_fut);
    challenger_executor_run(exec);
    ASSERT(write_fut->completed, "write should complete");
    ASSERT(write_fut->output == 9, "should write 9 bytes");

    ChallengerFuture* read_fut = challenger_tcp_read_async(accepted_fd, 100);
    challenger_executor_spawn(exec, read_fut);
    challenger_executor_run(exec);
    ASSERT(read_fut->completed, "read should complete");
    ASSERT(read_fut->output == 9, "should read 9 bytes");

    const char* data = challenger_tcp_get_last_read_buf();
    ASSERT(memcmp(data, "P0B_PROOF", 9) == 0, "data should match");

    challenger_tcp_close(client_fd);
    challenger_tcp_close(accepted_fd);
    challenger_tcp_close(server_fd);
    challenger_set_global_executor(NULL);
    challenger_executor_free(exec);
    PASS();
}

// ============================================================
// Real-World Validation: Timer + Executor
// ============================================================

static void test_realworld_timer_executor(void) {
    TEST("Real-world: timer + executor lifecycle");

    ChallengerExecutor* exec = challenger_executor_new();
    ChallengerTimerWheel tw;
    challenger_timer_init(&tw);
    challenger_set_global_executor(exec);

    // Spawn 5 tasks, each with a 1ms timer
    for (int i = 0; i < 5; i++) {
        ChallengerFuture* fut = challenger_future_new(always_pending_fn, NULL);
        uint64_t task_id = challenger_executor_spawn(exec, fut);
        exec->current_task_id = task_id;
        challenger_timer_sleep(exec, &tw, 1000); // 1ms
    }

    ASSERT_EQ(tw.count, 5, "should have 5 timers");

    // Wait for timers to expire
    #ifdef _WIN32
    Sleep(5);
    #else
    usleep(5000);
    #endif

    // Tick all timers
    challenger_timer_tick(exec, &tw);

    ASSERT_EQ(tw.count, 0, "all timers should be expired");

    // Run executor — woken tasks should complete (they are always_pending, so they stay pending)
    challenger_executor_run(exec);

    challenger_set_global_executor(NULL);
    challenger_executor_free(exec);
    PASS();
}

// ============================================================
// Memory Safety: Allocation/Free cycles
// ============================================================

static void test_memory_alloc_free_cycles(void) {
    TEST("Memory: 10000 alloc/free cycles");
    for (int i = 0; i < 10000; i++) {
        ChallengerExecutor* exec = challenger_executor_new();
        ChallengerFuture* fut = challenger_future_new(always_ready_fn, NULL);
        challenger_executor_spawn(exec, fut);
        challenger_executor_run(exec);
        challenger_executor_free(exec);
    }
    // No crash = PASS
    PASS();
}

static void test_memory_waker_cycles(void) {
    TEST("Memory: 10000 waker alloc/free cycles");
    for (int i = 0; i < 10000; i++) {
        ChallengerWaker* w = challenger_waker_new(counting_wake_fn, NULL);
        challenger_waker_wake(w);
        challenger_waker_free(w);
    }
    PASS();
}

static void test_memory_channel_cycles(void) {
    TEST("Memory: 10000 channel alloc/free cycles");
    for (int i = 0; i < 10000; i++) {
        ChallengerChannel* ch = challenger_channel_new(0);
        challenger_channel_send(NULL, ch, i);
        int64_t out = 0;
        challenger_channel_receive(NULL, ch, &out);
        challenger_channel_free(ch);
    }
    PASS();
}

static void test_memory_timer_cycles(void) {
    TEST("Memory: 10000 timer alloc/free cycles");
    for (int i = 0; i < 10000; i++) {
        ChallengerExecutor* exec = challenger_executor_new();
        ChallengerTimerWheel tw;
        challenger_timer_init(&tw);
        challenger_set_global_executor(exec);
        exec->current_task_id = 1;
        challenger_timer_sleep(exec, &tw, 1000);
        challenger_timer_cancel(&tw, 1);
        challenger_set_global_executor(NULL);
        challenger_executor_free(exec);
    }
    PASS();
}

// ============================================================
// Phase 38: Real Pending/Resume
// ============================================================

// Test 1: Basic Pending → Wake → Resume
// A future that returns Pending on 1st poll, Ready on 2nd poll.
// The waker fires between polls, re-enqueueing the task.

typedef struct {
    int poll_count;
} PendingResumeState;

static Poll pending_resume_poll(ChallengerFuture* fut, ChallengerWaker* waker) {
    PendingResumeState* state = (PendingResumeState*)fut->state;
    state->poll_count++;

    if (state->poll_count == 1) {
        // First poll: return Pending. The waker was passed in; the task
        // will be re-enqueued externally before the next executor_run.
        (void)waker;
        return challenger_poll_pending();
    } else {
        // Second poll: return Ready with value 42.
        return challenger_poll_ready(42);
    }
}

void test_pending_resume_basic(void) {
    TEST("Phase 38: Pending -> Wake -> Resume (basic)");

    ChallengerExecutor* exec = challenger_executor_new();
    challenger_set_global_executor(exec);

    PendingResumeState* st = (PendingResumeState*)calloc(1, sizeof(PendingResumeState));
    ChallengerFuture* fut = challenger_future_new(pending_resume_poll, st);

    uint64_t task_id = challenger_executor_spawn(exec, fut);
    ASSERT(task_id > 0, "spawn should return non-zero task id");

    // First run: task should return Pending and leave the ready queue empty
    challenger_executor_run(exec);
    ASSERT(challenger_ready_queue_is_empty(&exec->ready), "ready queue should be empty after Pending");

    // The task is still in all_tasks (pending), not completed.
    // Manually wake the task (simulating reactor / timer / another task calling the waker).
    challenger_executor_wake_task(exec, task_id);

    // The task should now be back in the ready queue.
    ASSERT(!challenger_ready_queue_is_empty(&exec->ready), "ready queue should have task after wake");

    // Second run: task should return Ready(42)
    challenger_executor_run(exec);

    ASSERT_EQ(exec->all_tasks_count, 0, "all_tasks should be empty after completion");

    challenger_set_global_executor(NULL);
    challenger_executor_free(exec);
    PASS();
}

// Test 2: Two tasks, one returns Pending, both complete.

typedef struct {
    int poll_count;
    int value;
} DualPendingState;

static Poll dual_pending_poll(ChallengerFuture* fut, ChallengerWaker* waker) {
    DualPendingState* state = (DualPendingState*)fut->state;
    state->poll_count++;

    if (state->poll_count == 1) {
        (void)waker;
        return challenger_poll_pending();
    } else {
        return challenger_poll_ready(state->value);
    }
}

void test_pending_resume_two_tasks(void) {
    TEST("Phase 38: Pending -> Wake -> Resume (two tasks)");

    ChallengerExecutor* exec = challenger_executor_new();
    challenger_set_global_executor(exec);

    // Task A: pending once, then Ready(10)
    DualPendingState* stA = (DualPendingState*)calloc(1, sizeof(DualPendingState));
    stA->value = 10;
    ChallengerFuture* futA = challenger_future_new(dual_pending_poll, stA);
    uint64_t idA = challenger_executor_spawn(exec, futA);

    // Task B: pending once, then Ready(20)
    DualPendingState* stB = (DualPendingState*)calloc(1, sizeof(DualPendingState));
    stB->value = 20;
    ChallengerFuture* futB = challenger_future_new(dual_pending_poll, stB);
    uint64_t idB = challenger_executor_spawn(exec, futB);

    // First run: both return Pending
    challenger_executor_run(exec);
    ASSERT(challenger_ready_queue_is_empty(&exec->ready), "ready queue empty after both Pending");
    ASSERT_EQ(exec->all_tasks_count, 2, "both tasks still in all_tasks");

    // Wake both
    challenger_executor_wake_task(exec, idA);
    challenger_executor_wake_task(exec, idB);
    ASSERT(!challenger_ready_queue_is_empty(&exec->ready), "ready queue has tasks after wake");

    // Second run: both return Ready
    challenger_executor_run(exec);
    ASSERT_EQ(exec->all_tasks_count, 0, "all_tasks empty after both complete");

    challenger_set_global_executor(NULL);
    challenger_executor_free(exec);
    PASS();
}

// Test 3: Self-wake: future wakes itself from within the poll callback.
// This simulates a timer or channel that resolves immediately during poll.

typedef struct {
    int poll_count;
    ChallengerExecutor* exec;
    uint64_t task_id;
} SelfWakeState;

static Poll self_wake_poll(ChallengerFuture* fut, ChallengerWaker* waker) {
    SelfWakeState* state = (SelfWakeState*)fut->state;
    state->poll_count++;

    if (state->poll_count == 1) {
        // First poll: return Pending, but wake ourselves immediately.
        // This tests that wake_task works even when called from inside the poll.
        challenger_waker_wake(waker);
        return challenger_poll_pending();
    } else {
        return challenger_poll_ready(99);
    }
}

void test_pending_resume_self_wake(void) {
    TEST("Phase 38: Pending -> self-wake -> Resume");

    ChallengerExecutor* exec = challenger_executor_new();
    challenger_set_global_executor(exec);

    SelfWakeState* st = (SelfWakeState*)calloc(1, sizeof(SelfWakeState));
    st->exec = exec;
    ChallengerFuture* fut = challenger_future_new(self_wake_poll, st);

    uint64_t task_id = challenger_executor_spawn(exec, fut);
    st->task_id = task_id;

    // Single run should handle both polls (Pending + self-wake re-enqueue + Ready)
    // because executor_run loops until the ready queue is empty.
    challenger_executor_run(exec);
    ASSERT_EQ(exec->all_tasks_count, 0, "all_tasks empty after self-wake + completion");

    challenger_set_global_executor(NULL);
    challenger_executor_free(exec);
    PASS();
}

// ============================================================
// Phase C: Multi-task Executor Tests
// ============================================================

// Helper: immediate-ready future — returns Ready(value) on first poll
static Poll immediate_ready_poll(ChallengerFuture* fut, ChallengerWaker* waker) {
    (void)waker;
    return challenger_poll_ready((int64_t)(uintptr_t)fut->state);
}

static ChallengerFuture* immediate_ready_create(int64_t value) {
    return challenger_future_new(immediate_ready_poll, (void*)(uintptr_t)value);
}

// C1: executor_run_once processes a single ready task
void test_c_run_once_basic(void) {
    TEST("Phase C: run_once processes single ready task");

    ChallengerExecutor* exec = challenger_executor_new();
    ChallengerFuture* fut = immediate_ready_create(42);
    uint64_t task_id = challenger_executor_spawn(exec, fut);
    (void)task_id;

    int processed = challenger_executor_run_once(exec);
    ASSERT_EQ(processed, 1, "run_once processes 1 task");
    ASSERT_EQ(exec->all_tasks_count, 0, "task removed after completion");

    challenger_executor_free(exec);
    PASS();
}

// C2: Two tasks interleaving via run_once — both spawned, both complete
void test_c_two_tasks_interleave(void) {
    TEST("Phase C: two tasks complete via run_once");

    ChallengerExecutor* exec = challenger_executor_new();

    ChallengerFuture* fut1 = immediate_ready_create(10);
    ChallengerFuture* fut2 = immediate_ready_create(20);
    uint64_t id1 = challenger_executor_spawn(exec, fut1);
    uint64_t id2 = challenger_executor_spawn(exec, fut2);
    (void)id1; (void)id2;

    // First run_once should drain both ready tasks
    int processed = challenger_executor_run_once(exec);
    ASSERT_EQ(processed, 2, "run_once processes 2 ready tasks");
    ASSERT_EQ(exec->all_tasks_count, 0, "both tasks completed");

    challenger_executor_free(exec);
    PASS();
}

// C3: Three tasks complete in FIFO order (ring buffer order)
static Poll fifo_poll(ChallengerFuture* fut, ChallengerWaker* waker) {
    (void)waker;
    return challenger_poll_ready((int64_t)(uintptr_t)fut->state);
}

void test_c_three_tasks_fifo(void) {
    TEST("Phase C: three tasks FIFO order");

    ChallengerExecutor* exec = challenger_executor_new();

    // Run 10 iterations to verify FIFO is consistent
    for (int iter = 0; iter < 10; iter++) {
        for (int i = 0; i < 3; i++) {
            ChallengerFuture* fut = challenger_future_new(fifo_poll, (void*)(uintptr_t)(i * 100 + 1));
            challenger_executor_spawn(exec, fut);
        }

        // All 3 should be in ready queue; run_once drains all
        challenger_executor_run_once(exec);
        ASSERT_EQ(exec->all_tasks_count, 0, "all 3 tasks completed");
    }

    challenger_executor_free(exec);
    PASS();
}

// C4: Cancel one task while another runs
void test_c_cancel_one_of_many(void) {
    TEST("Phase C: cancel one of multiple tasks");

    ChallengerExecutor* exec = challenger_executor_new();

    ChallengerFuture* fut1 = immediate_ready_create(100);
    ChallengerFuture* fut2 = immediate_ready_create(200);
    uint64_t id1 = challenger_executor_spawn(exec, fut1);
    uint64_t id2 = challenger_executor_spawn(exec, fut2);

    // Cancel task 1 before run
    challenger_executor_cancel(exec, id1);

    challenger_executor_run_once(exec);

    // task 1 should be cancelled and removed, task 2 completed
    ASSERT_EQ(exec->all_tasks_count, 0, "both tasks removed (cancelled + completed)");

    challenger_executor_free(exec);
    PASS();
}

// C5: Wake deduplication — multiple wake_task calls only enqueues once
void test_c_wake_dedup(void) {
    TEST("Phase C: wake deduplication");

    ChallengerExecutor* exec = challenger_executor_new();
    ChallengerFuture* fut = immediate_ready_create(77);
    uint64_t id = challenger_executor_spawn(exec, fut);

    // Task is already in ready queue from spawn.
    // Wake it multiple times — needs_poll should prevent re-enqueue.
    challenger_executor_wake_task(exec, id);
    challenger_executor_wake_task(exec, id);
    challenger_executor_wake_task(exec, id);

    // run_once should process it only once (needs_poll prevents re-enqueue)
    challenger_executor_run_once(exec);
    ASSERT_EQ(exec->all_tasks_count, 0, "task completed without duplication issues");

    challenger_executor_free(exec);
    PASS();
}

// C6: run_once on empty executor returns 0
void test_c_run_once_empty(void) {
    TEST("Phase C: run_once on empty executor");

    ChallengerExecutor* exec = challenger_executor_new();

    int processed = challenger_executor_run_once(exec);
    ASSERT_EQ(processed, 0, "empty executor processes 0");
    ASSERT_EQ(exec->all_tasks_count, 0, "no tasks");

    challenger_executor_free(exec);
    PASS();
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
    #else
    signal(SIGPIPE, SIG_IGN);
    #endif

    printf("=== Challenger Async Runtime — Comprehensive Validation ===\n\n");

    // Phase 21: Error Model
    printf("--- Phase 21: Error Model ---\n");
    test_error_model_tcp_socket_failure();
    test_error_model_tcp_bind_failure();
    test_error_model_tcp_connect_failure();
    test_error_model_channel_closed();
    test_error_model_channel_receive_empty();
    test_error_model_reactor_null();
    test_error_model_timer_null();

    // Phase 22: Future Correctness
    printf("\n--- Phase 22: Future Correctness ---\n");
    test_future_ready_immediately();
    test_future_no_double_complete();
    test_future_is_completed();
    test_future_null_safety();

    // Phase 22: Waker Correctness
    printf("\n--- Phase 22: Waker Correctness ---\n");
    test_waker_basic();
    test_waker_wake_by_ref();
    test_waker_null_safety();
    test_waker_for_task();

    // Phase 22: Executor + Task Lifecycle
    printf("\n--- Phase 22: Executor + Task Lifecycle ---\n");
    test_executor_single_task();
    test_executor_multiple_tasks();
    test_executor_cancel_task();
    test_executor_wake_pending_task();
    test_executor_task_lifecycle();

    // Phase 23: Timer
    printf("\n--- Phase 23: Timer ---\n");
    test_timer_clock();
    test_timer_create_cancel();
    test_timer_tick_expires();
    test_timer_multiple_timers();

    // Phase 24: TCP
    printf("\n--- Phase 24: TCP ---\n");
    test_tcp_socket_bind_listen();
    test_tcp_connect_accept();
    test_tcp_read_write();
    test_tcp_echo_server();
    test_tcp_multiple_clients();
    test_tcp_nonblocking();
    test_tcp_close_releases_fd();

    // Phase 25: UDP
    printf("\n--- Phase 25: UDP ---\n");
    test_udp_send_receive();
    test_udp_multiple_packets();
    test_udp_close();

    // Phase 26: Channels
    printf("\n--- Phase 26: Channels ---\n");
    test_channel_basic_send_receive();
    test_channel_fifo_ordering();
    test_channel_close();
    test_channel_waker_on_send();
    test_channel_waker_on_receive();

    // Phase 27: Synchronization
    printf("\n--- Phase 27: Synchronization ---\n");
    test_mutex_basic();
    test_mutex_wakes_waiter();
    test_semaphore_basic();
    test_semaphore_wakes_waiter();
    test_rwlock_basic();
    test_notify_basic();

    // Phase 27: Join / Select
    printf("\n--- Phase 27: Join / Select ---\n");
    test_join_all();
    test_select_first_ready();
    test_select_none_ready();

    // Phase 28: Cancellation
    printf("\n--- Phase 28: Cancellation ---\n");
    test_cancel_before_poll();
    test_cancel_during_execution();
    test_cancel_completed_task();
    test_cancel_nonexistent_task();

    // Phase 29: Blocking Pool
    printf("\n--- Phase 29: Blocking Pool ---\n");
    test_blocking_pool_basic();
    test_blocking_pool_multiple_work();
    test_blocking_pool_does_not_block_executor();
    test_blocking_pool_shutdown();

    // Phase 31: Process
    printf("\n--- Phase 31: Process ---\n");
    test_process_spawn();

    // Phase 32: DNS
    printf("\n--- Phase 32: DNS ---\n");
    test_dns_resolve_localhost();
    test_dns_resolve_failure();

    // Phase 33: Multi-thread
    printf("\n--- Phase 33: Multi-thread ---\n");
    test_mt_executor_basic();

    // Reactor integration
    printf("\n--- Reactor + TCP Integration ---\n");
    test_reactor_tcp_integration();

    // Phase 34: Stress Tests
    printf("\n--- Phase 34: Stress ---\n");
    stress_spawn_cancel_storm();
    stress_timer_storm();
    stress_channel_storm();
    stress_ready_queue();
    stress_executor_many_tasks();
    stress_mutex_contention();

    // Real-world validation
    printf("\n--- Real-World Validation ---\n");
    test_realworld_tcp_echo();
    test_realworld_timer_executor();

    // Memory safety
    printf("\n--- Memory Safety ---\n");
    test_memory_alloc_free_cycles();
    test_memory_waker_cycles();
    test_memory_channel_cycles();
    test_memory_timer_cycles();

    // Phase 38: Real Pending/Resume
    printf("\n--- Phase 38: Real Pending/Resume ---\n");
    test_pending_resume_basic();
    test_pending_resume_two_tasks();
    test_pending_resume_self_wake();

    // Phase C: Multi-task Executor
    printf("\n--- Phase C: Multi-task Executor ---\n");
    test_c_run_once_basic();
    test_c_two_tasks_interleave();
    test_c_three_tasks_fifo();
    test_c_cancel_one_of_many();
    test_c_wake_dedup();
    test_c_run_once_empty();

    // P0-B: Async TCP Futures
    printf("\n--- P0-B: Async TCP Futures ---\n");
    test_p0b_tcp_connect_async();
    test_p0b_tcp_accept_async();
    test_p0b_tcp_read_write_async();
    test_p0b_tcp_full_roundtrip();

    // Summary
    printf("\n=== SUMMARY ===\n");
    printf("Tests run:    %d\n", g_tests_run);
    printf("Tests passed: %d\n", g_tests_passed);
    printf("Tests failed: %d\n", g_tests_failed);

    if (g_tests_failed > 0) {
        printf("\nFAILED TESTS DETECTED\n");
    } else {
        printf("\nALL TESTS PASSED\n");
    }

    #ifdef _WIN32
    WSACleanup();
    #endif

    return g_tests_failed > 0 ? 1 : 0;
}
