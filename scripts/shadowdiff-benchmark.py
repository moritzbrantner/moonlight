#!/usr/bin/env python3
import argparse
import concurrent.futures
import datetime as dt
import json
import os
import statistics
import time
import urllib.error
import urllib.request
from pathlib import Path


DEFAULT_ENDPOINTS = [
    "/success",
    "/regression",
    "/noise",
    "/noisy-regression",
    "/status-regression",
    "/slow-candidate",
]

NORMALIZED_KEYS = {"timestamp", "requestId", "traceId", "id"}


def parse_args():
    parser = argparse.ArgumentParser(
        description="Benchmark moonlight against Diffy B/C and populate Diffy A validation."
    )
    parser.add_argument("--warmup", type=int, default=int(os.getenv("BENCHMARK_WARMUP", "50")))
    parser.add_argument(
        "--requests", type=int, default=int(os.getenv("BENCHMARK_REQUESTS", "600"))
    )
    parser.add_argument(
        "--concurrency", type=int, default=int(os.getenv("BENCHMARK_CONCURRENCY", "8"))
    )
    parser.add_argument(
        "--timeout", type=float, default=float(os.getenv("BENCHMARK_TIMEOUT", "10"))
    )
    parser.add_argument(
        "--validation-requests",
        type=int,
        default=int(os.getenv("BENCHMARK_VALIDATION_REQUESTS", "60")),
    )
    parser.add_argument(
        "--output-dir",
        default=os.getenv("BENCHMARK_OUTPUT_DIR", "data/shadowdiff/benchmark"),
    )
    parser.add_argument(
        "--moonlight-url",
        default=os.getenv("SHADOWDIFF_URL", "http://127.0.0.1:8080"),
    )
    parser.add_argument(
        "--diffy-b-url",
        default=os.getenv("DIFFY_B_URL", "http://127.0.0.1:8890"),
    )
    parser.add_argument(
        "--diffy-c-url",
        default=os.getenv("DIFFY_C_URL", "http://127.0.0.1:8900"),
    )
    parser.add_argument(
        "--diffy-a-url",
        default=os.getenv("DIFFY_A_URL", "http://127.0.0.1:8880"),
    )
    parser.add_argument(
        "--endpoint",
        dest="endpoints",
        action="append",
        help="Endpoint to include; may be specified multiple times.",
    )
    return parser.parse_args()


def request_url(base_url, path, timeout):
    started = time.perf_counter()
    url = f"{base_url.rstrip('/')}{path}"
    try:
        with urllib.request.urlopen(url, timeout=timeout) as response:
            body = response.read()
            status = response.status
            headers = dict(response.headers.items())
            error = None
    except urllib.error.HTTPError as exc:
        body = exc.read()
        status = exc.code
        headers = dict(exc.headers.items())
        error = None
    except Exception as exc:
        body = b""
        status = None
        headers = {}
        error = str(exc)

    elapsed_ms = (time.perf_counter() - started) * 1000
    return {
        "url": url,
        "path": path,
        "status": status,
        "headers": headers,
        "body": body.decode("utf-8", errors="replace"),
        "latency_ms": elapsed_ms,
        "error": error,
    }


def wait_for(name, base_url, path, timeout, deadline_seconds=120):
    deadline = time.monotonic() + deadline_seconds
    last_error = None
    while time.monotonic() < deadline:
        result = request_url(base_url, path, timeout)
        if result["error"] is None and result["status"] is not None:
            return
        last_error = result["error"] or f"status {result['status']}"
        time.sleep(1)

    raise RuntimeError(f"{name} was not ready at {base_url}{path}: {last_error}")


