import type { ReactNode } from "react";

type BenchmarkSectionProps = {
  title: string;
  generatedAt: string;
  details: string;
  children: ReactNode;
};

export function BenchmarkSection({ title, generatedAt, details, children }: BenchmarkSectionProps) {
  return (
    <section className="benchmark-section">
      <div className="benchmark-heading">
        <div>
          <h3>{title}</h3>
          <p>{details}</p>
        </div>
        <time dateTime={generatedAt}>{new Date(generatedAt).toLocaleString()}</time>
      </div>
      {children}
    </section>
  );
}
