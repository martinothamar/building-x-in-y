# Monte Carlo simulation

Some years ago in a previous job, I was working on systems that created probabilities for football matches, so that the company I worked for could both
place bets, and sell these probabilities to others in the industry (bookies etc). One fun challenge was generating probabilities for outright markets for different leagues,
where the canonical market example is this:

> Who will win Premier League the upcoming or current season?

We already had ratings for all teams in the form of **expected goals** which is the number of goals we expect from a team in a match (in some league/season).
We then used the [Monte Carlo method](https://en.wikipedia.org/wiki/Monte_Carlo_method) to simulate all the matches (or remaining matches) of a season.

The expected goals rating was used feeding the Lambda variable (`L = exp(−expected_goals)`) when sampling from a [poisson distribution](https://en.wikipedia.org/wiki/Poisson_distribution) using Knuth's algorithm (appropriate for small L's). Before extracting the probabilities for outright markets, we typically simulated all the matches of a season 100'000 times. The system then counted how many times each team won the season, and a whole bunch of other metrics that we could build market probabilities from.

> **Note**
> This project will be updated over time as I experiment and learn new things. I'll keep the benchmarking, stats and profiles up to date as I change the implementation. The algorithms, datastructures and code in general with be documented inline with the code in the [/rust-folder](/monte-carlo-sim/rust/).

## Previous implementations

The first implementation I saw of this system was written all in F#, as a funtional language is a good fit for statistical and mathematical code. That is probably true, but as we soon found out, idiomatic F# (and functional code in general) is often very poor for performance. The first naive implemenation used standard F# collections such as seq's and map's which all have copy-on-write semantics and are immutable. If I recall correctly, these collections are modelled as trees to accomodate these functional patterns. So these early versions had a _ton_ of allocations and memory copies. At some point, probabilities generation using 100K simulations was allocating something like 2GiB of RAM and taking ~60 seconds...

After some optimization passes the F# codebase didn't really look all that functional anymore. Arrays were used in place of seq's and the code was generally a lot more aware of the datastructures used and where allocations would occur. The algorithms were improved as well. Still, the runtime of 100K simulations of a Premier League season was still on the order of 10s of seconds. I don't remember if this was parallelized across cores or not..

## The ideal implementation

Now for this repo, what I have tried is to implement the "ideal implementation".
A solution that can do 100K simulations and extract probabilities in as little time as possible for my hardware.

```bash
$ lscpu
Architecture:            x86_64
  CPU op-mode(s):        32-bit, 64-bit
  Address sizes:         39 bits physical, 48 bits virtual
  Byte Order:            Little Endian
CPU(s):                  16
  On-line CPU(s) list:   0-15
Vendor ID:               GenuineIntel
  Model name:            11th Gen Intel(R) Core(TM) i7-11800H @ 2.30GHz
    CPU family:          6
    Model:               141
    Thread(s) per core:  2
    Core(s) per socket:  8
    Socket(s):           1
    Stepping:            1
    BogoMIPS:            4608.00
    Flags:               fpu vme de pse tsc msr pae mce cx8 apic sep mtrr pge mca cmov pat pse36 clflush mmx fxsr sse sse2 ss ht syscall nx pdpe1gb rdtscp lm constant_tsc arch_perfmon rep_good nopl
                         xtopology tsc_reliable nonstop_tsc cpuid pni pclmulqdq vmx ssse3 fma cx16 pdcm pcid sse4_1 sse4_2 x2apic movbe popcnt tsc_deadline_timer aes xsave avx f16c rdrand hypervisor
                          lahf_lm abm 3dnowprefetch invpcid_single ssbd ibrs ibpb stibp ibrs_enhanced tpr_shadow vnmi ept vpid ept_ad fsgsbase tsc_adjust bmi1 avx2 smep bmi2 erms invpcid avx512f avx
                         512dq rdseed adx smap avx512ifma clflushopt clwb avx512cd sha_ni avx512bw avx512vl xsaveopt xsavec xgetbv1 xsaves avx512vbmi umip avx512_vbmi2 gfni vaes vpclmulqdq avx512_vn
                         ni avx512_bitalg avx512_vpopcntdq rdpid movdiri movdir64b fsrm avx512_vp2intersect flush_l1d arch_capabilities
Virtualization features:
  Virtualization:        VT-x
  Hypervisor vendor:     Microsoft
  Virtualization type:   full
Caches (sum of all):
  L1d:                   384 KiB (8 instances)
  L1i:                   256 KiB (8 instances)
  L2:                    10 MiB (8 instances)
  L3:                    24 MiB (1 instance)
```

There is more than enough L1 cache space for what is needed (there is a unit-test verifying that), and there is AVX512 support. Earlier generations of AVX512 support in CPUs have been questionable due to lots of thermal throttling issues, but apparantly in these newer CPUs it shouldn't be a problem (after IceLake?), so I will try to make use AVX512 hardware intrinsics. This is a laptop, so I don't have perfect testing/benchmarking conditions here.

In the real world, calculating the expected goals ratings is very complex as you want to account for player form, injuries, travel time to the match and a whole bunch of stuff. But for this dummy implementation, where I really just want to play around with low level optimization, we just set expected goals as the average goals of the previous season. I extracted these from some tables on Wikipedia using some JavaScript in the chrome inspector...

Main challenges
* Cache locality and memory layout
  * Having all data fit in CPU L1 cache is tables stakes, so we need to choose appropriately sized and laid out datastructures so that data both fits in cache and is close together (although that is less important when it all fits in cache).
* Branch prediction
  * For each goal we are sampling a random floating point number from a poisson distribution. The CPU branch predictor now has to do a bunch of guesses and will fail a lot, leading to branch misses and messing up the instruction pipeline.

Constraints
* No parallelization across cores
  * In the real world this is usually done, but I just want to squeeze perf out of a single core for this solutions

## Results

I have implemented the simulation part of the system, and extraction of market probabilities for two markets - Winner and Top4.
The resulting solution is written in Rust in the [`rust`-folder](/monte-carlo-sim/rust/).
Benchmarks and perf counter stats are reported below. The generated code is pretty good,
and largely utilizes zmm-registers (which are the 512bit wide ones that can contain 8 lanes of 64bit numbers) in the the hot loop.
Use `make dasm` to inspect the generated code.

As part of getting here, I built [simd-rand](https://github.com/martinothamar/simd-rand), a Rust library containing
vectorized implementations of (already) fast PRNGs. I found Xoshiro256+ to be the fastest generator with good enough statistical properties (I think)
for my usecase. As soon as the RNG data was vectorized, I started vectorizing the whole simulation.
In the end the solution simulates 4 matches at the same time. Vectorizaion is great on many levels. There is the efficient parallelization, but there is also the
amortization (and elimination) of branches (the innermost branch is executed once per 8 goal simulations instead of 1 goal simulation so to speak),
which noticably impacts branch misprediction % reported from `perf stat`.

### Benchmark

During benchmarking, we simulate only 1'000 times, which lets us complete is a couple of milliseconds.
It's generally good practice to keep the latency of benchmarks low (for example to keep within an OS scheduler timeslice)

```
Simulation/simulation 1_000
                        time:   [2.5403 ms 2.5489 ms 2.5573 ms]
                        thrpt:  [391.04 Kelem/s 392.33 Kelem/s 393.66 Kelem/s]
Found 1 outliers among 100 measurements (1.00%)
  1 (1.00%) high mild
mean   [2.5403 ms 2.5573 ms] std. dev.      [37.783 µs 49.340 µs]
median [2.5497 ms 2.5670 ms] med. abs. dev. [26.345 µs 51.321 µs]
```

### Perf stats

```
 Performance counter stats for '../../target/release/monte-carlo-sim':

           8295.60 msec task-clock:u              #    0.999 CPUs utilized
                 0      context-switches:u        #    0.000 /sec
                 0      cpu-migrations:u          #    0.000 /sec
               111      page-faults:u             #   13.381 /sec
       35149873093      cycles:u                  #    4.237 GHz                      (45.24%)
       45128122355      instructions:u            #    1.28  insn per cycle           (54.40%)
        2855347801      branches:u                #  344.200 M/sec                    (63.56%)
         441162510      branch-misses:u           #   15.45% of all branches          (72.72%)
      174013920400      slots:u                   #   20.977 G/sec                    (81.88%)
       26704121408      topdown-retiring:u        #     13.3% retiring                (81.88%)
      155588500464      topdown-bad-spec:u        #     77.6% bad speculation         (81.88%)
        1773971873      topdown-fe-bound:u        #      0.9% frontend bound          (81.88%)
       16521169391      topdown-be-bound:u        #      8.2% backend bound           (81.88%)
        1839627062      L1-dcache-loads:u         #  221.759 M/sec                    (81.92%)
            432728      L1-dcache-load-misses:u   #    0.02% of all L1-dcache accesses  (81.92%)
   <not supported>      LLC-loads:u
   <not supported>      LLC-load-misses:u
   <not supported>      L1-icache-loads:u
            967885      L1-icache-load-misses:u                                       (81.92%)
        1837218912      dTLB-loads:u              #  221.469 M/sec                    (81.93%)
              8047      dTLB-load-misses:u        #    0.00% of all dTLB cache accesses  (36.16%)
   <not supported>      iTLB-loads:u
             74548      iTLB-load-misses:u                                            (36.16%)
   <not supported>      L1-dcache-prefetches:u
   <not supported>      L1-dcache-prefetch-misses:u

       8.300780891 seconds time elapsed

       8.295941000 seconds user
       0.000000000 seconds sys
```

## Flamegraph

About 70% of the time is spent simulating the matches (sampling goals from poisson distribution).
**[Open the raw file to inspect the flamegraph](https://raw.githubusercontent.com/martinothamar/building-x-in-y/main/monte-carlo-sim/rust/flamegraph.svg)**

![Flamegraph from `cargo flamegraph`](/monte-carlo-sim/rust/flamegraph.svg)

## Next steps?

I don't know here to go next, but I have some ideas that could be investigated

* Deeper analysis of the performance of the vectorized instructions used in the hot loop ([read up on Agner Fog's instruction tables](https://www.agner.org/optimize/instruction_tables.pdf))
* GPU parallelization (I did build a CUDA kernel prototype of this once, but it wasn't as fast. I'm probably bad at GPU)
* CPU parallelization (doubt this is good)
* Other algorithms?