def run_requests(name, base_url, endpoints, total, concurrency, timeout):
    paths = [endpoints[index % len(endpoints)] for index in range(total)]
    started = time.perf_counter()
    results = []

    with concurrent.futures.ThreadPoolExecutor(max_workers=concurrency) as executor:
        futures = [executor.submit(request_url, base_url, path, timeout) for path in paths]
        for future in concurrent.futures.as_completed(futures):
            results.append(future.result())

    elapsed_seconds = time.perf_counter() - started
    return summarize_results(name, base_url, results, elapsed_seconds)


def percentile(values, percentile_value):
    if not values:
        return None
    ordered = sorted(values)
    index = (len(ordered) - 1) * (percentile_value / 100)
    lower = int(index)
    upper = min(lower + 1, len(ordered) - 1)
    if lower == upper:
        return ordered[lower]
    weight = index - lower
    return ordered[lower] * (1 - weight) + ordered[upper] * weight


def summarize_results(name, base_url, results, elapsed_seconds):
    latencies = [result["latency_ms"] for result in results if result["error"] is None]
    status_counts = {}
    for result in results:
        key = "error" if result["status"] is None else str(result["status"])
        status_counts[key] = status_counts.get(key, 0) + 1

    success_count = sum(
        1
        for result in results
        if result["error"] is None
        and result["status"] is not None
        and 200 <= result["status"] < 400
    )
    error_count = len(results) - success_count

    return {
        "name": name,
        "base_url": base_url,
        "total_requests": len(results),
        "success_count": success_count,
        "error_count": error_count,
        "status_counts": status_counts,
        "requests_per_second": len(results) / elapsed_seconds if elapsed_seconds else 0,
        "latency_ms": {
            "min": min(latencies) if latencies else None,
            "mean": statistics.fmean(latencies) if latencies else None,
            "p50": percentile(latencies, 50),
            "p90": percentile(latencies, 90),
            "p95": percentile(latencies, 95),
            "p99": percentile(latencies, 99),
            "max": max(latencies) if latencies else None,
        },
    }


def normalize_json(value):
    if isinstance(value, dict):
        return {
            key: normalize_json(item)
            for key, item in sorted(value.items())
            if key not in NORMALIZED_KEYS
        }
    if isinstance(value, list):
        return [normalize_json(item) for item in value]
    return value


def normalized_body(text):
    try:
        return normalize_json(json.loads(text))
    except json.JSONDecodeError:
        return text


def compare_validity(targets, endpoints, timeout):
    comparisons = []
    for endpoint in endpoints:
        responses = {
            name: request_url(base_url, endpoint, timeout)
            for name, base_url in targets.items()
        }
        normalized = {
            name: {
                "status": response["status"],
                "body": normalized_body(response["body"]),
                "error": response["error"],
            }
            for name, response in responses.items()
        }
        baseline = normalized["moonlight"]
        mismatches = [
            name
            for name, value in normalized.items()
            if value != baseline
        ]
        comparisons.append(
            {
                "endpoint": endpoint,
                "match": not mismatches,
                "mismatches": mismatches,
                "responses": normalized,
            }
        )
    return comparisons


def fetch_json(base_url, path, timeout):
    result = request_url(base_url, path, timeout)
    if result["error"] is not None or result["status"] is None or result["status"] >= 400:
        return {
            "error": result["error"] or f"status {result['status']}",
            "body": result["body"],
        }
    try:
        return json.loads(result["body"])
    except json.JSONDecodeError as exc:
        return {"error": f"invalid json: {exc}", "body": result["body"]}


def format_ms(value):
    return "-" if value is None else f"{value:.2f}"


