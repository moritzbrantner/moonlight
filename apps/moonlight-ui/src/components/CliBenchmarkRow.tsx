import type { CliToolComparison } from "../benchmarkData";
import { divideMs, normalizePositiveCount } from "../utils/benchmark";
import { formatMs } from "../utils/format";

type CliBenchmarkRowProps = {
  name: string;
  comparison: CliToolComparison;
};

export function CliBenchmarkRow({ name, comparison }: CliBenchmarkRowProps) {
  const targetInvocationsPerCase = normalizePositiveCount(
    comparison.target_invocations_per_case ?? (name.startsWith("moonlight") ? 2 : 1),
  );
  const totalTargetInvocations =
    comparison.total_target_invocations ?? comparison.total_cases * targetInvocationsPerCase;
  const targetP50 =
    comparison.target_invocation_latency_ms?.p50 ??
    divideMs(comparison.case_latency_ms.p50, targetInvocationsPerCase);
  const targetP95 =
    comparison.target_invocation_latency_ms?.p95 ??
    divideMs(comparison.case_latency_ms.p95, targetInvocationsPerCase);

  return (
    <tr>
      <td>{name}</td>
      <td>{comparison.status}</td>
      <td>{comparison.total_invocations}</td>
      <td>{comparison.cases_per_invocation}</td>
      <td>{targetInvocationsPerCase}</td>
      <td>{comparison.total_cases}</td>
      <td>{totalTargetInvocations}</td>
      <td>{formatMs(comparison.latency_ms.p50)}</td>
      <td>{formatMs(comparison.latency_ms.p95)}</td>
      <td>{formatMs(comparison.case_latency_ms.p50)}</td>
      <td>{formatMs(comparison.case_latency_ms.p95)}</td>
      <td>{formatMs(targetP50)}</td>
      <td>{formatMs(targetP95)}</td>
      <td>{comparison.reason ?? comparison.version ?? ""}</td>
    </tr>
  );
}
