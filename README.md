
```
cargo run
```

will generate many variants and expose them to a js expression but there is a memory leak

The memory is only cleared when the program is done (after "done" is printed).

Here is the final GC diagnostic output:

```
[543451:0x649847966000]    15393 ms: Scavenge 123.5 (127.6) -> 122.8 (132.1) MB, pooled: 0 MB, 38.04 / 0.00 ms  (average mu = 0.100, current mu = 0.000) allocation failure; 
[543451:0x649847966000] Memory allocator,       used: 135296 KB, available: 1347456 KB
[543451:0x649847966000] Read-only space,        used:    114 KB, available:      0 KB, committed:    256 KB
[543451:0x649847966000] New space,              used:    836 KB, available:   3259 KB, committed:   8192 KB
[543451:0x649847966000] New large object space, used:      0 KB, available:   4096 KB, committed:      0 KB
[543451:0x649847966000] Old space,              used: 124492 KB, available:    123 KB, committed: 126012 KB
[543451:0x649847966000] Code space,             used:      0 KB, available:    255 KB, committed:    256 KB
[543451:0x649847966000] Large object space,     used:    256 KB, available:      0 KB, committed:    260 KB
[543451:0x649847966000] Code large object space,     used:      0 KB, available:      0 KB, committed:      0 KB
[543451:0x649847966000] Trusted space,              used:    201 KB, available:    258 KB, committed:    460 KB
[543451:0x649847966000] Trusted large object space,     used:      0 KB, available:      0 KB, committed:      0 KB
[543451:0x649847966000] All spaces,             used: 125900 KB, available: 1355449 KB, committed: 135436 KB
[543451:0x649847966000] Pool buffering 0 chunks of committed:      0 KB
[543451:0x649847966000] External memory reported: 268352 KB
[543451:0x649847966000] Backing store memory:      0 KB
[543451:0x649847966000] External memory global 0 KB
[543451:0x649847966000] Total time spent in GC  : 10929.6 ms
```

The process is using about 2.4GB of memory by the end.