def write_markdown(report, path):
    lines = [
        "# Shadowdiff Benchmark",
        "",
        f"Generated: `{report['generated_at']}`",
        "",
        "## Latency",
        "",
        "| Target | Requests | Success | Errors | Req/s | p50 ms | p95 ms | p99 ms | Mean ms | Max ms |",
        "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
    ]

    for target in report["targets"].values():
        latency = target["latency_ms"]
        lines.append(
            "| {name} | {total} | {success} | {errors} | {rps:.2f} | {p50} | {p95} | {p99} | {mean} | {max} |".format(
                name=target["name"],
                total=target["total_requests"],
                success=target["success_count"],
                errors=target["error_count"],
                rps=target["requests_per_second"],
                p50=format_ms(latency["p50"]),
                p95=format_ms(latency["p95"]),
                p99=format_ms(latency["p99"]),
                mean=format_ms(latency["mean"]),
                max=format_ms(latency["max"]),
            )
        )

    lines.extend(
        [
            "",
            "## Status Codes",
            "",
            "| Target | Status counts |",
            "| --- | --- |",
        ]
    )
    for target in report["targets"].values():
        lines.append(f"| {target['name']} | `{json.dumps(target['status_counts'], sort_keys=True)}` |")

    lines.extend(
        [
            "",
            "## Direct Validity",
            "",
            "| Endpoint | Result | Mismatches |",
            "| --- | --- | --- |",
        ]
    )
    for comparison in report["validity"]:
        result = "match" if comparison["match"] else "mismatch"
        mismatches = ", ".join(comparison["mismatches"]) if comparison["mismatches"] else "-"
        lines.append(f"| `{comparison['endpoint']}` | {result} | {mismatches} |")

    lines.extend(
        [
            "",
            "## Moonlight Stats",
            "",
            "```json",
            json.dumps(report["moonlight_stats"], indent=2, sort_keys=True),
            "```",
            "",
        ]
    )

    path.write_text("\n".join(lines), encoding="utf-8")


def main():
    args = parse_args()
    endpoints = args.endpoints or DEFAULT_ENDPOINTS
    output_dir = Path(args.output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)

    benchmark_targets = {
        "moonlight": args.moonlight_url,
        "diffy_b": args.diffy_b_url,
        "diffy_c": args.diffy_c_url,
    }

    wait_for("moonlight health", args.moonlight_url, "/api/health", args.timeout)
    wait_for("moonlight proxy", args.moonlight_url, "/success", args.timeout)
    wait_for("Diffy A", args.diffy_a_url, "/success", args.timeout)
    wait_for("Diffy B", args.diffy_b_url, "/success", args.timeout)
    wait_for("Diffy C", args.diffy_c_url, "/success", args.timeout)

    for name, base_url in benchmark_targets.items():
        print(f"warming {name} ({args.warmup} requests)")
        run_requests(name, base_url, endpoints, args.warmup, args.concurrency, args.timeout)

    summaries = {}
    for name, base_url in benchmark_targets.items():
        print(f"measuring {name} ({args.requests} requests)")
        summaries[name] = run_requests(
            name,
            base_url,
            endpoints,
            args.requests,
            args.concurrency,
            args.timeout,
        )

    print(f"populating Diffy A validation ({args.validation_requests} requests)")
    run_requests(
        "diffy_a_validation",
        args.diffy_a_url,
        endpoints,
        args.validation_requests,
        min(args.concurrency, 4),
        args.timeout,
    )

    validity = compare_validity(benchmark_targets, endpoints, args.timeout)
    moonlight_stats = fetch_json(args.moonlight_url, "/api/stats", args.timeout)

    report = {
        "generated_at": dt.datetime.now(dt.timezone.utc).isoformat(),
        "config": {
            "warmup": args.warmup,
            "requests": args.requests,
            "concurrency": args.concurrency,
            "timeout": args.timeout,
            "validation_requests": args.validation_requests,
            "endpoints": endpoints,
        },
        "targets": summaries,
        "validity": validity,
        "moonlight_stats": moonlight_stats,
    }

    json_path = output_dir / "latest.json"
    markdown_path = output_dir / "latest.md"
    json_path.write_text(json.dumps(report, indent=2, sort_keys=True), encoding="utf-8")
    write_markdown(report, markdown_path)

    print(f"wrote {json_path}")
    print(f"wrote {markdown_path}")


if __name__ == "__main__":
    main()
