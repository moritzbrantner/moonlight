import type { Meta, StoryObj } from "@storybook/react-vite";
import { httpBenchmarkTargetFixture } from "../test/fixtures";
import { LatencyCells } from "./LatencyCells";

const meta = {
  title: "Components/LatencyCells",
  component: LatencyCells,
  decorators: [
    (Story) => (
      <table>
        <tbody>
          <tr>
            <Story />
          </tr>
        </tbody>
      </table>
    )
  ]
} satisfies Meta<typeof LatencyCells>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {
    latency: httpBenchmarkTargetFixture.latency_ms
  }
};
