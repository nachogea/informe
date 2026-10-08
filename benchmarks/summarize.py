#!/usr/bin/env python3
"""Summarize measured JSONL trials. No dependencies, model calls, or estimates."""
import argparse
from collections import defaultdict
import json
import math
from pathlib import Path
import statistics

GROUP = ("task", "format", "model", "environment", "scenario")
COUNTS = ("input_tokens", "cached_input_tokens", "output_tokens", "reasoning_tokens", "repair_count", "output_bytes")
METRICS = COUNTS + ("first_token_ms", "generation_ms", "open_ms", "reload_ms", "request_to_ready_ms")


def read_trials(path):
    records = []
    for number, line in enumerate(path.read_text().splitlines(), 1):
        if not line.strip():
            continue
        try:
            record = json.loads(line)
            if not isinstance(record, dict):
                raise ValueError("record must be an object")
            for key in GROUP:
                if not isinstance(record.get(key), str) or not record[key].strip():
                    raise ValueError(f"{key} must be a nonempty string")
            if record["format"] not in ("native_json", "native_markdown", "html_standalone", "html_shared"):
                raise ValueError("unknown artifact format")
            if not isinstance(record.get("success"), bool):
                raise ValueError("success must be a boolean")
            for key in METRICS:
                value = record.get(key)
                if value is None:
                    continue
                if isinstance(value, bool) or not isinstance(value, (int, float)) or not math.isfinite(value) or value < 0:
                    raise ValueError(f"{key} must be finite and nonnegative")
                if key in COUNTS and not isinstance(value, int):
                    raise ValueError(f"{key} must be an integer")
            if record.get("cached_input_tokens") is not None and record.get("input_tokens") is not None:
                if record["cached_input_tokens"] > record["input_tokens"]:
                    raise ValueError("cached input exceeds total input")
            records.append(record)
        except (ValueError, TypeError) as error:
            raise ValueError(f"{path}:{number}: {error}") from error
    if not records:
        raise ValueError(f"{path}: no trial records")
    return records


def percentile(values, fraction):
    values = sorted(values)
    index = (len(values) - 1) * fraction
    lower = math.floor(index)
    upper = math.ceil(index)
    return values[lower] + (values[upper] - values[lower]) * (index - lower)


def summarize(records):
    groups = defaultdict(list)
    for record in records:
        groups[tuple(record[key] for key in GROUP)].append(record)
    result = []
    for group, trials in sorted(groups.items()):
        successful = [trial for trial in trials if trial["success"]]
        report = dict(zip(GROUP, group))
        report.update(trials=len(trials), successes=len(successful), success_rate=len(successful) / len(trials))
        report["usage_all_trials"] = {}
        report["metrics_successful_trials"] = {}
        for metric in METRICS:
            if metric.endswith("_tokens"):
                all_values = [trial[metric] for trial in trials if trial.get(metric) is not None]
                if all_values:
                    report["usage_all_trials"][metric] = {"measured_trials": len(all_values), "total": sum(all_values)}
            values = [trial[metric] for trial in successful if trial.get(metric) is not None]
            if values:
                report["metrics_successful_trials"][metric] = {
                    "measured_trials": len(values), "median": statistics.median(values), "p95": percentile(values, 0.95)
                }
        result.append(report)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("records", type=Path)
    args = parser.parse_args()
    try:
        records = read_trials(args.records)
    except (OSError, ValueError) as error:
        parser.exit(1, f"{error}\n")
    print(json.dumps(summarize(records), indent=2, allow_nan=False))


if __name__ == "__main__":
    main()
