#! /bin/bash
cargo b --release
echo 'starting benchmark'
brench brench.toml > results.csv
python3 ./summarize.py results.csv