import type { Meta, StoryObj } from "@storybook/react-vite";
import { BenchmarkSection } from "./BenchmarkSection";

const meta = {
  title: "Components/BenchmarkSection",
  component: BenchmarkSection
} satisfies Meta<typeof BenchmarkSection>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {
    title: "HTTP Benchmark",
    generatedAt: "2026-06-13T12:00:00.000Z",
    details: "600 requests, concurrency 8, 4 endpoints",
    children: <p>Benchmark content</p>
  }
};
