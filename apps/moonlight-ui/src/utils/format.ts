export function formatMs(value: number | null) {
  return value === null ? "-" : value.toFixed(2);
}

export function formatNumber(value: number, maximumFractionDigits = 2) {
  return new Intl.NumberFormat("en", { maximumFractionDigits }).format(value);
}
