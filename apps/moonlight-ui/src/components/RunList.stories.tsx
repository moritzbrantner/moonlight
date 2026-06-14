import type { Meta, StoryObj } from "@storybook/react-vite";
import { runFixture, runListFixture } from "../test/fixtures";
import { RunList } from "./RunList";

const meta = {
  title: "Components/RunList",
  component: RunList
} satisfies Meta<typeof RunList>;

export default meta;
type Story = StoryObj<typeof meta>;

export const WithRuns: Story = {
  args: {
    runs: runListFixture,
    selectedId: runFixture.id,
    onSelect: () => undefined
  }
};

export const Empty: Story = {
  args: {
    runs: [],
    selectedId: null,
    onSelect: () => undefined
  }
};
