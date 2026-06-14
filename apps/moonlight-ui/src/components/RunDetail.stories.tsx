import type { Meta, StoryObj } from "@storybook/react-vite";
import { runFixture, runListFixture } from "../test/fixtures";
import { RunDetail } from "./RunDetail";

const meta = {
  title: "Components/RunDetail",
  component: RunDetail
} satisfies Meta<typeof RunDetail>;

export default meta;
type Story = StoryObj<typeof meta>;

export const SelectedRun: Story = {
  args: {
    run: runFixture,
    fallback: runListFixture[0]
  }
};

export const FallbackOnly: Story = {
  args: {
    run: null,
    fallback: runListFixture[0]
  }
};

export const Empty: Story = {
  args: {
    run: null,
    fallback: null
  }
};
