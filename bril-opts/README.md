# Bril Optimizations
## Implementation of Local Value Numbering and Dead Code Elimination 

This implementation of LVN/TDCE uses the ```bril-rs``` library to parse bril JSON into a Rust representation. It then performes LVN/TDCE on the parsed program and prints the result to stdout.

### Usage
To run the optimizations, use 
```cargo run --release -- <path-to-bril-json> [--lvn] [--tdce]``` 

The ```--lvn``` and ```--tdce``` flags are optional. If both are specified, both optimizations will be run (first applying LVN, then TDCE). If only one is specified, only that optimization will be run. If neither is specified, the program will simply print the parsed program to stdout.

### Tests
Testing is implemented using the ```brench``` tool provided with ```bril```. To run the tests, simply use ```./test.sh```. This will report the results (avg, stddev, min, max of relative change in dyn instruction count) of each optimization on ```benchmarks```.

Note: Timeout/Incorrect results will be reported but ignored in the final statistics. 

## Control Flow Graphs and Liveness Analysis

This implementation of CFG and Liveness Analysis uses the ```bril-rs``` library to parse bril JSON into a Rust representation. It then constructs a CFG using ```petgraph``` and performs liveness analysis using on the parsed program and prints the result to stdout. It can be used in tandem in the LVN/TDCE flags. 


### Usage 
To view the liveness analysis of a program, use
```cargo run --release -- <path-to-bril-json> [--lvn] [--tdce] [--dom] [--cfg] --liveness``` 

To view the dominator analysis of a program, use
```cargo run --release -- <path-to-bril-json> [--lvn] [--tdce] [--cfg] [--liveness ]--dom``` 

To view the constructed CFG of a program (graphviz), use
```cargo run --release -- <path-to-bril-json> [--lvn] [--tdce] [--dom] [--liveness] --cfg```

