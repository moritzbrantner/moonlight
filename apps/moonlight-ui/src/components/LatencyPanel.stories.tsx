import type { Meta, StoryObj } from "@storybook/react-vite";
import { runFixture } from "../test/fixtures";
import { LatencyPanel } from "./LatencyPanel";

const meta = {
  title: "Components/LatencyPanel",
  component: LatencyPanel
} satisfies Meta<typeof LatencyPanel>;

export default meta;
type Story = StoryObj<typeof meta>;

export const WithRun: Story = {
  args: {
    run: runFixture
  }
};

export const Empty: Story = {
  args: {
    run: null
  }
};
