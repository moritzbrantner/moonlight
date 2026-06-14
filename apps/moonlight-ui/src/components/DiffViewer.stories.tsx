import type { Meta, StoryObj } from "@storybook/react-vite";
import { runFixture } from "../test/fixtures";
import { DiffViewer } from "./DiffViewer";

const meta = {
  title: "Components/DiffViewer",
  component: DiffViewer
} satisfies Meta<typeof DiffViewer>;

export default meta;
type Story = StoryObj<typeof meta>;

export const WithDiffs: Story = {
  args: {
    title: "Noise-filtered diff",
    diffs: runFixture.comparison.noise_filtered_diffs
  }
};

export const Empty: Story = {
  args: {
    title: "Reference noise",
    diffs: []
  }
};
