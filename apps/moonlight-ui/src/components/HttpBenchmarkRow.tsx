import type { HttpBenchmarkTarget } from "../benchmarkData";
import { formatNumber } from "../utils/format";
import { LatencyCells } from "./LatencyCells";

type HttpBenchmarkRowProps = {
  target: HttpBenchmarkTarget;
};

export function HttpBenchmarkRow({ target }: HttpBenchmarkRowProps) {
  return (
    <tr>
      <td>{target.name}</td>
      <td>{target.total_requests}</td>
      <td>{target.success_count}</td>
      <td>{target.error_count}</td>
      <td>{formatNumber(target.requests_per_second)}</td>
      <LatencyCells latency={target.latency_ms} />
    </tr>
  );
}
