import csv
import sys

data = []
with open(sys.argv[1], 'r', newline='') as f:
    reader = csv.reader(f)
    data = list(reader)[1:]

class Entry:
    def __init__(self, name): 
        self.name = name 
        self.tests = {}
    
    def add_test(self, test, result):
        self.tests[test] = result

    def compute_score(self, test):
        baseline = self.tests['baseline']
        return  baseline / self.tests[test] 


entries = {} 
for entry in data:
    name = entry[0]
    test = entry[1]
    if entry[2].isdigit():
        result = float(entry[2])
    else:
        print(f'Warning: results for {entry[0]} {entry[1]} ({entry[2]}) will be ignored.')
        continue
    
    if name in entries:
        entries[name].add_test(test, result)
    else:
        entries[name] = Entry(name)
        entries[name].add_test(test, result)

# compute mean, stddev, min, max of a list of numbers
def compute_stats(l):
    mean = sum(l) / len(l)
    stddev = (sum([(x - mean)**2 for x in l]) / len(l))**0.5
    return mean, stddev, min(l), max(l)
    

ivs = sorted(list(set(data[1] for data in data) - {'baseline'}))
for iv in ivs:
    scores = []
    for entry in entries.values():
        if iv in entry.tests:
            scores.append(entry.compute_score(iv))
    mean, stddev, xmin, xmax = compute_stats(scores)
    print(f'{iv}: mean = {mean}, stddev = {stddev}, min = {xmin}, max = {xmax}')
    