import type { Meta, StoryObj } from "@storybook/react-vite";
import { httpBenchmarkTargetFixture } from "../test/fixtures";
import { HttpBenchmarkRow } from "./HttpBenchmarkRow";

const meta = {
  title: "Components/HttpBenchmarkRow",
  component: HttpBenchmarkRow,
  decorators: [
    (Story) => (
      <table>
        <tbody>
          <Story />
        </tbody>
      </table>
    )
  ]
} satisfies Meta<typeof HttpBenchmarkRow>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {
    target: httpBenchmarkTargetFixture
  }
};
