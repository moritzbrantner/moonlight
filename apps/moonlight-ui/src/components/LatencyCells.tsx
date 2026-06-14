import type { PercentileSummary } from "../benchmarkData";
import { formatMs } from "../utils/format";

type LatencyCellsProps = {
  latency: PercentileSummary;
};

export function LatencyCells({ latency }: LatencyCellsProps) {
  return (
    <>
      <td>{formatMs(latency.p50)}</td>
      <td>{formatMs(latency.p95)}</td>
      <td>{formatMs(latency.p99)}</td>
      <td>{formatMs(latency.mean)}</td>
      <td>{formatMs(latency.max)}</td>
    </>
  );
}
