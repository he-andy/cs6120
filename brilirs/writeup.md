# Summary 
I, @emw236, @emwangs worked together on this assignment. We chose to use `brilirs` for this assignment due to our lack of familiarity with `typescript`. The repo is linked [here]()

# Implementation \& Testing

We implemented a simple reference counter by adding an extra map to the `Heap` struct in brilirs that maintains the count of the live pointers to any base address in memory. The counter is incremented only through the `call` and `id` `ValueOps` and is decremented either through a free of a pointer previously pointing to it's base address, a reassignment of a pointer to a different base address, or the end of the scope of a pointer. When the `rc` hits 0, the memory is freed and recursive calls are made to free any memory that the freed memory was pointing to. 

We added a `--gc` to the `brilirs` command line interface to indicate whether garbage collection should be performed. If the `--gc` flag was enabled, the `free` instruction is automatically ignored to prevent accidental double frees. Brilirs actually indicates whether there were memory leaks in the program, so no additional implementation was neeeded to track memory leaks. We used the `benchmark/mem` directory to test our implementation with `brench` to test the correctness of our implementation. 

brench.toml:
```brench.toml
extract = 'total_dyn_inst: (\d+)'
benchmarks = 'benchmarks/mem/*.bril'

[runs.baseline]
pipeline = [
    "bril2json",
    "brili -p {args}",
]


[runs.gc]
pipeline = [
    "bril2json",
    "brilirs --gc -p {args}",
]
```

### Results
```
benchmark,run,result
sieve,baseline,3482
sieve,gc,3482
bubblesort,baseline,253
bubblesort,gc,253
primitive-root,baseline,11029
primitive-root,gc,11029
adler32,baseline,6851
adler32,gc,6851
adj2csr,baseline,56629
adj2csr,gc,56629
max-subarray,baseline,193
max-subarray,gc,193
mat-mul,baseline,1990407
mat-mul,gc,1990407
fib,baseline,121
fib,gc,121
vsmul,baseline,86036
vsmul,gc,86036
quicksort,baseline,264
quicksort,gc,264
two-sum,baseline,98
two-sum,gc,98
eight-queens,baseline,1006454
eight-queens,gc,1006454
binary-search,baseline,78
binary-search,gc,78
```

# Challenges 
The biggest challenge we faced was understanding the brilirs codebase. We had to read through the codebase to understand how the brilirs compiler worked and how we could implement our reference counter. 




