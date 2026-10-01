#!/usr/bin/env python3
"""Summarize production-only LCOV line coverage (excluding inline test modules).

Usage: python3 scripts/production_coverage.py coverage.lcov
Run against the matching checkout: LCOV SF records must point to that source tree.
"""
import pathlib
import re
import sys

covered = total = 0
for record in pathlib.Path(sys.argv[1]).read_text().split('end_of_record'):
    source = re.search(r'^SF:(.+)$', record, re.MULTILINE)
    if not source:
        continue
    path = pathlib.Path(source.group(1))
    if path.parent.name != 'src':
        continue
    lines = path.read_text().splitlines()
    limit = next((i for i, line in enumerate(lines, 1)
                  if line.strip() == '#[cfg(test)]'), len(lines) + 1)
    counts = {int(line): int(count) for line, count in
              re.findall(r'^DA:(\d+),(\d+)', record, re.MULTILINE)
              if int(line) < limit}
    hit = sum(count > 0 for count in counts.values())
    covered += hit
    total += len(counts)
    print(f'{path.name}: {hit}/{len(counts)} production lines')
print(f'TOTAL: {covered}/{total} production lines ({covered / total:.2%})')
