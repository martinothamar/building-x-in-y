# HTTP server

In this project I experiment with various techniques, architectures and infrastructure (on Linux)
to make fast HTTP 1.1 servers. I'm using the Rust `may-minihttp` microframework as a baseline
and trying to make something faster. `may-minihttp` ended up on [2nd place in the plaintext Techempower benchmarks
during round 21](https://www.techempower.com/benchmarks/#section=data-r21&test=plaintext).

Here I'll simply loadtest the servers using [wrk2 by giltene](https://github.com/giltene/wrk2) on my desktop machine
to observe latency distribution and throughput and use that as a tool to learn more about Linux, HTTP etc etc...

This will always be a work in progress/playground for stuff I want to try.

Current projects under testing
* [rust-baseline](/http-server/rust-baseline)
  * `may-minihttp` based server, copied from the [Techempower benchmark source](https://github.com/TechEmpower/FrameworkBenchmarks/tree/17b7ef209d2c3a16d2f687ee0a2108f846df223a/frameworks/Rust/may-minihttp).
* [rust](/http-server/rust)
  * My server based on IO uring using a thread-per-core architecture (eventloop per thread - no runtime, no async)

This is never going to be a fair comparison, as my implementation is not a real server.
Just an excuse to try out new things and theories.

## My hardware

```bash
$ uname -a
Linux desktop 6.2.6-76060206-generic #202303130630~1689015125~22.04~ab2190e SMP PREEMPT_DYNAMIC Mon J x86_64 x86_64 x86_64 GNU/Linux
$ lspci | egrep -i --color 'network|ethernet'
08:00.0 Ethernet controller: Intel Corporation I211 Gigabit Network Connection (rev 03)
$ lscpu
Architecture:            x86_64
  CPU op-mode(s):        32-bit, 64-bit
  Address sizes:         48 bits physical, 48 bits virtual
  Byte Order:            Little Endian
CPU(s):                  12
  On-line CPU(s) list:   0-11
Vendor ID:               AuthenticAMD
  Model name:            AMD Ryzen 5 5600X 6-Core Processor
    CPU family:          25
    Model:               33
    Thread(s) per core:  2
    Core(s) per socket:  6
    Socket(s):           1
    Stepping:            0
    Frequency boost:     enabled
    CPU max MHz:         4650.2920
    CPU min MHz:         2200.0000
    BogoMIPS:            7385.58
    Flags:               fpu vme de pse tsc msr pae mce cx8 apic sep mtrr pge mca cmov pat pse36 clflush mmx fxsr sse sse2 ht syscall nx mmxext fxsr_opt
                         pdpe1gb rdtscp lm constant_tsc rep_good nopl nonstop_tsc cpuid extd_apicid aperfmperf rapl pni pclmulqdq monitor ssse3 fma cx16
                         sse4_1 sse4_2 movbe popcnt aes xsave avx f16c rdrand lahf_lm cmp_legacy svm extapic cr8_legacy abm sse4a misalignsse 3dnowprefet
                         ch osvw ibs skinit wdt tce topoext perfctr_core perfctr_nb bpext perfctr_llc mwaitx cpb cat_l3 cdp_l3 hw_pstate ssbd mba ibrs ib
                         pb stibp vmmcall fsgsbase bmi1 avx2 smep bmi2 erms invpcid cqm rdt_a rdseed adx smap clflushopt clwb sha_ni xsaveopt xsavec xget
                         bv1 xsaves cqm_llc cqm_occup_llc cqm_mbm_total cqm_mbm_local clzero irperf xsaveerptr rdpru wbnoinvd arat npt lbrv svm_lock nrip
                         _save tsc_scale vmcb_clean flushbyasid decodeassists pausefilter pfthreshold avic v_vmsave_vmload vgif v_spec_ctrl umip pku ospk
                         e vaes vpclmulqdq rdpid overflow_recov succor smca fsrm
Virtualization features:
  Virtualization:        AMD-V
Caches (sum of all):
  L1d:                   192 KiB (6 instances)
  L1i:                   192 KiB (6 instances)
  L2:                    3 MiB (6 instances)
  L3:                    32 MiB (1 instance)
```

## Methodology

It's certainly harder to run these kinds of load test as opposed to microbenchmarks.
I try to run my hardware as idle as possible, but there is alawys going to be some noise.
I pin `wrk` to 4 specific threads, and I also pin the servers to specific threads (if I'm able).
There is no other tuning done. NIC, kernel etc uses whatever CPUs it wants and scheduling priority are all defaults.

## Latency

### Baseline

```bash
$ taskset -c 8-11 wrk -t4 -c1000 -d15s -R100000 --latency http://127.0.0.1:8080
Running 15s test @ http://127.0.0.1:8080
  4 threads and 1000 connections
  Thread calibration: mean lat.: 1.418ms, rate sampling interval: 10ms
  Thread calibration: mean lat.: 1.424ms, rate sampling interval: 10ms
  Thread calibration: mean lat.: 1.406ms, rate sampling interval: 10ms
  Thread calibration: mean lat.: 1.428ms, rate sampling interval: 10ms
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency     1.41ms  582.55us   9.78ms   69.90%
    Req/Sec    26.38k     3.78k   40.33k    82.41%
  Latency Distribution (HdrHistogram - Recorded Latency)
 50.000%    1.35ms
 75.000%    1.78ms
 90.000%    2.15ms
 99.000%    2.79ms
 99.900%    4.70ms
 99.990%    8.27ms
 99.999%    8.94ms
100.000%    9.78ms

#[Mean    =        1.405, StdDeviation   =        0.583]
#[Max     =        9.776, Total count    =       375000]
#[Buckets =           27, SubBuckets     =         2048]
----------------------------------------------------------
  1438076 requests in 15.00s, 172.80MB read
Requests/sec:  95861.78
Transfer/sec:     11.52MB
```

### My server

```bash
$ taskset -c 8-11 wrk -t4 -c1000 -d15s -R100000 --latency http://127.0.0.1:8081
Running 15s test @ http://127.0.0.1:8081
  4 threads and 1000 connections
  Thread calibration: mean lat.: 1.372ms, rate sampling interval: 10ms
  Thread calibration: mean lat.: 1.353ms, rate sampling interval: 10ms
  Thread calibration: mean lat.: 1.410ms, rate sampling interval: 10ms
  Thread calibration: mean lat.: 1.339ms, rate sampling interval: 10ms
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency     1.38ms  575.84us   4.62ms   66.34%
    Req/Sec    26.31k     3.47k   42.44k    83.40%
  Latency Distribution (HdrHistogram - Recorded Latency)
 50.000%    1.34ms
 75.000%    1.78ms
 90.000%    2.13ms
 99.000%    2.83ms
 99.900%    3.27ms
 99.990%    3.90ms
 99.999%    4.48ms
100.000%    4.62ms

#[Mean    =        1.377, StdDeviation   =        0.576]
#[Max     =        4.616, Total count    =       374995]
#[Buckets =           27, SubBuckets     =         2048]
----------------------------------------------------------
  1427928 requests in 15.00s, 100.77MB read
Requests/sec:  95186.12
Transfer/sec:      6.72MB
```

## Topics to explore

* Tuning the kernel to improve results and methodology
  * https://rigtorp.se/low-latency-guide/
* Different lanuages
* Thread per core (shared nothing) vs workstrealing schedulers
* IOUring vs epoll vs ...
* [TigerStyle](https://github.com/tigerbeetle/tigerbeetle/blob/787485820188fb74ac08e07a63c87be41344ea8b/docs/TIGER_STYLE.md)?

