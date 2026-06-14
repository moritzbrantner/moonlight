import type { Meta, StoryObj } from "@storybook/react-vite";
import { cliBenchmarkComparisonFixture, skippedCliComparisonFixture } from "../test/fixtures";
import { CliBenchmarkRow } from "./CliBenchmarkRow";

const meta = {
  title: "Components/CliBenchmarkRow",
  component: CliBenchmarkRow,
  decorators: [
    (Story) => (
      <table>
        <tbody>
          <Story />
        </tbody>
      </table>
    )
  ]
} satisfies Meta<typeof CliBenchmarkRow>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Ok: Story = {
  args: {
    name: "moonlight",
    comparison: cliBenchmarkComparisonFixture
  }
};

export const Skipped: Story = {
  args: {
    name: "bats",
    comparison: skippedCliComparisonFixture
  }
};
