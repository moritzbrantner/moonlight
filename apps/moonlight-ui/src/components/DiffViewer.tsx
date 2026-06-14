import type { DiffEntry } from "../types";

type DiffViewerProps = {
  title: string;
  diffs: DiffEntry[];
};

export function DiffViewer({ title, diffs }: DiffViewerProps) {
  return (
    <section className="diff-viewer">
      <div className="section-heading">
        <h3>{title}</h3>
        <span>{diffs.length}</span>
      </div>
      {diffs.length === 0 ? (
        <p className="empty">No entries.</p>
      ) : (
        <div className="diff-table">
          {diffs.map((diff, index) => (
            <div className="diff-row" key={`${diff.path}-${index}`}>
              <span>{diff.kind}</span>
              <strong>{diff.path}</strong>
              <code>P: {diff.primary ?? "-"}</code>
              <code>C: {diff.candidate ?? "-"}</code>
              <code>S: {diff.secondary ?? "-"}</code>
            </div>
          ))}
        </div>
      )}
    </section>
  );
}
