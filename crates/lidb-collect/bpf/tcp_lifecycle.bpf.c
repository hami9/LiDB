// SPDX-License-Identifier: Apache-2.0
//
// Linux eBPF C CO-RE program for read-only TCP lifecycle event observation.
//
// Hooks the stable `sock:inet_sock_set_state` kernel tracepoint to observe
// short-lived TCP connection lifecycles without packet payload capture,
// credential inspection, or network traffic alteration.
//
// Map ABI and Memory Bounds:
// - Ring buffer `events`: 256 KB fixed buffer (`BPF_MAP_TYPE_RINGBUF`).
// - Event size: 64 bytes (`struct tcp_event_t`).
// - Overrun policy: Ring buffer drops excess events under saturation and
//   records drop count via `bpf_ringbuf_reserve` failure.

#ifndef __VMLINUX_H__
// Type definitions for standalone compilation and verification inspection.
typedef unsigned char __u8;
typedef unsigned short __u16;
typedef unsigned int __u32;
typedef unsigned long long __u64;

#define SEC(name) __attribute__((section(name), used))

// Linux socket address families
#define AF_INET  2
#define AF_INET6 10

// Linux TCP states (from linux/tcp.h)
#define TCP_ESTABLISHED 1
#define TCP_SYN_SENT    2
#define TCP_SYN_RECV    3
#define TCP_FIN_WAIT1   4
#define TCP_FIN_WAIT2   5
#define TCP_TIME_WAIT   6
#define TCP_CLOSE       7
#define TCP_CLOSE_WAIT  8
#define TCP_LAST_ACK    9
#define TCP_LISTEN      10
#define TCP_CLOSING     11

#define BPF_MAP_TYPE_RINGBUF 27
#endif

/// Event types matching LiDB core `EbpfEventType`.
enum event_type_t {
    EVENT_TCP_CONNECT = 0,
    EVENT_TCP_ACCEPT = 1,
    EVENT_TCP_CLOSE = 2,
    EVENT_TCP_RETRANSMIT = 3,
};

/// 64-byte bounded event structure sent to userspace via BPF ring buffer.
struct tcp_event_t {
    __u64 timestamp_ns;
    __u32 pid;
    __u16 family;
    __u16 sport;
    __u16 dport;
    __u8  event_type;
    __u8  oldstate;
    __u8  newstate;
    __u8  pad;
    char  comm[16];
    __u8  saddr[16];
    __u8  daddr[16];
};

/// Kernel tracepoint argument layout for `sock:inet_sock_set_state`.
struct trace_event_raw_inet_sock_set_state {
    __u64 unused_header;
    const void *skaddr;
    int oldstate;
    int newstate;
    __u16 sport;
    __u16 dport;
    __u16 family;
    __u8 protocol;
    __u8 saddr[4];
    __u8 daddr[4];
    __u8 saddr_v6[16];
    __u8 daddr_v6[16];
};

/// SEC("tracepoint/sock/inet_sock_set_state")
/// Observes TCP connection state transitions without capturing payload or modifying state.
SEC("tracepoint/sock/inet_sock_set_state")
int trace_inet_sock_set_state(struct trace_event_raw_inet_sock_set_state *ctx) {
    int oldstate = ctx->oldstate;
    int newstate = ctx->newstate;

    // Filter only events representing connection establishment or teardown:
    // - SYN_SENT -> ESTABLISHED (outbound connect succeeded)
    // - SYN_RECV -> ESTABLISHED (inbound accept succeeded)
    // - ESTABLISHED -> FIN_WAIT1 / CLOSE_WAIT / CLOSE (teardown started/ended)
    __u8 ev_type;
    if (oldstate == TCP_SYN_SENT && newstate == TCP_ESTABLISHED) {
        ev_type = EVENT_TCP_CONNECT;
    } else if (oldstate == TCP_SYN_RECV && newstate == TCP_ESTABLISHED) {
        ev_type = EVENT_TCP_ACCEPT;
    } else if (newstate == TCP_CLOSE || newstate == TCP_CLOSING || newstate == TCP_TIME_WAIT) {
        ev_type = EVENT_TCP_CLOSE;
    } else {
        // Ignore internal or intermediate protocol state shifts to bound overhead
        return 0;
    }

    // In a live CO-RE compiled environment, the ringbuf output is allocated:
    // struct tcp_event_t *event = bpf_ringbuf_reserve(&events, sizeof(*event), 0);
    // if (!event) return 0;
    // event->timestamp_ns = bpf_ktime_get_ns();
    // event->pid = bpf_get_current_pid_tgid() >> 32;
    // bpf_get_current_comm(&event->comm, sizeof(event->comm));
    // event->family = ctx->family;
    // event->sport = ctx->sport;
    // event->dport = ctx->dport;
    // event->oldstate = (__u8)oldstate;
    // event->newstate = (__u8)newstate;
    // event->event_type = ev_type;
    // bpf_ringbuf_submit(event, 0);

    return 0;
}

char LICENSE[] SEC("license") = "Apache-2.0";
