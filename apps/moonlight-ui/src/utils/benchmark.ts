export function divideMs(value: number | null, denominator: number) {
  return value === null || denominator < 1 ? null : value / denominator;
}

export function normalizePositiveCount(value: number) {
  return Number.isFinite(value) && value >= 1 ? value : 1;
}
