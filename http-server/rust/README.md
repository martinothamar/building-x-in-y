# HTTP server in Rust

My goal is to create a fast HTTP server implementation based on modern architecture

* IO uring
* Thread-per-core architecture

## Setup

```sh
$ uname -a
Linux Desktop 6.1.21.2-microsoft-standard-WSL2 #1 SMP Sun Jul 30 12:35:27 CEST 2023 x86_64 x86_64 x86_64 GNU/Linux
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
...
```

## Loadtest

```sh
$ wrk -t3 -c9 -d15s -R100 --latency http://127.0.0.1:8080
Running 15s test @ http://127.0.0.1:8080
  3 threads and 9 connections
  Thread calibration: mean lat.: 0.910ms, rate sampling interval: 10ms
  Thread calibration: mean lat.: 1.008ms, rate sampling interval: 10ms
  Thread calibration: mean lat.: 0.742ms, rate sampling interval: 10ms
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency   655.01us  359.11us   2.07ms   73.54%
    Req/Sec    33.95     67.29   222.00     87.20%
  Latency Distribution (HdrHistogram - Recorded Latency)
 50.000%  618.00us
 75.000%    0.88ms
 90.000%    1.07ms
 99.000%    1.72ms
 99.900%    2.07ms
 99.990%    2.07ms
 99.999%    2.07ms
100.000%    2.07ms

  Detailed Percentile spectrum:
       Value   Percentile   TotalCount 1/(1-Percentile)

       0.080     0.000000            1         1.00
       0.235     0.100000           50         1.11
       0.345     0.200000           99         1.25
       0.423     0.300000          150         1.43
       0.502     0.400000          199         1.67
       0.618     0.500000          248         2.00
       0.651     0.550000          273         2.22
       0.691     0.600000          297         2.50
       0.748     0.650000          322         2.86
       0.828     0.700000          347         3.33
       0.882     0.750000          372         4.00
       0.900     0.775000          384         4.44
       0.929     0.800000          396         5.00
       0.963     0.825000          409         5.71
       0.991     0.850000          422         6.67
       1.031     0.875000          434         8.00
       1.053     0.887500          440         8.89
       1.068     0.900000          447        10.00
       1.085     0.912500          452        11.43
       1.101     0.925000          458        13.33
       1.137     0.937500          465        16.00
       1.199     0.943750          468        17.78
       1.427     0.950000          471        20.00
       1.511     0.956250          474        22.86
       1.535     0.962500          477        26.67
       1.566     0.968750          480        32.00
       1.572     0.971875          482        35.56
       1.586     0.975000          483        40.00
       1.639     0.978125          485        45.71
       1.658     0.981250          486        53.33
       1.676     0.984375          488        64.00
       1.686     0.985938          489        71.11
       1.686     0.987500          489        80.00
       1.717     0.989062          490        91.43
       1.759     0.990625          491       106.67
       1.817     0.992188          492       128.00
       1.817     0.992969          492       142.22
       1.817     0.993750          492       160.00
       1.992     0.994531          493       182.86
       1.992     0.995313          493       213.33
       2.051     0.996094          494       256.00
       2.051     0.996484          494       284.44
       2.051     0.996875          494       320.00
       2.051     0.997266          494       365.71
       2.051     0.997656          494       426.67
       2.069     0.998047          495       512.00
       2.069     1.000000          495          inf
#[Mean    =        0.655, StdDeviation   =        0.359]
#[Max     =        2.068, Total count    =          495]
#[Buckets =           27, SubBuckets     =         2048]
----------------------------------------------------------
  1506 requests in 15.03s, 108.83KB read
Requests/sec:    100.19
Transfer/sec:      7.24KB
```
