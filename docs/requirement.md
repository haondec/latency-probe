# Documentation

:eight_spoked_asterisk: Strategy Comparison
| Capability            | Case 1: No Agent on Target      | Case 2: Agent on Target                                                 |
| --------------------- | ------------------------------- | ----------------------------------------------------------------------- |
| RTT (round trip)      | ✅ Yes                           | ✅ Yes                                                                   |
| One-way latency (µs)  | ❌ No (need PTP both ends)       | ✅ Yes                                                                   |
| Precision             | ms to 100µs (depends)           | µs possible (hw timestamp)                                              |
| External services     | ✅ Works                         | ❌ Not possible                                                          |
| Control granularity   | Low                             | High (break down app, net, kernel)                                      |
| Deployment complexity | Low                             | Medium–High                                                             |
| Trading use-case      | Basic monitoring (exchange RTT) | True latency tracking between your own nodes (critical for algo tuning) |

```text
The challenge: if you probe many hosts, you don’t 
want scheduling drift (jitter) or one slow probe delaying others.
```

## Techniques

### Concurrency model

Use event loop or worker pool.

Each probe target gets its own goroutine (Go) or async task (Python asyncio, Rust tokio).

Use monotonic clock (CLOCK_MONOTONIC_RAW in Linux) for timing.

### Coordinated scheduling

Maintain a fixed interval scheduler (like a metronome).

Example: if probe interval = 1s, always fire at t=0s, 1s, 2s…, not "1s after last probe finished".

Add small random jitter (e.g., ±50ms) per target to avoid synchronized bursts if you have thousands of targets.

### Probe timeouts

Run probes with context timeout (Go context.WithTimeout, Python asyncio.wait_for).

If probe doesn’t finish within timeout, cancel task and increment probe_timeout_total.

### Avoiding scheduler drift

Don’t use naive time.sleep(interval) loops.

Use "absolute schedule":

next_time = start_time + N*interval
sleep_until(next_time)

This ensures consistent spacing even if probes take longer.

### Clock source

Use CLOCK_MONOTONIC_RAW for probe timestamps (not CLOCK_REALTIME, which is subject to NTP/PTP adjustments).

For one-way latency (if you deploy agents both ends): still use CLOCK_MONOTONIC_RAW locally, but synchronize via PTP for alignment.

## Programming Language

:eight_spoked_asterisk: Go and GC

Go’s garbage collector is concurrent and pause times are very low (<1ms on modern versions).

For interval probes at ≥1ms granularity, GC noise is usually negligible.

But: if you’re chasing sub-millisecond consistency (say, 50µs vs 500µs), the GC can introduce rare but noticeable spikes.

That’s why Blackbox Exporter (Go) is "good enough" for general infra monitoring, but not necessarily for quant-grade timing.

:eight_spoked_asterisk: Rust (no GC)

And in this sample, personally I prefer Rust (try something new)

✅ Pros

* Zero GC → no runtime pauses at all.

* Deterministic performance — you fully control allocations.

* Nanosecond timers (std::time::Instant maps to CLOCK_MONOTONIC_RAW).

* System-level control — you can pin threads to CPU cores, set real-time scheduling, even use kernel bypass (DPDK) if you go extreme.

* Memory safety → avoids C-style footguns while staying performant.

⚠️ Cons

* Higher development cost (more verbose, steep learning curve).

* Ecosystem is younger — fewer pre-built exporters/libraries compared to Go.

* For simple "probe N hosts every second," Go already works very well.

:eight_spoked_asterisk: When to Prefer Rust

* Probes need ultra-low jitter (tens of µs level, not ms).

* Measure exchange gateway round-trips or colocated FIX/REST latencies where every µs counts.

* Run thousands of probes per second with zero GC interference.

* If you might later integrate with kernel-bypass networking (DPDK, io_uring, etc.).

:eight_spoked_asterisk: Practical Trade-Off

Go is a sweet spot for "infra-style monitoring":

* <1ms jitter.

* Easier integration with Prometheus, exporters.

* Faster development.

Rust shines if you need "quant-style monitoring":

* Jitter consistently <100µs.

* Deterministic timings, no GC.

* Ability to squeeze every µs from NIC to userland.

✅ TL;DR:

For most monitoring setups (Prometheus, Grafana, crypto exchange health checks) → Go is plenty.

For quant trading latency probes (where you want to see if your order router is 50µs slower than yesterday) → Rust is the safer long-term bet.
