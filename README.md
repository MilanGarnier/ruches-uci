# Ruches-Chess

Ruches-Chess is a Rust Chess engine project. The goal is to implement a relatively optimized multithreaded chess engine.

## Features

- Ongoing UCI Interface
    - Set a custom position - OK
    - Perft - OK
    - Analyze with different budgets (time|nodes|depth) - TODO

- Legal move generation - OK

- Transposition tables - Ongoing

- Alpha-beta search - TODO

- MiniMax - OK

## Get started

### Build
```bash
rustup override set nightly
cargo build --release
```
### Run
```bash
cargo run --release
```

### Example UCI input
```
position fen r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 0
go perft 5
```

This will give the following output
```
d2c1: 3793390
d2e3: 4407041
d2f4: 3941257
d2g5: 4370915
d2h6: 3967365
e2d1: 3074219
e2f1: 4095479
e2d3: 4066966
e2c4: 4182989
e2b5: 4032348
e2a6: 3553501
f3d3: 3949570
f3e3: 4477772
f3g3: 4669768
f3h3: 5067173
f3f4: 4327936
f3g4: 4514010
f3f5: 5271134
f3h5: 4743335
f3f6: 3975992
a1b1: 3827454
a1c1: 3814203
a1d1: 3568344
h1f1: 3685756
h1g1: 3989454
c3b1: 3996171
c3d1: 3995761
c3a4: 4628497
c3b5: 4317482
e5d3: 3288812
e5c4: 3494887
e5g4: 3415992
e5c6: 4083458
e5g6: 3949417
e5d7: 4404043
e5f7: 4164923
a2a3: 4627439
a2a4: 4387586
b2b3: 3768824
g2g3: 3472039
g2h3: 3819456
g2g4: 3338154
d5d6: 3835265
d5e6: 4727437
e1d1: 3559113
e1f1: 3377351
e1g1: 4119629
e1c1: 3551583

Nodes searched : 193690690
```

### Unit tests (ongoing)
```bash
cargo test
```
