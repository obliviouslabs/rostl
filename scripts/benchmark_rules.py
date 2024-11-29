import argparse
from dataclasses import dataclass
import re
from typing import Dict

arg_parser = argparse.ArgumentParser()
arg_parser.add_argument('--input', type=str, default="target/bencher.log")
arg_parser.add_argument('--rules', type=str, default="scripts/benchmark_rules.txt")
args = arg_parser.parse_args()

@dataclass
class Measurement:
  name: str
  value: float
  unit: str
  stddev: float

def parse_input(file_path: str) -> Dict[str, Measurement]:
  result = {}
  with open(file_path, "r") as file:
    regex = r"^test (.+)\s+\.\.\. bench:\s+([0-9,.]+) (\w+\/\w+) \(\+\/- ([0-9,.]+)\)$"
    for line in file:
      m = re.match(regex, line)
      if m:
        name = m.group(1)
        value = int(m.group(2))
        unit = m.group(3)
        stddev = int(m.group(4))
        assert name not in result
        result[name] = Measurement(name, value, unit, stddev)
  return result

def check_rules(file_path: str, measurements: Dict[str, Measurement]):
  with open(file_path, "r") as file:
    for line in file:
      if line.startswith("#"):
        continue
      if line.strip() == "":
        continue
      parts = line.split()
      name1 = parts[0]
      name2 = parts[1]
      condition = " ".join(parts[2:])
      condition = condition
      assert name1 in measurements, f"Measurement {name1} not found"
      assert name2 in measurements, f"Measurement {name2} not found"
      value1 = measurements[name1].value
      value2 = measurements[name2].value
      expr = eval(f"(lambda x=float({value1}), y=float({value2}): {condition})()")
      assert expr, f"Rule violated for ({name1}, {name2}) = ({value1}, {value2}): {condition}"
  
  
measurements = parse_input(args.input)
rules = check_rules(args.rules, measurements)
print("Benchmark rules are ok!")